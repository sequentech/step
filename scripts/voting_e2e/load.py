# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Prepare encrypted ballots, dispatch disjoint load shards and verify global goals.

All inputs/output are private. Keep the preparation claims directory durable when
moving workers to ephemeral machines; never retry or reissue an attempted shard.
"""
import argparse
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys
import time
from urllib.parse import urlsplit

from capture import (
    ROOT,
    save,
    connect,
    preflight,
    log_positions,
    collect_logs,
    verify_casts,
)
from measurements import percentile, summarize_sql
from traffic import inventory


def runner_sources() -> dict:
    """Fingerprint tracked and new runner source files without reading private inputs."""
    paths = [
        *Path(__file__).parent.glob("*.py"),
        *Path(__file__).parent.glob("*.js"),
        *(ROOT / "packages/voting-portal/test/load").glob("*.ts"),
        *(ROOT / "packages/voting-portal/test/load").glob("*.cjs"),
        Path(__file__).parent / "worker.sh",
        ROOT / "devenv.lock",
        ROOT / "devenv.nix",
    ]
    return {
        str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(paths)
    }


def claim(path: Path):
    """Permanently reserve an attempt; a crash must not make a cast replayable."""
    with path.open("x") as stream:
        stream.write(f"{time.time()}\n")


def partition(rows: list, nodes: int, per_node: int) -> list[list]:
    """Assign every selected voter once without wrapping an exhausted census."""
    if nodes < 1 or per_node < 1 or len(rows) != nodes * per_node:
        raise ValueError(
            "Require exactly nodes × rate × duration distinct prepared ballots"
        )
    return [rows[node::nodes] for node in range(nodes)]


def assess(
    samples: list[dict], expected: int, start: float, duration: int, goals: dict
) -> dict:
    """Evaluate merged raw samples, never average worker percentiles or rates."""
    casts = [s for s in samples if s.get("kind") == "cast"]
    latencies = [s["duration_ms"] for s in casts]
    elapsed = max(
        duration * 1000, max((s["ended_at_ms"] for s in casts), default=start) - start
    )
    accepted = sum(bool(s["accepted"]) for s in casts)
    p50 = percentile(latencies, 50) if latencies else None
    p99 = percentile(latencies, 99) if latencies else None
    rate = accepted * 1000 / elapsed
    passed = (
        len(casts) == expected == accepted
        and len(samples) == len(casts)
        and p50 is not None
        and p50 <= goals["p50_ms"]
        and p99 <= goals["p99_ms"]
        and rate >= goals["min_casts_per_second"]
    )
    return dict(
        expected_casts=expected,
        attempted_casts=len(casts),
        accepted_casts=accepted,
        failed_casts=len(casts) - accepted,
        dropped=sum(s.get("kind") == "dropped" for s in samples),
        runner_errors=sum(s.get("kind") == "runner_error" for s in samples),
        missing_attempts=max(0, expected - len(casts)),
        p50_ms=p50,
        p99_ms=p99,
        measured_window_ms=elapsed,
        accepted_casts_per_second=rate,
        goals=goals,
        goals_passed=passed,
    )


def playwright(spec: str, output: Path, env: dict) -> int:
    """Resolve the CLI from the same Playwright package used by the tests."""
    cwd = ROOT / "packages/voting-portal"
    package = subprocess.check_output(
        ["node", "-p", "require.resolve('@playwright/test/package.json')"],
        cwd=cwd,
        text=True,
    ).strip()
    with (output / "runner.log").open("w") as log:
        return subprocess.run(
            [
                "node",
                str(Path(package).parent / "cli.js"),
                "test",
                "--config",
                "playwright.capture.config.ts",
                spec,
            ],
            cwd=cwd,
            env=dict(os.environ, CAPTURE_OUTPUT_DIR=str(output), **env),
            stdout=log,
            stderr=log,
            timeout=1800,
        ).returncode


def prepare(args):
    """Use imported census voters to encrypt without sending any cast mutation."""
    started = time.monotonic()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    target = json.loads(args.target.read_text())
    old = json.loads(args.refresh.read_text()) if args.refresh else None
    if old:
        old = old[args.offset : args.offset + args.count]
        voters = [row["credentials"] for row in old]
    else:
        with args.voters.open() as stream:
            voters = list(csv.DictReader(stream))[
                args.offset : args.offset + args.count
            ]
        if len(voters) != args.count or len({v["username"] for v in voters}) != len(
            voters
        ):
            raise ValueError("Require requested distinct census voters")
    if len(voters) != args.count or len({v["username"] for v in voters}) != len(voters):
        raise ValueError(
            "Require requested distinct census voters for preparation or refresh"
        )
    checks = preflight(target)
    save(output / "preflight.json", checks)
    if not all(item["ready"] for item in checks.values()):
        raise RuntimeError(
            "Preparation prerequisites failed; inspect private preflight"
        )
    save(
        output / "input.json",
        dict(target=target, voters=voters, refresh=old, concurrency=args.concurrency),
    )
    code = playwright(
        "prepare.spec.ts",
        output,
        dict(PREPARE_INPUT=str(output / "input.json"), PREPARE_OUTPUT=str(output)),
    )
    if code:
        raise RuntimeError(
            "Ballot preparation failed; partial output must not be dispatched"
        )
    rows = json.loads((output / "ballots.json").read_text())
    ids = [row["payload"]["variables"]["ballotId"] for row in rows]
    with connect(target["databases"]["backend"]) as connection:
        count = connection.execute(
            "SELECT count(*) FROM sequent_backend.cast_vote WHERE tenant_id=%s AND election_event_id=%s AND ballot_id=ANY(%s)",
            (target["tenant_id"], target["election_event_id"], ids),
        ).fetchone()[0]
    if count or len(rows) != len(voters) or len(set(ids)) != len(ids):
        raise RuntimeError(
            "Prepared ballots are incomplete, duplicated or already persisted"
        )
    save(
        output / "prepared.json",
        dict(
            schema_version=1,
            count=len(rows),
            casts_during_preparation=0,
            ballots_sha256=hashlib.sha256(
                (output / "ballots.json").read_bytes()
            ).hexdigest(),
            source_commit=subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
            ).strip(),
            earliest_token_expiry=min(row["token_expires_at"] for row in rows),
        ),
    )
    print(
        f"Prepared {len(rows)} distinct encrypted ballots; zero persisted casts",
        flush=True,
    )


def dispatch(args):
    """Create immutable disjoint shards with a common start time and durable claims."""
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    target = json.loads(args.target.read_text())
    source = args.prepared.resolve()
    prepared = json.loads((source.parent / "prepared.json").read_text())
    profile_path = getattr(args, "profile", None)
    profile = json.loads(profile_path.read_text()) if profile_path else None
    if profile and (
        profile.get("schema_version") != 1 or profile.get("browser") != "chromium"
    ):
        raise ValueError("Require a generated Chromium profile")
    if profile and args.engine != "k6":
        raise ValueError("Profile replay uses k6; Chromium full UI uses capture.py")
    if profile and any(
        profile.get("scope", {}).get(key) != target[key]
        for key in ("tenant_id", "election_event_id")
    ):
        raise ValueError("Capture a Chromium profile for the target event")
    if hashlib.sha256(source.read_bytes()).hexdigest() != prepared["ballots_sha256"]:
        raise ValueError("Prepared fixture digest mismatch")
    count = args.nodes * args.rate * args.duration
    rows = json.loads(source.read_text())[args.offset : args.offset + count]
    shards = partition(rows, args.nodes, args.rate * args.duration)
    ids = [r["payload"]["variables"]["ballotId"] for r in rows]
    if (
        len(set(ids)) != count
        or len({r["credentials"]["username"] for r in rows}) != count
    ):
        raise ValueError("Duplicate prepared voter or ballot")
    start = round(time.time() * 1000) + args.start_delay * 1000
    for row in rows:
        url = urlsplit(row["url"])
        if (
            f"{url.scheme}://{url.netloc}" not in target["allowed_origins"]
            or url.path != "/v1/graphql"
            or url.query
            or url.username
            or url.password
        ):
            raise ValueError(
                "Prepared endpoint must be the explicitly allowed GraphQL endpoint"
            )
        if (
            not profile
            and row["token_expires_at"] * 1000 < start + (args.duration + 60) * 1000
        ):
            raise ValueError(
                "Refresh prepared authentication before dispatch; tokens expire during run"
            )
        if (
            row["tenant_id"] != target["tenant_id"]
            or row["election_event_id"] != target["election_event_id"]
        ):
            raise ValueError("Prepared scope does not match target")
        if row["payload"]["operationName"] != "InsertCastVote":
            raise ValueError("Only prepared cast mutations may be dispatched")
    ledger = args.ledger.resolve()
    ledger.mkdir(parents=True, exist_ok=True)
    # Reserve the whole assignment before starting any process. Partial claim
    # failures deliberately leave reservations in place for operator reconciliation.
    for ballot_id in ids:
        claim(ledger / hashlib.sha256(ballot_id.encode()).hexdigest())
    goals = dict(p50_ms=args.p50, p99_ms=args.p99, min_casts_per_second=args.min_cps)
    if profile:
        goals.update(journey_p50_ms=args.journey_p50, journey_p99_ms=args.journey_p99)
    manifest = dict(
        schema_version=1,
        nodes=args.nodes,
        rate_per_node=args.rate,
        duration_seconds=args.duration,
        expected_casts=count,
        engine=args.engine,
        start_at_ms=start,
        goals=goals,
        architecture=platform.machine(),
        logical_cpus=os.cpu_count(),
        fixture_sha256=prepared["ballots_sha256"],
        source_commit=prepared["source_commit"],
        shards=[],
        mode="journey" if profile else "cast-only",
        runner_sources=runner_sources(),
        profile_sha256=(
            hashlib.sha256(profile_path.read_bytes()).hexdigest() if profile else None
        ),
        profile_requests=len(profile["steps"]) if profile else None,
    )
    for node, ballots in enumerate(shards):
        directory = output / f"node-{node:03d}"
        directory.mkdir()
        save(directory / "ballots.json", ballots)
        save(
            directory / "config.json",
            dict(
                node=node,
                rate=args.rate,
                duration_seconds=args.duration,
                vus=args.vus,
                goals=goals,
                start_at_ms=start,
                engine=args.engine,
                login_url=target["login_url"],
                allowed_origins=target["allowed_origins"],
                cdp_url=target.get("cdp_url"),
                pacing=getattr(args, "pacing", 1),
            ),
        )
        assigned_files = ["ballots.json", "config.json"]
        if profile:
            save(directory / "profile.json", profile)
            assigned_files.append("profile.json")
        save(
            directory / "assignment.json",
            {
                name: hashlib.sha256((directory / name).read_bytes()).hexdigest()
                for name in assigned_files
            },
        )
        manifest["shards"].append(directory.name)
    save(output / "run.json", manifest)
    save(output / "target.json", target)
    return output, manifest, target


def worker(directory: Path) -> int:
    """Execute one assigned shard once; the claim is never cleared after a crash."""
    directory = directory.resolve()
    claim(directory / "attempted")
    assignment = json.loads((directory / "assignment.json").read_text())
    if set(assignment) not in (
        {"ballots.json", "config.json"},
        {"ballots.json", "config.json", "profile.json"},
    ) or any(
        hashlib.sha256((directory / name).read_bytes()).hexdigest() != digest
        for name, digest in assignment.items()
    ):
        raise ValueError("Assigned shard digest mismatch")
    config = json.loads((directory / "config.json").read_text())
    env = dict(
        LOAD_CONFIG=str(directory / "config.json"),
        LOAD_BALLOTS=str(directory / "ballots.json"),
        LOAD_SUMMARY=str(directory / "k6-summary.json"),
        LOAD_SAMPLES=str(directory / "samples.jsonl"),
    )
    if "profile.json" in assignment:
        env["LOAD_PROFILE"] = str(directory / "profile.json")
    if config["engine"] == "k6":
        with (directory / "runner.log").open("w") as log:
            code = subprocess.run(
                [
                    "k6",
                    "run",
                    "--log-format",
                    "raw",
                    "--console-output",
                    env["LOAD_SAMPLES"],
                    str(ROOT / "scripts/voting_e2e/cast.k6.js"),
                ],
                env=dict(os.environ, **env),
                stdout=log,
                stderr=log,
                timeout=max(0, (config["start_at_ms"] - time.time() * 1000) / 1000)
                + config["duration_seconds"]
                + 180,
            ).returncode
    else:
        code = playwright("prepared.spec.ts", directory, env)
    save(directory / "worker.json", dict(exit_code=code))
    return code


def merge(output: Path, publish_docs: bool = False) -> dict:
    """Reconcile all receipts with prepared IDs and storage before evaluating goals."""
    manifest = json.loads((output / "run.json").read_text())
    target = json.loads((output / "target.json").read_text())
    samples = []
    journeys = []
    http_requests = []
    scheduler_dropped = 0
    boundary_ticks = 0
    workers_ok = True
    for name in manifest["shards"]:
        directory = output / name
        worker_path = directory / "worker.json"
        workers_ok &= (
            worker_path.exists()
            and json.loads(worker_path.read_text())["exit_code"] == 0
        )
        summary_path = directory / "k6-summary.json"
        if summary_path.exists():
            metrics = json.loads(summary_path.read_text()).get("metrics", {})
            scheduler_dropped += (
                metrics.get("dropped_iterations", {}).get("values", {}).get("count", 0)
            )
            boundary_ticks += (
                metrics.get("arrival_boundary_ticks", {})
                .get("values", {})
                .get("count", 0)
            )
        ballots = json.loads((directory / "ballots.json").read_text())
        path = directory / "samples.jsonl"
        local = (
            [
                json.loads(line)
                for line in path.read_text().splitlines()
                if line.startswith('{"kind":')
            ]
            if path.exists()
            else []
        )
        for item in local:
            item["node"] = name
        journeys.extend(item for item in local if item["kind"] == "journey")
        http_requests.extend(item for item in local if item["kind"] == "http")
        local = [item for item in local if item["kind"] not in ("journey", "http")]
        seen = set()
        for sample in local:
            index = sample["index"]
            if index in seen or not 0 <= index < len(ballots):
                raise ValueError("Duplicate or out-of-range worker sample")
            seen.add(index)
            if (
                sample.get("accepted")
                and sample["receipt"]["ballot_id"]
                != ballots[index]["payload"]["variables"]["ballotId"]
            ):
                raise ValueError("Worker receipt does not match assigned ballot")
            sample["node"] = name
        samples.extend(local)
    receipts = [s["receipt"] for s in samples if s.get("accepted")]
    persisted = verify_casts(target, receipts) if receipts else False
    result = assess(
        samples,
        manifest["expected_casts"],
        manifest["start_at_ms"],
        manifest["duration_seconds"],
        manifest["goals"],
    )
    result["scheduler_dropped_iterations"] = scheduler_dropped
    if manifest.get("mode") == "journey":
        unique = {(item["node"], item["index"]) for item in journeys}
        valid = len(journeys) == len(unique) == manifest["expected_casts"] and all(
            item["passed"] for item in journeys
        )
        counts = {}
        for item in http_requests + [
            item for item in samples if item["kind"] == "cast"
        ]:
            key = item["node"], item["index"]
            counts[key] = counts.get(key, 0) + 1
        parity = all(counts.get(key) == manifest["profile_requests"] for key in unique)
        valid &= parity
        result["profile_requests_per_journey"] = manifest["profile_requests"]
        result["observed_http_requests"] = sum(counts.values())
        result["profile_request_count_matched"] = parity
        durations = [item["duration_ms"] for item in journeys]
        result["journey"] = dict(
            completed=sum(item["passed"] for item in journeys),
            p50_ms=percentile(durations, 50) if durations else None,
            p99_ms=percentile(durations, 99) if durations else None,
        )
        result["goals_passed"] &= (
            valid
            and bool(durations)
            and result["journey"]["p50_ms"] <= manifest["goals"]["journey_p50_ms"]
            and result["journey"]["p99_ms"] <= manifest["goals"]["journey_p99_ms"]
        )
        for item in http_requests:
            item["timing"] = {"responseEnd": item["duration_ms"]}
            item["sizes"] = {"responseBodySize": item["response_bytes"]}
        save(output / "http.json", http_requests)
        save(output / "journeys.json", journeys)
        save(
            output / "traffic.json",
            inventory(
                http_requests
                + [
                    dict(
                        url=s["endpoint"],
                        method="POST",
                        operation="InsertCastVote",
                        status=s["status"],
                        timing={"responseEnd": s["duration_ms"]},
                        sizes={"responseBodySize": s["response_bytes"]},
                    )
                    for s in samples
                    if s["kind"] == "cast"
                ]
            ),
        )
    result["arrival_boundary_ticks"] = boundary_ticks
    result["goals_passed"] = result["goals_passed"] and scheduler_dropped == 0
    result.update(
        workers_passed=workers_ok,
        persistence_verified=persisted,
        passed=result["goals_passed"] and workers_ok and persisted,
    )
    save(output / "samples.json", samples)
    save(output / "results.json", result)
    from load_report import generate

    generate(output, publish_docs)
    print(json.dumps(result, indent=2))
    return result


def local(args):
    """Run separate load-node processes on this machine, collecting interval SQL."""
    output, manifest, target = dispatch(args)
    ready = preflight(target)
    save(output / "preflight.json", ready)
    if not all(item["ready"] for item in ready.values()):
        raise RuntimeError("Load prerequisites failed; no workers started")
    positions = {
        name: log_positions(db["jsonlog_glob"])
        for name, db in target["databases"].items()
    }
    processes = []
    for name in manifest["shards"]:
        log = (output / name / "process.log").open("w")
        process = subprocess.Popen(
            [
                sys.executable,
                str(Path(__file__).resolve()),
                "worker",
                str(output / name),
            ],
            stdout=log,
            stderr=log,
        )
        log.close()
        processes.append(process)
    for process in processes:
        process.wait()
    time.sleep(1)
    summaries = {}
    for name, database in target["databases"].items():
        records = collect_logs(
            database["jsonlog_glob"], positions[name], ready[name]["database"]
        )
        save(output / f"{name}-sql.json", records)
        summaries[name] = summarize_sql(records, target.get("sql_clients", {}))
    save(output / "sql-summary.json", summaries)
    result = merge(output, getattr(args, "publish_docs", False))
    raise SystemExit(0 if result["passed"] else 1)


def main():
    """Expose preparation, independent worker dispatch and local orchestration."""
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("target", type=Path)
    p.add_argument("voters", type=Path)
    p.add_argument("output", type=Path)
    p.add_argument("--count", type=int, default=10)
    p.add_argument("--offset", type=int, default=0)
    p.add_argument("--refresh", type=Path)
    p.add_argument("--concurrency", type=int, default=1)
    for name in ("local", "dispatch"):
        p = sub.add_parser(name)
        if name == "local":
            p.add_argument("--publish-docs", action="store_true")
        p.add_argument("target", type=Path)
        p.add_argument("prepared", type=Path)
        p.add_argument("output", type=Path)
        p.add_argument("--ledger", type=Path, required=True)
        p.add_argument("--nodes", type=int, default=2)
        p.add_argument("--rate", type=int, default=1, help="Offered arrivals/s per worker")
        p.add_argument("--duration", type=int, default=3)
        p.add_argument("--vus", type=int, default=2)
        p.add_argument("--offset", type=int, default=0)
        p.add_argument("--start-delay", type=int, default=15)
        p.add_argument("--engine", choices=("k6", "chromium"), default="k6")
        p.add_argument(
            "--profile",
            type=Path,
            help="profile.json generated by a verified Chromium capture",
        )
        p.add_argument(
            "--cast-only",
            action="store_true",
            help="Explicitly omit login/resources/status",
        )
        p.add_argument(
            "--pacing",
            type=float,
            default=1,
            help="Multiply observed Chromium request offsets; 0 removes delays",
        )
        p.add_argument("--journey-p50", type=float, default=5000)
        p.add_argument("--journey-p99", type=float, default=10000)
        p.add_argument("--p50", type=float, default=1000)
        p.add_argument("--p99", type=float, default=2000)
        p.add_argument("--min-cps", type=float, default=1)
    for name in ("worker", "merge"):
        command = sub.add_parser(name)
        command.add_argument("directory", type=Path)
        if name == "merge":
            command.add_argument("--publish-docs", action="store_true")
    report = sub.add_parser("report")
    report.add_argument("directory", type=Path)
    report.add_argument("--publish-docs", action="store_true")
    args = parser.parse_args()
    if args.command in ("local", "dispatch"):
        if bool(args.profile) == args.cast_only:
            parser.error("Choose --profile for journey replay or --cast-only")
        if not math.isfinite(args.pacing) or args.pacing < 0:
            parser.error("pacing must be finite and nonnegative")
    for name in ("nodes", "rate", "duration", "vus", "count", "start_delay"):
        if hasattr(args, name) and getattr(args, name) < 1:
            parser.error(f"{name} must be positive")
    if hasattr(args, "concurrency") and not 1 <= args.concurrency <= 8:
        parser.error("preparation concurrency must be between 1 and 8")
    if getattr(args, "offset", 0) < 0:
        parser.error("offset must be nonnegative")
    for name in ("p50", "p99", "min_cps", "journey_p50", "journey_p99"):
        if hasattr(args, name) and (
            not math.isfinite(getattr(args, name)) or getattr(args, name) <= 0
        ):
            parser.error(f"{name} must be finite and positive")
    if args.command == "prepare":
        prepare(args)
    elif args.command == "local":
        local(args)
    elif args.command == "dispatch":
        dispatch(args)
    elif args.command == "worker":
        raise SystemExit(worker(args.directory))
    elif args.command == "report":
        from load_report import generate

        generate(args.directory, args.publish_docs)
    else:
        raise SystemExit(0 if merge(args.directory, args.publish_docs)["passed"] else 1)


if __name__ == "__main__":
    main()
