# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run one private browser capture and collect both PostgreSQL JSON logs.

The target uses environment-variable names for DSNs. This observer never changes
server logging settings; devenv must be configured for diagnostic statement logs.
"""

import argparse
import glob
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.request

import psycopg

from resources import extract
from report import generate
from measurements import summarize_sql
from traffic import inventory, validate_s3_flow
from replay_profile import compile_profile


ROOT = Path(__file__).resolve().parents[2]


def save(path: Path, value: object) -> None:
    """Write private machine-readable evidence without exposing it in console output."""
    path.write_text(json.dumps(value, indent=2) + "\n")
    path.chmod(0o600)


def log_positions(pattern: str) -> dict[str, int]:
    """Remember existing log lengths so the capture excludes earlier journeys."""
    return {name: Path(name).stat().st_size for name in glob.glob(pattern)}


def collect_logs(pattern: str, positions: dict[str, int], database: str) -> list[dict]:
    """Collect new JSON records, including rotated files, without claiming attribution."""
    records = []
    for name in sorted(glob.glob(pattern)):
        with open(name) as stream:
            stream.seek(positions.get(name, 0))
            for line in stream:
                record = json.loads(line)
                if (
                    record.get("dbname") == database
                    and record.get("application_name") != "voting_e2e_observer"
                ):
                    records.append(record)
    return records


def connect(database: dict):
    """Open a clearly identified observer connection using an environment-held DSN."""
    return psycopg.connect(
        os.environ[database["dsn_env"]],
        application_name="voting_e2e_observer",
        connect_timeout=5,
        autocommit=True,
    )


def preflight(target: dict) -> dict:
    """Require live portal/CDP endpoints and readable statement logs for both databases."""
    results = {}
    endpoints = {"portal": target["login_url"]}
    if target.get("engine", "chromium") == "obscura":
        endpoints["obscura"] = target["cdp_url"].rstrip("/") + "/json/version"
    for name, url in endpoints.items():
        try:
            with urllib.request.urlopen(url, timeout=5) as response:
                results[name] = {"ready": response.status < 400}
        except Exception as error:
            # Exception text may include a credential-bearing DSN or URL.
            results[name] = {"ready": False, "error_type": type(error).__name__}
    for name in ("backend", "keycloak"):
        try:
            database = target["databases"][name]
            with connect(database) as connection:
                row = connection.execute(
                    "SELECT current_database(), current_setting('log_statement'), current_setting('log_min_duration_statement'), current_setting('log_destination'), current_setting('auto_explain.log_nested_statements', true), current_setting('auto_explain.log_min_duration', true)"
                ).fetchone()
            readable = log_positions(database["jsonlog_glob"])
            ready = (
                bool(readable)
                and (row[1] == "all" or row[2] == "0")
                and "jsonlog" in row[3]
            )
            results[name] = {
                "ready": ready,
                "database": row[0],
                "log_statement": row[1],
                "log_min_duration_statement": row[2],
                "json_logs_readable": bool(readable),
                "nested_sql_configured": row[4] == "on" and row[5] in ("0", "0ms"),
                "request_attribution": "unverified",
            }
        except Exception as error:
            results[name] = {"ready": False, "error_type": type(error).__name__}
    return results


def verify_casts(target: dict, casts: list[dict]) -> bool:
    """Reconcile each accepted API response with its persisted ballot and scope."""
    if not casts:
        return False
    with connect(target["databases"]["backend"]) as connection:
        for cast in casts:
            row = connection.execute(
                "SELECT ballot_id, tenant_id::text, election_event_id::text, election_id::text FROM sequent_backend.cast_vote WHERE id = %s",
                (cast["id"],),
            ).fetchone()
            expected = tuple(
                cast[key]
                for key in (
                    "ballot_id",
                    "tenant_id",
                    "election_event_id",
                    "election_id",
                )
            )
            if (
                row != expected
                or cast["tenant_id"] != target["tenant_id"]
                or cast["election_event_id"] != target["election_event_id"]
            ):
                return False
    return True


def run(target: dict, target_path: Path, output: Path) -> int:
    """Capture a single journey and fail closed on unavailable targets or missing casts."""
    readiness = preflight(target)
    save(output / "preflight.json", readiness)
    if not all(value["ready"] for value in readiness.values()):
        return 2
    positions = {
        name: log_positions(db["jsonlog_glob"])
        for name, db in target["databases"].items()
    }
    proxy_path = (
        Path(target["action_proxy_log"]) if target.get("action_proxy_log") else None
    )
    proxy_offset = (
        proxy_path.stat().st_size if proxy_path and proxy_path.exists() else 0
    )
    env = dict(
        os.environ,
        CAPTURE_TARGET=str(target_path.resolve()),
        CAPTURE_OUTPUT_DIR=str(output),
    )
    # Nightwatch also installs Playwright in this workspace. Resolve the CLI
    # from the same @playwright/test package that imports the test declaration.
    test_package = subprocess.check_output(
        ["node", "-p", "require.resolve('@playwright/test/package.json')"],
        cwd=ROOT / "packages/voting-portal",
        text=True,
    ).strip()
    result = subprocess.run(
        [
            "node",
            str(Path(test_package).parent / "cli.js"),
            "test",
            "--config",
            "playwright.capture.config.ts",
            "status.spec.ts" if target.get("mode") == "status" else "capture.spec.ts",
        ],
        cwd=ROOT / "packages/voting-portal",
        env=env,
        timeout=240,
        check=False,
    )
    # Logging collector output can lag request completion; collection is diagnostic,
    # not part of the measured browser interval. Remaining gaps stay explicit.
    time.sleep(1)
    summaries = {}
    for name, database in target["databases"].items():
        records = collect_logs(
            database["jsonlog_glob"], positions[name], readiness[name]["database"]
        )
        summaries[name] = summarize_sql(records, target.get("sql_clients", {}))
        save(
            output / f"{name}-sql.json",
            records,
        )
    if proxy_path:
        with proxy_path.open() as stream:
            stream.seek(proxy_offset)
            save(
                output / "action-http.json",
                [json.loads(line) for line in stream if line.strip()],
            )
    capture_path = output / "capture.json"
    if not capture_path.exists():
        save(
            capture_path,
            {
                "completed": False,
                "persistence_verified": False,
                "requests": [],
                "casts": [],
                "sql_summary": summaries,
                "failure": "Playwright exited without a capture artifact",
                "runner_exit_code": result.returncode,
            },
        )
        return 1
    capture = json.loads(capture_path.read_text())
    capture["sql_summary"] = summaries
    capture["logging_configuration"] = readiness
    capture["completed"] = capture["completed"] and result.returncode == 0
    capture["persistence_verified"] = (
        None
        if target.get("mode") == "status"
        else verify_casts(target, capture["casts"])
    )
    capture["traffic"] = inventory(capture["requests"])
    capture["path_errors"] = (
        validate_s3_flow(capture["requests"])
        if target.get("expected_path") == "s3" and target.get("mode") != "status"
        else []
    )
    capture["completed"] = capture["completed"] and not capture["path_errors"]
    capture["sql_attribution"] = (
        "database capture interval; background SQL may be included"
    )
    save(output / "capture.json", capture)
    if (
        capture["completed"]
        and capture["persistence_verified"]
        and target.get("expected_path") == "s3"
        and capture["engine"] == "chromium"
    ):
        save(
            output / "profile.json",
            compile_profile(capture, json.loads((output / "journey.har").read_text())),
        )
    save(
        output / "resource-profile.json",
        dict(
            extract(json.loads((output / "journey.har").read_text())),
            observed_traffic=capture["traffic"],
        ),
    )
    return (
        0
        if result.returncode == 0
        and capture["completed"]
        and (capture["persistence_verified"] or target.get("mode") == "status")
        else 1
    )


def main() -> None:
    """Expose one command for capture and an independent readiness-only mode."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--preflight-only", action="store_true")
    args = parser.parse_args()
    os.umask(0o077)
    args.output.mkdir(parents=True, exist_ok=False)
    target = json.loads(args.target.read_text())
    try:
        if args.preflight_only:
            results = preflight(target)
            save(args.output / "preflight.json", results)
            raise SystemExit(
                0 if all(item["ready"] for item in results.values()) else 2
            )
        raise SystemExit(run(target, args.target, args.output.resolve()))
    finally:
        # An unavailable stack is a documented result, never a successful E2E run.
        generate(args.output)


if __name__ == "__main__":
    main()
