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
        "| Scenario | Before seconds | After seconds | Before votes/s | After votes/s | Accepted per variant | Errors before / after |",
        "|---|---:|---:|---:|---:|---:|---:|",
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
            f"| {scenario['name']} | {before['elapsed_seconds']:.4f} | "
            f"{after['elapsed_seconds']:.4f} | {before['requests_per_second']:.1f} | "
            f"{after['requests_per_second']:.1f} | {after['accepted_requests']:,} | "
            f"{before['errors']} / {after['errors']} |"
        )
    return "\n".join(rows) + "\n\n" + "\n".join(throughput)


def main():
    report = json.loads(REPORT.read_text())
    guide = GUIDE.read_text()
    prefix, remainder = guide.split(START)
    _, suffix = remainder.split(END)
    reference = next(s for s in report["scenarios"] if s["name"] == "reference")
    before, after = reference["results"]
    calculation = (
        "**Accepted votes/second = accepted submissions / elapsed measurement seconds.** "
        "Elapsed time is the sum of the opening, lull and closing phase wall times; "
        "seeding and warmups are excluded. Each measured submission uses a distinct voter.\n\n"
        f"For the reference workload, before: {before['accepted_requests']:,} / "
        f"{before['elapsed_seconds']:.4f} s = **{before['requests_per_second']:.1f} votes/s**. "
        f"After: {after['accepted_requests']:,} / {after['elapsed_seconds']:.4f} s = "
        f"**{after['requests_per_second']:.1f} votes/s**. "
        "Calculations use unrounded durations from the JSON; displayed durations are rounded.\n\n"
    )
    measurements = (
        f"SQL-only measurements at implementation `{report['implementation_commit'][:10]}` "
        f"against baseline `{report['baseline_commit']}`.\n\n"
        + calculation
        + measurement_tables(report)
        + "\n\nLatencies cover the complete SQL path per request. Throughput is total "
        "completed requests divided by the combined phase wall time, including "
        "driver scheduling overhead. These measurements come from one local run, not production "
        "capacity estimates or statistical confidence intervals.\n"
    )
    verification_path = REPORT.with_name("voting-flow-release-10.json")
    if verification_path.exists():
        verification = json.loads(verification_path.read_text())
        measurements += (
            "\n### Release 10 verification\n\n"
            f"A separate run at implementation `{verification['implementation_commit'][:10]}` "
            "repeats the reference and 64-voter workloads on this release branch. "
            "The same accepted-votes/elapsed-seconds calculation applies. "
            f"[Raw verification report](/benchmarks/{verification_path.name}).\n\n"
            + measurement_tables(verification)
            + "\n"
        )
    GUIDE.write_text(prefix + START + "\n\n" + measurements + "\n" + END + suffix)
    print(f"Updated benchmark tables in {GUIDE.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
