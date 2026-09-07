# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Refresh Docusaurus measurements from the checked-in benchmark JSON.

Run from the repository root inside devenv:
    python3 scripts/voting_flow/report.py
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REPORT = ROOT / "docs/docusaurus/static/benchmarks/voting-flow.json"
GUIDE = (
    ROOT
    / "docs/docusaurus/docs/07-developers/05-voting-portal/voting-flow-performance.md"
)
START = "<!-- voting-flow-benchmark:start -->"
END = "<!-- voting-flow-benchmark:end -->"


def measurement_tables(report):
    rows = [
        "| Scenario | Ballots | Peak voters | Schedules | Before p50 / p99 (ms) | After p50 / p99 (ms) |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    throughput = [
        "| Scenario | Before casts/s | After casts/s | Accepted per variant | Errors before / after |",
        "|---|---:|---:|---:|---:|",
    ]
    for scenario in report["scenarios"]:
        before, after = scenario["results"]
        assert before["variant"] == "before" and after["variant"] == "after"
        assert before["accepted_requests"] == after["accepted_requests"]
        rows.append(
            f"| {scenario['name']} | {scenario['seeded_ballots']:,} | "
            f"{scenario['peak_voters']} | {scenario['unrelated_schedules']:,} | "
            f"{before['p50_ms']:.2f} / {before['p99_ms']:.2f} | "
            f"{after['p50_ms']:.2f} / {after['p99_ms']:.2f} |"
        )
        throughput.append(
            f"| {scenario['name']} | {before['requests_per_second']:.1f} | "
            f"{after['requests_per_second']:.1f} | {after['accepted_requests']:,} | "
            f"{before['errors']} / {after['errors']} |"
        )
    return "\n".join(rows) + "\n\n" + "\n".join(throughput)


def main():
    report = json.loads(REPORT.read_text())
    guide = GUIDE.read_text()
    prefix, remainder = guide.split(START)
    _, suffix = remainder.split(END)
    measurements = (
        f"SQL-only measurements at implementation `{report['implementation_commit'][:10]}` "
        f"against baseline `{report['baseline_commit']}`.\n\n"
        + measurement_tables(report)
        + "\n\nLatencies cover the complete SQL path per request. Throughput is total "
        "completed requests divided by the combined phase wall time, including "
        "driver scheduling overhead. These measurements come from one local run, not production "
        "capacity estimates or statistical confidence intervals.\n"
    )
    GUIDE.write_text(prefix + START + "\n\n" + measurements + "\n" + END + suffix)
    print(f"Updated benchmark tables in {GUIDE.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
