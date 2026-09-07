#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Stage 2 of the ONLINE (voting portal) load test: takes the outputs of
Stage 1 (setup_telephone_load_test.py with setup.voting_channel: ONLINE — a
summary.json + voters CSV), renders a voter manifest, then drives one
Playwright run in packages/voting-portal where each voter is a real
headless-browser session going through login, election selection, candidate
selection, review, cast and confirmation — so all portal overhead (Keycloak
auth, GraphQL, ballot styles, WASM ballot encryption) is exercised for real.
Concurrency is delegated to Playwright workers (one process, one reused
browser per worker) rather than fanning out separate processes, because
browsers are expensive. See
docs/docusaurus/docs/07-developers/02-cli/02-tutorials/load-testing/online-load-testing-design.md.

Takes no command-line arguments — every setting lives in
telephone-load-test-inputs/config/layers.yaml, under 'online_run:'.

Requires @playwright/test and the Playwright Chromium browser — either the
standalone install under packages/voting-portal/test/load (what
install_load_client.sh sets up: `npm install` there, then `npx playwright
install --with-deps chromium`) or the full Yarn workspace (`yarn` from
packages/, then `yarn --cwd packages/voting-portal playwright install
chromium`). $PLAYWRIGHT_BIN overrides the binary lookup altogether.

