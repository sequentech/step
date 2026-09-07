#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Merges the outputs of several run_online_load_test.py runs — typically one
per load client machine, each having cast a disjoint voter slice
(online_run.voter_offset / max_votes) against the same deployment — into one
results.csv and one summary.json, plus a per-client table on stderr.

Unlike the setup/run scripts this one takes positional arguments: the
directories to merge are whatever was collected from the clients, not a
deployment setting that belongs in layers.yaml.

    python3 aggregate_online_load_test.py [--out DIR] DIR [DIR ...]

Each DIR is either one client's online_run.out_dir (it holds a summary.json)
or a directory whose immediate children are such out_dirs — e.g. the target
of an `rsync client:.../online-load-test-output/votes/ collected/<client>/`
loop. Counts are additive and voter ids are globally unique across a
properly sharded run, so the CSVs simply concatenate; a voter id seen from
more than one client is reported as a sharding overlap (that voter's second
attempt was rejected as a duplicate vote, so its failure is not the
server's). Throughput spans the earliest started_at to the latest
finished_at across every client.
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

import load_test_common as common

DEFAULT_OUT_DIR = common.SCRIPTS_DIR / "online-load-test-output" / "aggregate"


def discover_client_dirs(paths: list[Path]) -> list[Path]:
    found: list[Path] = []
    for path in paths:
        if not path.is_dir():
            common.die(f"{path} is not a directory")
        if (path / "summary.json").is_file():
            found.append(path)
            continue
        children = sorted(child for child in path.iterdir() if (child / "summary.json").is_file())
        if not children:
            common.die(f"{path} holds no summary.json, and none of its subdirectories does either")
        found.extend(children)
    return found


def load_client(client_dir: Path) -> dict[str, Any]:
    with (client_dir / "summary.json").open() as f:
        summary = json.load(f)
    tenants = summary.get("tenants") or []
    if not tenants:
        common.die(f"{client_dir / 'summary.json'} lists no tenants — is this a run_online_load_test.py out_dir?")
    # Older summaries (before the client/started_at fields were added at the
    # top level) still carry both per tenant.
    started = summary.get("started_at") or min(t["started_at"] for t in tenants)
    finished = summary.get("finished_at") or max(t["finished_at"] for t in tenants)
    rows: list[dict[str, str]] = []
    for tenant in tenants:
        results_csv = Path(tenant.get("results_csv", ""))
        if not results_csv.is_file():
            # results_csv is written as an absolute path on the client;
            # tolerate a copied directory the same way Stage 2 does.
            results_csv = client_dir / f"tenant-{tenant['tenant_id']}" / "results.csv"
        if not results_csv.is_file():
            common.die(f"results.csv for tenant {tenant['tenant_id']} not found under {client_dir}")
        with results_csv.open(newline="") as f:
            for row in csv.DictReader(f):
                rows.append({"tenant_id": tenant["tenant_id"], **row})
    return {
        "client": summary.get("client") or client_dir.name,
        "dir": str(client_dir),
        "concurrency": summary.get("concurrency"),
        "voter_offset": summary.get("voter_offset"),
        "max_votes": summary.get("max_votes"),
        "start_at": summary.get("start_at"),
        "started_at": started,
        "finished_at": finished,
        "total_votes": summary.get("total_votes", len(rows)),
        "cast": summary.get("cast", sum(1 for r in rows if r["status"] == "passed")),
        "failed": summary.get("failed", sum(1 for r in rows if r["status"] != "passed")),
        "rows": rows,
    }


def percentile(sorted_values: list[int], fraction: float) -> int | None:
    if not sorted_values:
        return None
    index = min(len(sorted_values) - 1, max(0, round(fraction * (len(sorted_values) - 1))))
    return sorted_values[index]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("dirs", nargs="+", type=Path, help="client out_dirs, or directories containing them")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT_DIR, help=f"where to write the merged results.csv and summary.json (default: {DEFAULT_OUT_DIR})")
    args = parser.parse_args()

    clients = [load_client(d) for d in discover_client_dirs(args.dirs)]

    all_rows = [dict(client=c["client"], **row) for c in clients for row in c["rows"]]
    seen = Counter(row["voter_id"] for row in all_rows)
    overlaps = sorted(voter_id for voter_id, n in seen.items() if n > 1)

    total = sum(c["total_votes"] for c in clients)
    cast = sum(c["cast"] for c in clients)
    failed = sum(c["failed"] for c in clients)
    throughput = common.run_throughput(clients, cast)
    durations = sorted(int(row["duration_ms"]) for row in all_rows if row["status"] == "passed" and row.get("duration_ms"))

    args.out.mkdir(parents=True, exist_ok=True)
    results_path = args.out / "results.csv"
    with results_path.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["client", "tenant_id", "voter_id", "status", "duration_ms", "ballot_id"])
        writer.writeheader()
        writer.writerows(all_rows)

    summary = {
        "clients": [{k: v for k, v in c.items() if k != "rows"} for c in clients],
        "total_votes": total,
        "cast": cast,
        "failed": failed,
        **throughput,
        "vote_duration_ms": {
            "p50": percentile(durations, 0.5),
            "p95": percentile(durations, 0.95),
            "max": durations[-1] if durations else None,
        },
        "overlapping_voter_ids": overlaps,
        "results_csv": str(results_path),
    }
    summary_path = args.out / "summary.json"
    common.write_json(summary_path, summary)

    width = max(len(c["client"]) for c in clients)
    print(f"{'client':<{width}}  {'cast':>6} {'failed':>6} {'total':>6}  started_at            finished_at           conc", file=sys.stderr)
    for c in clients:
        print(
            f"{c['client']:<{width}}  {c['cast']:>6} {c['failed']:>6} {c['total_votes']:>6}  "
            f"{c['started_at']}  {c['finished_at']}  {c['concurrency'] or '-'}",
            file=sys.stderr,
        )
    print(f"{'ALL':<{width}}  {cast:>6} {failed:>6} {total:>6}  {summary['started_at']}  {summary['finished_at']}", file=sys.stderr)
    common.log(f"{cast}/{total} ballots cast across {len(clients)} client(s) in {throughput['elapsed_secs']}s: {throughput['cast_per_second']} cast/second")
    if durations:
        common.log(f"Per-vote duration (successful): p50 {summary['vote_duration_ms']['p50']}ms, p95 {summary['vote_duration_ms']['p95']}ms, max {summary['vote_duration_ms']['max']}ms")
    if overlaps:
        common.log(f"WARNING: {len(overlaps)} voter id(s) were attempted by more than one client — check each client's voter_offset/max_votes slice: {', '.join(overlaps[:10])}{'...' if len(overlaps) > 10 else ''}")
    common.log(f"Merged results: {results_path}")
    common.log(f"Merged summary: {summary_path}")


if __name__ == "__main__":
    main()
