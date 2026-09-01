#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Stage 2 of the telephone (IVR) load test: takes the outputs of Stage 1
(setup_telephone_load_test.py: a summary.json + voters CSV), generates a
local phone_config.json and one DTMF input script per voter from a captured
call template, then fans out N parallel `ivr-cli --bundle dev` processes —
each an independent simulated phone call against the dev container's real
Keycloak/Hasura. See
docs/docusaurus/docs/07-developers/12-ivr/telephone-load-testing-design.md.

The DTMF template is captured empirically: run one interactive call by hand
against the Stage-1 event and note every keystroke (see
dtmf-template.example.txt next to this script for the procedure), replacing
the identifier/PIN entries with {{PIN}}, plus whichever of {{VOTER_ID}} /
{{DOB}} matches this realm's auth flow (check {realm}/ivr-config — it varies
per realm, e.g. voter_id+pin vs dateOfBirth+pin).

Takes no command-line arguments — every setting lives in
telephone-load-test-inputs/layers.yaml, under 'telephone_run:'.

Requires the `ivr-cli` binary (cd beyond/packages && cargo build --release -p
ivr-cli) and a Redis-compatible session store; if none is reachable this
script starts a local `valkey` docker container (disable with
telephone_run.start_valkey: false).
"""

from __future__ import annotations

import csv
import json
import os
import re
import socket
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path
from shutil import which

import load_test_common as common

VALKEY_CONTAINER_NAME = "ivr-load-test-valkey"
VALKEY_IMAGE = "valkey/valkey:8-alpine"
VALKEY_PORT = 6379
DEFAULT_SYSTEM_NUMBER = "+111111111111"
# Printed by the BallotReceipt prompt only after a ballot has actually been
# cast (builtin_prompts.rs: "Your ballot locator for {election_name} is, ...").
DEFAULT_SUCCESS_REGEX = "ballot locator"


def find_ivr_cli(configured: str | None) -> str:
    if configured:
        path = Path(configured)
        if path.is_file() and os.access(path, os.X_OK):
            return str(path)
        common.die(f"telephone_run.ivr_cli_bin '{configured}' is not an executable file")
    candidates = [which("ivr-cli")]
    beyond_root = common.REPO_ROOT / "beyond" / "packages"
    candidates += [
        beyond_root / "rust-local-target" / "release" / "ivr-cli",
        beyond_root / "target" / "release" / "ivr-cli",
    ]
    for candidate in candidates:
        if candidate and Path(candidate).is_file() and os.access(candidate, os.X_OK):
            return str(candidate)
    common.die(
        "ivr-cli binary not found. Build it: (cd beyond/packages && cargo build --release -p ivr-cli), "
        "or set telephone_run.ivr_cli_bin"
    )
    raise AssertionError("unreachable")


def env_file_get(env_file: Path, key: str) -> str | None:
    if not env_file.is_file():
        return None
    prefix = f"{key}="
    for line in env_file.read_text().splitlines():
        if line.startswith(prefix):
            return line[len(prefix):]
    return None


def port_open(host: str, port: int, timeout: float = 1) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


def docker_inspect(name: str, fmt: str) -> str | None:
    proc = subprocess.run(["docker", "inspect", name, "--format", fmt], capture_output=True, text=True)
    if proc.returncode != 0:
        return None
    return proc.stdout.strip() or None


def resolve_valkey_url(configured: str | None, start_valkey: bool) -> str:
    if configured:
        return configured
    if port_open("127.0.0.1", VALKEY_PORT):
        return f"redis://127.0.0.1:{VALKEY_PORT}"

    # This script (and ivr-cli) typically run inside the devcontainer, which
    # is itself a container on the compose stack's docker network — a plain
    # `docker run -p` publishes to the outer host's network namespace, which
    # is unreachable from here. Attach the auto-started container to the same
    # network as an already-running stack service (keycloak) instead, and
    # address it by container name rather than 127.0.0.1.
    compose_network = docker_inspect("keycloak", "{{range $k,$v := .NetworkSettings.Networks}}{{$k}}{{end}}")
    if not compose_network:
        common.die(
            f"no Redis-compatible session store reachable at 127.0.0.1:{VALKEY_PORT}, and couldn't find the "
            "compose network (no running 'keycloak' container) to start one on; set telephone_run.valkey_url"
        )
    container_state = docker_inspect(VALKEY_CONTAINER_NAME, "{{.State.Status}}")
    if container_state != "running":
        if not (start_valkey and which("docker")):
            common.die(
                "no session store reachable and auto-start is disabled (telephone_run.start_valkey: false) "
                "or docker is unavailable; set telephone_run.valkey_url"
            )
        if container_state:
            common.log(f"Restarting existing, stopped {VALKEY_CONTAINER_NAME}")
            subprocess.run(["docker", "start", VALKEY_CONTAINER_NAME], check=True, stdout=subprocess.DEVNULL)
        else:
            common.log(f"Starting {VALKEY_CONTAINER_NAME} ({VALKEY_IMAGE}) on network {compose_network}")
            subprocess.run(
                ["docker", "run", "-d", "--name", VALKEY_CONTAINER_NAME, "--network", compose_network, VALKEY_IMAGE],
                check=True,
                stdout=subprocess.DEVNULL,
            )
    valkey_url = f"redis://{VALKEY_CONTAINER_NAME}:{VALKEY_PORT}"
    for _ in range(30):
        if port_open(VALKEY_CONTAINER_NAME, VALKEY_PORT):
            break
        time.sleep(1)
    else:
        common.die(f"{VALKEY_CONTAINER_NAME} did not become reachable on {compose_network}")
    return valkey_url


def csv_column_index(header: list[str], name: str) -> int | None:
    return header.index(name) if name in header else None


def run_one_call(
    input_file: Path,
    out_dir: Path,
    ivr_cli_bin: str,
    system_number: str,
    call_timeout: int,
    env: dict[str, str],
) -> None:
    voter_id = input_file.stem.removeprefix("call-")
    # Unique fake ANI per call; the caller's number is only blacklist-checked,
    # never used for auth, so any well-formed unique number works.
    caller = f"+1555{int(voter_id):07d}"
    log_file = out_dir / "logs" / f"call-{voter_id}.log"
    with log_file.open("w") as log_f:
        try:
            proc = subprocess.run(
                [ivr_cli_bin, "--bundle", "dev", "--system-number", system_number, "--number", caller, "--input-file", str(input_file)],
                stdout=log_f,
                stderr=subprocess.STDOUT,
                timeout=call_timeout,
                env=env,
            )
            rc = proc.returncode
        except subprocess.TimeoutExpired:
            rc = 124  # match bash `timeout`'s exit code on kill
    with (out_dir / "exit_codes.csv").open("a") as f:
        f.write(f"{voter_id},{rc}\n")


def main() -> None:
    config = common.load_config()
    cfg = common.section(config, "telephone_run")

    run_dir = common.resolve_path(common.req_str(cfg, "run_dir"))
    summary_json = run_dir / "summary.json"
    if not summary_json.is_file():
        common.die(f"no summary.json in {run_dir} — run setup_telephone_load_test.py first")

    dtmf_template = common.resolve_path(common.req_str(cfg, "dtmf_template"))
    if not dtmf_template.is_file():
        common.die(f"no such file: {dtmf_template}")
    template_text = dtmf_template.read_text()
    if "{{PIN}}" not in template_text:
        common.die(f"template {dtmf_template} has no {{{{PIN}}}} placeholder")
    if "{{VOTER_ID}}" not in template_text and "{{DOB}}" not in template_text:
        common.die(
            f"template {dtmf_template} has no {{{{VOTER_ID}}}} or {{{{DOB}}}} placeholder "
            "(whichever this realm's auth flow identifies voters by)"
        )

    concurrency = int(cfg.get("concurrency") or 10)
    max_calls = cfg.get("max_calls")
    max_calls = int(max_calls) if max_calls else None
    voter_offset = int(cfg.get("voter_offset") or 0)
    if voter_offset < 0:
        common.die("telephone_run.voter_offset must be a non-negative integer")
    call_timeout = int(cfg.get("call_timeout") or 300)
    system_number = str(cfg.get("system_number") or DEFAULT_SYSTEM_NUMBER)
    success_regex = re.compile(str(cfg.get("success_regex") or DEFAULT_SUCCESS_REGEX), re.IGNORECASE)
    out_dir = common.resolve_path(common.req_str(cfg, "out_dir"))

    ivr_cli_bin = find_ivr_cli(cfg.get("ivr_cli_bin") or None)
    common.log(f"Using ivr-cli: {ivr_cli_bin}")

    # --- Read Stage 1's summary.json ---
    with summary_json.open() as f:
        summary = json.load(f)

    tenant_id = summary.get("tenant_id")
    election_event_id = summary.get("election_event_id")
    keycloak_realm = summary.get("keycloak_realm")
    keycloak_url = cfg.get("keycloak_url") or summary.get("keycloak_url")
    # summary.json stores step-cli's GraphQL endpoint (.../v1/graphql);
    # ivr-core's phone_config.hasura_url is the bare Hasura base URL, which it
    # appends its own /api/rest/ivr/... paths to (see
    # election_config_hasura.rs) - strip the GraphQL suffix so the IVR REST
    # calls don't 404 on a doubled-up path.
    hasura_url = (cfg.get("hasura_url") or summary.get("hasura_url") or "").removesuffix("/v1/graphql")
    voters_csv = Path(summary.get("voters_csv", ""))
    # voting_channel was added to summary.json when Stage 1 grew a
    # voting_channel setting; an empty value is an older TELEPHONE-only run
    # dir, which is fine.
    voting_channel = summary.get("voting_channel")
    if voting_channel and voting_channel != "TELEPHONE":
        common.die(
            f"{summary_json} was provisioned for the {voting_channel} channel — re-run "
            "setup_telephone_load_test.py with setup.voting_channel: TELEPHONE (the IVR eligibility "
            "check gates on the TELEPHONE channel being open)"
        )
    for name, value in [
        ("tenant_id", tenant_id), ("election_event_id", election_event_id), ("keycloak_realm", keycloak_realm),
        ("keycloak_url", keycloak_url), ("hasura_url", hasura_url),
    ]:
        if not value:
            common.die(f"could not read {name} from {summary_json}")
    if not voters_csv.is_file():
        # Stage 1 writes voters_csv as an absolute path; tolerate a moved run dir.
        voters_csv = run_dir / voters_csv.name
    if not voters_csv.is_file():
        common.die(f"voters CSV not found: {summary.get('voters_csv')}")
    common.log(f"Election event {election_event_id} (tenant {tenant_id}), voters: {voters_csv}")

    # --- Keycloak IVR client secrets ---
    dev_env_file = common.REPO_ROOT / ".devcontainer" / ".env.development"
    ivr_service_client_id = os.environ.get("KEYCLOAK_IVR_SERVICE_CLIENT_ID") or env_file_get(dev_env_file, "KEYCLOAK_IVR_SERVICE_CLIENT_ID") or "ivr-service"
    ivr_voting_client_id = os.environ.get("KEYCLOAK_IVR_VOTING_CLIENT_ID") or "ivr-voting"
    ivr_service_client_secret = os.environ.get("KEYCLOAK_IVR_SERVICE_CLIENT_SECRET") or env_file_get(dev_env_file, "KEYCLOAK_IVR_SERVICE_CLIENT_SECRET")
    ivr_voting_client_secret = os.environ.get("KEYCLOAK_IVR_VOTING_CLIENT_SECRET") or env_file_get(dev_env_file, "KEYCLOAK_IVR_VOTING_CLIENT_SECRET")
    if not ivr_service_client_secret:
        common.die(f"KEYCLOAK_IVR_SERVICE_CLIENT_SECRET not set and not found in {dev_env_file}")
    if not ivr_voting_client_secret:
        common.die(f"KEYCLOAK_IVR_VOTING_CLIENT_SECRET not set and not found in {dev_env_file}")

    # --- Session store (valkey) ---
    valkey_url = resolve_valkey_url(cfg.get("valkey_url") or os.environ.get("VALKEY_URL"), bool(cfg.get("start_valkey", True)))
    common.log(f"Session store: {valkey_url}")

    # --- Output layout ---
    (out_dir / "inputs").mkdir(parents=True, exist_ok=True)
    (out_dir / "logs").mkdir(parents=True, exist_ok=True)
    common.log(f"Writing call inputs/logs to {out_dir}")

    # --- phone_config.json ---
    # Same shape as ivr-core's ports/phone_config.rs (see the fixture at
    # adapters/mock/fixtures/phone_config.json). cluster_id/region/environment
    # are required by the deserializer but unused outside AWS routing.
    phone_config_path = out_dir / "phone_config.json"
    common.write_json(phone_config_path, {
        "entries": {
            system_number: {
                "tenant_id": tenant_id,
                "election_event_id": election_event_id,
                "keycloak_realm": keycloak_realm,
                "cluster_id": "dev",
                "region": "local",
                "environment": "dev",
                "keycloak_url": keycloak_url,
                "hasura_url": hasura_url,
                "default_language": "en",
                "enabled": True,
            }
        }
    })
    common.log(f"Generated {phone_config_path}")

    # --- Per-voter DTMF input files ---
    # dateOfBirth is optional in the CSV - only realms whose IVR auth flow
    # identifies voters by date of birth (checked live via
    # {realm}/ivr-config, not assumed) need {{DOB}} in the template; realms
    # using the generic voter_id+pin default flow only need
    # {{VOTER_ID}}/{{PIN}}.
    with voters_csv.open(newline="") as f:
        rows = list(csv.reader(f))
    header, data_rows = rows[0], rows[1:]
    username_col = csv_column_index(header, "username")
    password_col = csv_column_index(header, "password")
    dob_col = csv_column_index(header, "dateOfBirth")
    if username_col is None or password_col is None:
        common.die(f"{voters_csv} has no username/password columns (header: {','.join(header)})")

    count = 0
    for row in data_rows[voter_offset:]:
        voter_id = row[username_col]
        pin = row[password_col]
        if not voter_id or not pin:
            continue
        # generate-voters writes dateOfBirth as YYYY-MM-DD; DTMF collection
        # is raw digits only (YYYYMMDD - a phone keypad has no "-" key).
        dob = row[dob_col].replace("-", "") if dob_col is not None else ""
        rendered = template_text.replace("{{VOTER_ID}}", voter_id).replace("{{PIN}}", pin).replace("{{DOB}}", dob)
        (out_dir / "inputs" / f"call-{voter_id}.txt").write_text(rendered)
        count += 1
        if max_calls and count >= max_calls:
            break
    if count == 0:
        common.die(f"no voter rows found in {voters_csv} after skipping voter_offset {voter_offset}")
    common.log(f"Rendered {count} DTMF input files")

    # --- Fan out the calls ---
    call_env = {
        **os.environ,
        "PHONE_CONFIG_PATH": str(phone_config_path),
        "VALKEY_URL": valkey_url,
        "KEYCLOAK_IVR_SERVICE_CLIENT_ID": ivr_service_client_id,
        "KEYCLOAK_IVR_SERVICE_CLIENT_SECRET": ivr_service_client_secret,
        "KEYCLOAK_IVR_VOTING_CLIENT_ID": ivr_voting_client_id,
        "KEYCLOAK_IVR_VOTING_CLIENT_SECRET": ivr_voting_client_secret,
        "RUST_LOG": os.environ.get("RUST_LOG", "info"),
    }

    common.log(f"Placing {count} calls with concurrency {concurrency} (timeout {call_timeout}s each)")
    exit_codes_path = out_dir / "exit_codes.csv"
    exit_codes_path.write_text("")
    started_at = datetime.now(timezone.utc)
    start_ts = time.monotonic()
    input_files = sorted((out_dir / "inputs").glob("call-*.txt"))
    with ThreadPoolExecutor(max_workers=concurrency) as pool:
        list(pool.map(lambda fp: run_one_call(fp, out_dir, ivr_cli_bin, system_number, call_timeout, call_env), input_files))
    elapsed = round(time.monotonic() - start_ts)
    finished_at = datetime.now(timezone.utc)

    # --- Results ---
    results_path = out_dir / "results.csv"
    cast = 0
    failed = 0
    with exit_codes_path.open() as f, results_path.open("w") as results_f:
        results_f.write("voter_id,exit_code,ballot_cast\n")
        for line in f:
            line = line.strip()
            if not line:
                continue
            voter_id, rc = line.split(",", 1)
            log_text = (out_dir / "logs" / f"call-{voter_id}.log").read_text(errors="replace") if (out_dir / "logs" / f"call-{voter_id}.log").is_file() else ""
            if success_regex.search(log_text):
                results_f.write(f"{voter_id},{rc},true\n")
                cast += 1
            else:
                results_f.write(f"{voter_id},{rc},false\n")
                failed += 1

    summary_out = {
        "run_dir": str(run_dir),
        "election_event_id": election_event_id,
        "tenant_id": tenant_id,
        "dtmf_template": str(dtmf_template),
        "concurrency": concurrency,
        "voter_offset": voter_offset,
        "call_timeout_secs": call_timeout,
        "system_number": system_number,
        "started_at": started_at.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "finished_at": finished_at.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "elapsed_secs": elapsed,
        "total_calls": count,
        "cast": cast,
        "failed": failed,
        "results_csv": str(results_path),
        "logs_dir": str(out_dir / "logs"),
    }
    summary_out_path = out_dir / "summary.json"
    common.write_json(summary_out_path, summary_out)

    common.log(f"Done in {elapsed}s: {cast}/{count} calls cast a ballot ({failed} did not)")
    common.log(f"Per-call results: {results_path}")
    common.log(f"Per-call logs:    {out_dir / 'logs'}/")
    common.log(f"Run summary:      {summary_out_path}")
    if failed > 0:
        common.log(f"Inspect a failed call's log for where the flow diverged from the template (searched for /{success_regex.pattern}/i as the cast marker)")
        sys.exit(1)


if __name__ == "__main__":
    main()
