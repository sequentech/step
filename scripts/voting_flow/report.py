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


def compact_count(value):
    if value >= 1_000_000 and value % 1_000_000 == 0:
        return f"{value // 1_000_000}M"
    if value >= 1000 and value % 1000 == 0:
        return f"{value // 1000}k"
    return str(value)


def scenario_label(scenario):
    return (
        f"{compact_count(scenario['seeded_ballots'])} votes table, "
        f"{scenario['peak_voters']} concurrent voters, "
        f"{compact_count(scenario['unrelated_schedules'])} other same-event schedules"
    )


def measurement_tables(report):
    rows = [
        "| Scenario | Ballots | Peak concurrent voters | Other same-event schedules | Before p50 / p99 (ms) | After p50 / p99 (ms) |",
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
            f"| {scenario_label(scenario)} | {scenario['seeded_ballots']:,} | "
            f"{scenario['peak_voters']} | {scenario['unrelated_schedules']:,} | "
            f"{before['p50_ms']:.2f} / {before['p99_ms']:.2f} | "
            f"{after['p50_ms']:.2f} / {after['p99_ms']:.2f} |"
        )
        throughput.append(
            f"| {scenario_label(scenario)} | {before['elapsed_seconds']:.4f} | "
            f"{after['elapsed_seconds']:.4f} | {before['requests_per_second']:.1f} | "
            f"{after['requests_per_second']:.1f} | {after['accepted_requests']:,} | "
            f"{before['errors']} / {after['errors']} |"
        )
    return "\n".join(rows) + "\n\n" + "\n".join(throughput)


def schedule_tables(report):
    by_placement = {}
    for scenario in report["scenarios"]:
        by_placement.setdefault(scenario["placement"], {})[scenario["indexed"]] = (
            scenario["queries"]
        )
    broad = [
        "| Extra schedules in | Rows returned | Broad query without index p50 (ms) | With index p50 (ms) |",
        "|---|---:|---:|---:|",
    ]
    selection = [
        "| Extra schedules in | Two-endpoint query without index p50 (ms) | With index p50 (ms) | Projection p50 (ms) |",
        "|---|---:|---:|---:|",
    ]
    writes = [
        "| Extra schedules in | Reschedule without index p50 (ms) | With index p50 (ms) |",
        "|---|---:|---:|",
    ]
    for placement, variants in by_placement.items():
        before, after = variants[False], variants[True]
        broad.append(
            f"| {placement} | {before['broad_event']['returned_rows']:,} | "
            f"{before['broad_event']['p50_ms']:.3f} | {after['broad_event']['p50_ms']:.3f} |"
        )
        selection.append(
            f"| {placement} | {before['two_endpoints']['p50_ms']:.3f} | "
            f"{after['two_endpoints']['p50_ms']:.3f} | {after['projection']['p50_ms']:.3f} |"
        )
        writes.append(
            f"| {placement} | {before['reschedule']['p50_ms']:.3f} | "
            f"{after['reschedule']['p50_ms']:.3f} |"
        )
    return "\n\n".join("\n".join(table) for table in (broad, selection, writes))


def main():
    report = json.loads(REPORT.read_text())
    guide = GUIDE.read_text()
    prefix, remainder = guide.split(START)
    _, suffix = remainder.split(END)
    reference = next(
        s
        for s in report["scenarios"]
        if (s["seeded_ballots"], s["peak_voters"], s["unrelated_schedules"])
        == (100_000, 8, 100)
    )
    before, after = reference["results"]
    calculation = (
        "**Accepted votes/second = accepted submissions / elapsed measurement seconds.** "
        "Elapsed time is the sum of the opening, lull and closing phase wall times; "
        "seeding and warmups are excluded. Each measured submission uses a distinct voter.\n\n"
        f"For the {scenario_label(reference)} workload, before: {before['accepted_requests']:,} / "
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
            "repeats the 100k votes table workloads at 8 and 64 concurrent voters on this release branch. "
            "The same accepted-votes/elapsed-seconds calculation applies. "
            f"[Raw verification report](/benchmarks/{verification_path.name}).\n\n"
            + measurement_tables(verification)
            + "\n"
        )
    updated = prefix + START + "\n\n" + measurements + "\n" + END + suffix
    schedule_path = REPORT.with_name("schedule-indexes.json")
    if schedule_path.exists():
        schedule_report = json.loads(schedule_path.read_text())
        schedule_start = "<!-- schedule-index-benchmark:start -->"
        schedule_end = "<!-- schedule-index-benchmark:end -->"
        before, remainder = updated.split(schedule_start)
        _, after = remainder.split(schedule_end)
        evidence = (
            f"Measurements at `{schedule_report['implementation_commit'][:10]}`. "
            "[Raw schedule-query evidence](/benchmarks/schedule-indexes.json).\n\n"
            + schedule_tables(schedule_report)
        )
        updated = (
            before + schedule_start + "\n\n" + evidence + "\n\n" + schedule_end + after
        )
    GUIDE.write_text(updated)
    print(f"Updated benchmark tables in {GUIDE.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
