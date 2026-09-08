# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Merge raw samples on disk and produce a portable, self-contained load report."""
from __future__ import annotations

from collections import Counter
from contextlib import closing
import json
import math
import os
from pathlib import Path
import sqlite3

from runner import read, save, shard_bounds, shard_count
from presentation import render


def percentile(db: sqlite3.Connection, column: str, fraction: float) -> float | None:
    """Calculate global interpolated percentiles; never average worker percentiles."""
    if column not in {"cast_ms", "status_ms", "journey_ms"}:
        raise ValueError("Unknown latency column")
    count = db.execute(f"SELECT count({column}) FROM samples").fetchone()[0]
    if not count:
        return None
    position = (count - 1) * fraction
    rows = db.execute(
        f"SELECT {column} FROM samples WHERE {column} IS NOT NULL ORDER BY {column} LIMIT 2 OFFSET ?",
        (math.floor(position),),
    ).fetchall()
    low = rows[0][0]
    return low + (rows[-1][0] - low) * (position - math.floor(position))


def verify_receipts(db: sqlite3.Connection, config: dict, dsn_env: str) -> int:
    """Audit accepted IDs in bounded read-only database batches after load timing."""
    import psycopg

    cursor = db.execute("SELECT receipt FROM samples WHERE receipt IS NOT NULL")
    verified = 0
    with psycopg.connect(
        os.environ[dsn_env],
        autocommit=True,
        connect_timeout=config.get("reporting", {}).get(
            "audit_connect_timeout_seconds", 15
        ),
    ) as observer:
        observer.execute("SET default_transaction_read_only=on")
        observer.execute(
            "SELECT set_config('statement_timeout', %s, false)",
            (str(config.get("reporting", {}).get("audit_timeout_ms", 30000)),),
        )
        while rows := cursor.fetchmany(
            config.get("reporting", {}).get("audit_batch_size", 1000)
        ):
            verified += observer.execute(
                "SELECT count(*) FROM sequent_backend.cast_vote WHERE id=ANY(%s::uuid[]) "
                "AND tenant_id=%s::uuid AND election_event_id=%s::uuid",
                (
                    [row[0] for row in rows],
                    config["tenant_id"],
                    config["election_event_id"],
                ),
            ).fetchone()[0]
    return verified


def report(directory: Path, dsn_env: str | None = None) -> dict:
    """Validate complete ownership and receipts, then render aggregate-only artifacts.

    SQLite bounds merge memory independently of voter count. Raw receipts remain
    private; the HTML contains only outcomes, latencies and configured goals.
    API acceptance is reported separately from independent database verification.
    """
    config = read(directory / "config.json")
    database = directory / "measurements.sqlite"
    database.unlink(missing_ok=True)
    traffic: Counter[str] = Counter()
    errors = []
    with closing(sqlite3.connect(database)) as db:
        db.execute(
            f"PRAGMA cache_size=-{int(config.get('reporting', {}).get('sqlite_cache_kib', 16384))}"
        )
        db.execute("PRAGMA temp_store=FILE")
        db.execute(
            "CREATE TABLE samples (voter INTEGER PRIMARY KEY, passed INTEGER, start REAL, end REAL, cast_ms REAL, status_ms REAL, journey_ms REAL, receipt TEXT UNIQUE)"
        )
        for shard in range(shard_count(config)):
            result = directory / "results" / f"{shard:06d}"
            first, count = shard_bounds(config, shard)
            if (
                not (result / "exit.json").exists()
                or read(result / "exit.json")["code"] != 0
            ):
                errors.append(f"Shard {shard} did not finish successfully")
            samples = result / "samples.jsonl"
            if not samples.exists():
                continue
            with samples.open() as stream:
                for line in stream:
                    row = json.loads(line)
                    if not first <= row["index"] < first + count:
                        raise ValueError("Worker submitted a voter outside its shard")
                    try:
                        db.execute(
                            "INSERT INTO samples VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                            (
                                row["index"],
                                row["passed"],
                                row["start"],
                                row["end"],
                                row.get("cast_ms"),
                                row.get("status_ms"),
                                row["end"] - row["start"],
                                row.get("receipt"),
                            ),
                        )
                    except sqlite3.IntegrityError:
                        message = "Duplicate voter or API receipt in worker results"
                        if message not in errors:
                            errors.append(message)
            db.commit()
            summary_path = result / "summary.json"
            if summary_path.exists():
                for name, metric in read(summary_path).get("metrics", {}).items():
                    if name.startswith("http_reqs{name:"):
                        traffic[name[len("http_reqs{name:") : -1]] += metric["values"][
                            "count"
                        ]
            browser_traffic = result / "traffic.json"
            if browser_traffic.exists():
                traffic.update(read(browser_traffic))
        for column in ("cast_ms", "status_ms", "journey_ms"):
            db.execute(f"CREATE INDEX {column}_order ON samples({column})")
        completed, passed, receipts, start, end = db.execute(
            "SELECT count(*), coalesce(sum(passed),0), count(receipt), min(start), max(end) FROM samples"
        ).fetchone()
        elapsed = (end - start) / 1000 if completed else 0
        cps = receipts / elapsed if elapsed else 0
        metrics = {
            column: {
                "p50": percentile(db, column, 0.5),
                "p99": percentile(db, column, 0.99),
            }
            for column in ("cast_ms", "status_ms", "journey_ms")
        }
        if completed != config["count"] or passed != config["count"]:
            errors.append("Missing or failed voter journeys")
        if config.get("mode", "vote") == "vote" and receipts != config["count"]:
            errors.append("Missing unique API receipts")
        for metric, limits in config.get("goals", {}).items():
            if metric == "min_casts_per_second":
                if cps < limits:
                    errors.append(f"Throughput {cps:.2f} below goal {limits}")
            else:
                for quantile, limit in limits.items():
                    actual = metrics[metric][quantile]
                    if actual is None or actual > limit:
                        errors.append(
                            f"{metric} {quantile} exceeds {limit} ms or is missing"
                        )
        verification = "API receipts; no independent database audit"
        if dsn_env:
            verified = verify_receipts(db, config, dsn_env)
            verification = (
                f"{verified}/{receipts} API receipts matched PostgreSQL in batches"
            )
            if verified != receipts:
                errors.append("Database receipt audit is incomplete")
        result = dict(
            engine=config["engine"],
            mode=config.get("mode", "vote"),
            planned=config["count"],
            completed=completed,
            passed=passed,
            accepted_casts=receipts,
            elapsed_seconds=elapsed,
            casts_per_second=cps,
            latency=metrics,
            traffic=dict(traffic),
            errors=errors,
            persistence_verification=verification,
        )
        save(directory / "results.json", result)
        render(directory, db, config, result)
    if errors:
        raise RuntimeError(
            "Load goals failed; inspect report.html: " + "; ".join(errors[:5])
        )
    return result