For a distributed run, set online_run.start_at (or $LOAD_TEST_START_AT) to
the same UTC timestamp on every load client: each one preflights and renders
its manifest immediately, then holds until that instant before launching any
browser, so the clients hit the portal together. Give each client its own
online_run.start_delay (or $LOAD_TEST_START_DELAY, seconds on top of
start_at) to ramp the load up in steps instead. Merge the per-client outputs
afterwards with aggregate_online_load_test.py.
"""

from __future__ import annotations

import csv
import json
import os
import socket
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from shutil import which
from typing import Any

import load_test_common as common

VOTING_PORTAL_DIR = common.REPO_ROOT / "packages" / "voting-portal"
LOAD_TEST_DIR = VOTING_PORTAL_DIR / "test" / "load"
PLAYWRIGHT_CONFIG = "playwright.config.ts"
# Searched in order when $PLAYWRIGHT_BIN is unset: the standalone test/load
# install first (a load client machine typically has only that), then the
# Yarn workspace's hoisted/per-package binaries (the devcontainer).
PLAYWRIGHT_BIN_CANDIDATES = (
    LOAD_TEST_DIR / "node_modules" / ".bin" / "playwright",
    VOTING_PORTAL_DIR / "node_modules" / ".bin" / "playwright",
    common.REPO_ROOT / "packages" / "node_modules" / ".bin" / "playwright",
)
PLAYWRIGHT_INSTALL_HINT = (
    f"(cd {LOAD_TEST_DIR} && npm install && npx playwright install --with-deps chromium) "
    "— or run packages/step-cli/scripts/install_load_client.sh"
)


def dig(d: Any, *keys: Any, default: Any = None) -> Any:
    for key in keys:
        if not isinstance(d, (dict, list)):
            return default
        try:
            d = d[key]
        except (KeyError, IndexError, TypeError):
            return default
    return default if d is None else d


def iter_report_specs(report: dict[str, Any]):
    def walk(suite: dict[str, Any]):
        yield suite
        for child in suite.get("suites") or []:
            yield from walk(child)

    for top_suite in report.get("suites") or []:
        for suite in walk(top_suite):
            yield from suite.get("specs") or []


def main() -> None:
    config = common.load_config()
    cfg = common.section(config, "online_run")

    run_dir = common.resolve_path(common.req_str(cfg, "run_dir"))
    tenants_json = run_dir / "tenants.json"
    if not tenants_json.is_file():
        common.die(f"no tenants.json in {run_dir} — run setup_telephone_load_test.py with setup.voting_channel: ONLINE first")
    with tenants_json.open() as f:
        tenants_index = json.load(f)
    tenants = tenants_index.get("tenants") or []
    if not tenants:
        common.die(f"{tenants_json} lists no tenants")

    voter_offset = int(cfg.get("voter_offset") or 0)
    if voter_offset < 0:
        common.die("online_run.voter_offset must be a non-negative integer")
    concurrency = int(cfg.get("concurrency") or 4)
    if concurrency < 1:
        common.die("online_run.concurrency must be a positive integer")
    max_votes = cfg.get("max_votes")
    max_votes = int(max_votes) if max_votes else None
    vote_timeout = int(cfg.get("vote_timeout") or 180)
    candidates_pattern = cfg.get("candidates_pattern") or ""
    headed = bool(cfg.get("headed") or False)
    out_dir = common.resolve_path(common.req_str(cfg, "out_dir"))
    start_at, start_delay = common.resolve_start(cfg)

    if headed and concurrency != 1:
        common.log("headed is a debugging mode; forcing concurrency to 1")
        concurrency = 1

    # --- Read every tenant's summary.json ---
    tenant_infos = []
    for tenant in tenants:
        tenant_summary_json = run_dir / tenant["dir"] / "summary.json"
        with tenant_summary_json.open() as f:
            summary = json.load(f)

        tenant_id = summary.get("tenant_id")
        election_event_id = summary.get("election_event_id")
        keycloak_realm = summary.get("keycloak_realm")
        keycloak_url = cfg.get("keycloak_url") or summary.get("keycloak_url")
        hasura_url = (cfg.get("hasura_url") or summary.get("hasura_url") or "").removesuffix("/v1/graphql")
        voters_csv = Path(summary.get("voters_csv", ""))
        voting_channel = summary.get("voting_channel")
        voting_portal_url = cfg.get("voting_portal_url") or summary.get("voting_portal_url")
        for name, value in [
            ("tenant_id", tenant_id), ("election_event_id", election_event_id), ("keycloak_realm", keycloak_realm),
            ("keycloak_url", keycloak_url), ("hasura_url", hasura_url),
        ]:
            if not value:
                common.die(f"could not read {name} from {tenant_summary_json}")
        if voting_channel != "ONLINE":
            common.die(
                f"{tenant_summary_json} was provisioned for the {voting_channel or 'TELEPHONE'} channel — re-run "
                "setup_telephone_load_test.py with setup.voting_channel: ONLINE (the portal's eligibility "
                "check gates on the ONLINE channel being open)"
            )
        if not voting_portal_url:
            common.die(f"could not read voting_portal_url from {tenant_summary_json} (set online_run.voting_portal_url)")
        voting_portal_url = voting_portal_url.rstrip("/")
        login_url = f"{voting_portal_url}/tenant/{tenant_id}/event/{election_event_id}/login"
        if not voters_csv.is_file():
            # Stage 1 writes voters_csv as an absolute path; tolerate a
            # moved/copied run dir (e.g. rsync'ed to another load machine).
            voters_csv = tenant_summary_json.parent / voters_csv.name
        if not voters_csv.is_file():
            common.die(f"voters CSV not found: {summary.get('voters_csv')}")
        common.log(f"Election event {election_event_id} (tenant {tenant_id}), voters: {voters_csv}")
        common.log(f"Login URL: {login_url}")
        tenant_infos.append({
            "tenant_id": tenant_id, "election_event_id": election_event_id, "keycloak_realm": keycloak_realm,
            "keycloak_url": keycloak_url, "hasura_url": hasura_url, "voters_csv": voters_csv,
            "voting_portal_url": voting_portal_url, "login_url": login_url,
        })

    # --- Preflight: every dependency checked before any load is generated ---
    # Every tenant shares the same portal/Keycloak/Hasura deployment (only
    # the realm differs), so it's enough to preflight against the first.
    first = tenant_infos[0]
    voting_portal_url, keycloak_url, keycloak_realm, hasura_url = (
        first["voting_portal_url"], first["keycloak_url"], first["keycloak_realm"], first["hasura_url"],
    )
    if not common.http_ok(f"{voting_portal_url}/"):
        common.die(f"voting portal not reachable at {voting_portal_url} — start it with 'yarn start:voting-portal' (or set online_run.voting_portal_url)")
    if not common.http_ok(f"{keycloak_url}/realms/{keycloak_realm}/.well-known/openid-configuration"):
        if common.http_ok(f"{keycloak_url}/realms/master/.well-known/openid-configuration"):
            common.die(f"Keycloak is up at {keycloak_url} but realm {keycloak_realm} does not exist — was the election event deleted, or is {run_dir} stale?")
        else:
            common.die(f"Keycloak not reachable at {keycloak_url} — is the stack running? (set online_run.keycloak_url to override summary.json's URL on another machine)")
    if not common.http_ok(f"{hasura_url}/healthz"):
        common.die(f"Hasura not reachable at {hasura_url} — is the stack running? (set online_run.hasura_url to override summary.json's URL on another machine)")

    playwright_bin = None
    configured_bin = os.environ.get("PLAYWRIGHT_BIN")
    if configured_bin:
        playwright_bin = Path(configured_bin)
        if not (playwright_bin.is_file() and os.access(playwright_bin, os.X_OK)):
            common.die(f"$PLAYWRIGHT_BIN={configured_bin!r} is not an executable file")
    else:
        for candidate in PLAYWRIGHT_BIN_CANDIDATES:
            if candidate.is_file() and os.access(candidate, os.X_OK):
                playwright_bin = candidate
                break
    if playwright_bin is None:
        common.die(f"Playwright not installed. Install it: {PLAYWRIGHT_INSTALL_HINT}")

    env = dict(os.environ)
    if os.environ.get("IN_NIX_SHELL") and not os.environ.get("PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS"):
        # Playwright's own ldd-based dependency check runs against devenv's
        # nix glibc, which doesn't see this system's actual runtime
        # libraries — a false positive that blocks every launch even though
        # Chromium runs fine. Skip it; the chromium-install check just below
        # still catches a genuinely missing browser.
        env["PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS"] = "1"

    # Best-effort browser check against Playwright's default cache location;
    # a custom PLAYWRIGHT_BROWSERS_PATH is honored, and a false negative
    # still fails later with Playwright's own (equally actionable) error.
    browsers_path = Path(os.environ.get("PLAYWRIGHT_BROWSERS_PATH") or (Path.home() / ".cache" / "ms-playwright"))
    if not list(browsers_path.glob("chromium*")):
        common.die(f"no Playwright Chromium found under {browsers_path} — install it: {PLAYWRIGHT_INSTALL_HINT}")
    common.log(f"Using Playwright: {playwright_bin}")

    cores = os.cpu_count()
    if cores and concurrency > cores - 2:
        common.log(
            f"WARNING: concurrency {concurrency} exceeds {max(cores - 2, 1)} (cores - 2 on this machine); "
            "ballot encryption is CPU-bound and the compose stack shares these cores — expect queueing to skew latency numbers"
        )

    # --- Local port bridging (devcontainer-only) ---
    #
    # The voting portal's login flow sends the BROWSER to Keycloak/MinIO URLs
    # baked in for the developer's own OS browser (reachable via VS Code's
    # forwardPorts, e.g. http://localhost:8090) — a headless browser launched
    # from *inside* the devcontainer can't reach those the same way, since
    # "localhost" there is the devcontainer's own loopback, not the host's.
    # Bridge the ports this flow is known to need with socat for the run,
    # skipping any that already resolve (e.g. a distributed run, or a
    # devcontainer with host networking where this is unnecessary). Content
    # the browser may also fetch straight from MinIO (e.g. candidate photos)
    # is not covered here.
    forward_procs: list[subprocess.Popen] = []

    def maybe_forward_local_port(port: int, target: str) -> None:
        if common.http_ok(f"http://127.0.0.1:{port}/", timeout=2):
            return
        if not which("socat"):
            common.log(f"WARNING: 127.0.0.1:{port} is not reachable from inside this container, and socat is not installed to bridge it to {target} — the browser may fail to reach it")
            return
        common.log(f"Bridging 127.0.0.1:{port} -> {target} for the browser (socat)")
        forward_procs.append(subprocess.Popen(["socat", f"TCP-LISTEN:{port},fork,reuseaddr", f"TCP:{target}"]))

    if voting_portal_url.startswith("http://localhost:") or voting_portal_url.startswith("http://127.0.0.1:"):
        keycloak_hostport = keycloak_url.removeprefix("http://")
        hasura_hostport = hasura_url.removeprefix("http://")
        maybe_forward_local_port(int(keycloak_hostport.rsplit(":", 1)[1]), keycloak_hostport)
        maybe_forward_local_port(int(hasura_hostport.rsplit(":", 1)[1]), hasura_hostport)
        maybe_forward_local_port(9002, "minio-proxy:9002")

    try:
        common.wait_for_start(start_at, start_delay)
        tenant_summaries = []
        for i, info in enumerate(tenant_infos):
            common.log(f"=== Tenant {i + 1}/{len(tenant_infos)}: {info['tenant_id']} ===")
            tenant_out_dir = out_dir / f"tenant-{info['tenant_id']}"
            tenant_summaries.append(_run(
                cfg, tenant_out_dir, info["login_url"], info["voters_csv"], voter_offset, max_votes,
                concurrency, vote_timeout, candidates_pattern, headed, playwright_bin, env,
                info["election_event_id"], info["tenant_id"],
            ))
    finally:
        for proc in forward_procs:
            proc.terminate()
        for proc in forward_procs:
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()

    total_votes = sum(s["total_votes"] for s in tenant_summaries)
    total_cast = sum(s["cast"] for s in tenant_summaries)
    total_failed = sum(s["failed"] for s in tenant_summaries)
    throughput = common.run_throughput(tenant_summaries, total_cast)
    summary_out_path = out_dir / "summary.json"
    common.write_json(summary_out_path, {
        # Identifies this load client's slice in aggregate_online_load_test.py
        # once several clients' out_dirs are collected side by side.
        "client": socket.gethostname(),
        "start_at": common.format_timestamp(start_at) if start_at else None,
        "start_delay_secs": start_delay,
        "voter_offset": voter_offset,
        "max_votes": max_votes,
        "concurrency": concurrency,
        "run_dir": str(run_dir),
        "tenants": tenant_summaries,
        "total_votes": total_votes,
        "cast": total_cast,
        "failed": total_failed,
        **throughput,
    })
    common.log(f"Done: {total_cast}/{total_votes} voters cast a ballot across {len(tenant_infos)} tenant(s) ({total_failed} did not)")
    common.log(f"Throughput: {throughput['cast_per_second']} ballots cast/second over {throughput['elapsed_secs']}s (all tenants combined)")
    common.log(f"Run summary: {summary_out_path}")
    if total_failed > 0:
        sys.exit(1)


def _run(
    cfg: dict[str, Any],
    out_dir: Path,
    login_url: str,
    voters_csv: Path,
    voter_offset: int,
    max_votes: int | None,
    concurrency: int,
    vote_timeout: int,
    candidates_pattern: str,
    headed: bool,
    playwright_bin: Path,
    env: dict[str, str],
    election_event_id: str,
    tenant_id: str,
) -> dict:
    # --- Output layout ---
    out_dir.mkdir(parents=True, exist_ok=True)
    common.log(f"Writing manifest/report/results to {out_dir}")

    # --- Voter manifest ---
    # Every CSV column is passed through (not just username/password): the
    # login form's fields depend on the realm's configured match-attributes
    # (e.g. dateOfBirth alongside — or instead of — a voter-id username), and
    # the CSV columns are named to match those fields, so the whole row is
    # what the browser flow needs to fill in.
    with voters_csv.open(newline="") as f:
        rows = list(csv.reader(f))
    header, data_rows = rows[0], rows[1:]
    if "username" not in header or "password" not in header:
        common.die(f"{voters_csv} has no username/password columns (header: {','.join(header)})")
    selected = data_rows[voter_offset:]
    if max_votes:
        selected = selected[:max_votes]
    manifest = [
        dict(zip(header, row))
        for row in selected
        if row[header.index("username")] and row[header.index("password")]
    ]
    manifest_path = out_dir / "voters.json"
    common.write_json(manifest_path, manifest)
    count = len(manifest)
    if count == 0:
        common.die(f"no voter rows found in {voters_csv} after skipping voter_offset {voter_offset}")
    common.log(f"Rendered voter manifest with {count} voters")

    # --- Drive the browsers ---
    report_json = out_dir / "playwright-report.json"
    cast_csv = out_dir / "cast_ballots.csv"
    cast_csv.write_text("")

    run_env = {
        **env,
        "LOGIN_URL": login_url,
        "LOAD_TEST_MANIFEST": str(manifest_path),
        "LOAD_TEST_CAST_CSV": str(cast_csv),
        "LOAD_TEST_OUT_DIR": str(out_dir),
        "LOAD_TEST_VOTE_TIMEOUT_MS": str(vote_timeout * 1000),
        "CANDIDATES_PATTERN": candidates_pattern,
        "PLAYWRIGHT_JSON_OUTPUT_NAME": str(report_json),
    }
    if headed:
        run_env["LOAD_TEST_HEADED"] = "true"

    common.log(f"Casting {count} votes with concurrency {concurrency} (timeout {vote_timeout}s each)")
    started_at = datetime.now(timezone.utc)
    start_ts = time.monotonic()
    proc = subprocess.run(
        [str(playwright_bin), "test", "--config", PLAYWRIGHT_CONFIG, "--workers", str(concurrency), "--reporter=json,line"],
        cwd=LOAD_TEST_DIR,
        env=run_env,
    )
    pw_exit = proc.returncode
    elapsed = round(time.monotonic() - start_ts)
    finished_at = datetime.now(timezone.utc)
    if not report_json.is_file():
        common.die(f"Playwright exited with {pw_exit} without writing {report_json} — the run crashed before any result was produced")

    # --- Results ---
    # Per-voter status/duration come from the Playwright JSON report (which
    # also covers voters killed by the per-test timeout); ballot ids come
    # from the rows the spec appends to cast_ballots.csv on success.
    ballot_ids: dict[str, str] = {}
    with cast_csv.open() as f:
        for row in csv.reader(f):
            if row:
                ballot_ids[row[0]] = row[2] if len(row) > 2 else ""

    with report_json.open() as f:
        report = json.load(f)

    results_path = out_dir / "results.csv"
    cast = 0
    failed = 0
    with results_path.open("w") as results_f:
        results_f.write("voter_id,status,duration_ms,ballot_id\n")
        for spec in iter_report_specs(report):
            voter_id = str(spec.get("title", "")).removeprefix("voter ")
            status = dig(spec, "tests", 0, "results", 0, "status", default="unknown")
            duration = round(dig(spec, "tests", 0, "results", 0, "duration", default=0))
            if status == "passed":
                cast += 1
            else:
                failed += 1
            results_f.write(f"{voter_id},{status},{duration},{ballot_ids.get(voter_id, '')}\n")

    if cast + failed == 0:
        common.die(f"the Playwright run (exit {pw_exit}) produced no per-voter results — it failed before any test ran; check the output above and {report_json}")
    if cast + failed != count:
        common.log(f"WARNING: expected {count} voters but the report covers {cast + failed}")

    tenant_summary = {
        "tenant_id": tenant_id,
        "election_event_id": election_event_id,
        "login_url": login_url,
        "concurrency": concurrency,
        "voter_offset": voter_offset,
        "vote_timeout_secs": vote_timeout,
        "started_at": common.format_timestamp(started_at),
        "finished_at": common.format_timestamp(finished_at),
        "elapsed_secs": elapsed,
        "total_votes": count,
        "cast": cast,
        "failed": failed,
        "results_csv": str(results_path),
        "traces_dir": str(out_dir / "traces"),
    }
    common.write_json(out_dir / "summary.json", tenant_summary)

    common.log(f"Tenant {tenant_id} done in {elapsed}s: {cast}/{count} voters cast a ballot ({failed} did not)")
    common.log(f"Per-voter results: {results_path}")
    common.log(f"Failure traces:    {out_dir / 'traces'}/ (open with: {playwright_bin} show-trace <trace.zip>)")
    return tenant_summary


if __name__ == "__main__":
    main()
