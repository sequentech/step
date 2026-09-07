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
        f"{scenario['election_count']} elections, "
        f"{compact_count(scenario['area_count'])} areas, "
        f"{compact_count(scenario['election_count'] * scenario['schedules_per_election'])} total schedules"
    )


def measurement_tables(report):
    rows = [
        "| Scenario | Before p50 / p99 (ms) | After p50 / p99 (ms) |",
        "|---|---:|---:|",
    ]
    throughput = [
        "| Scenario | Before seconds | After seconds | Before votes/s | After votes/s | Accepted per variant | Errors before / after |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    coverage = [
        "| Scenario | Distinct measured voters | Distinct measured elections | Distinct measured areas |",
        "|---|---:|---:|---:|",
    ]
    for scenario in report["scenarios"]:
        before, after = scenario["results"]
        assert before["variant"] == "before" and after["variant"] == "after"
        assert before["accepted_requests"] == after["accepted_requests"]
        assert before["distinct_measured_areas"] == after["distinct_measured_areas"]
        assert (
            before["distinct_measured_elections"]
            == after["distinct_measured_elections"]
        )
        coverage.append(
            f"| {scenario_label(scenario)} | {after['distinct_measured_voters']:,} | "
            f"{after['distinct_measured_elections']:,} | {after['distinct_measured_areas']:,} |"
        )
        rows.append(
            f"| {scenario_label(scenario)} | "
            f"{before['p50_ms']:.2f} / {before['p99_ms']:.2f} | "
            f"{after['p50_ms']:.2f} / {after['p99_ms']:.2f} |"
        )
        throughput.append(
            f"| {scenario_label(scenario)} | {before['elapsed_seconds']:.4f} | "
            f"{after['elapsed_seconds']:.4f} | {before['requests_per_second']:.1f} | "
            f"{after['requests_per_second']:.1f} | {after['accepted_requests']:,} | "
            f"{before['errors']} / {after['errors']} |"
        )
    return "\n\n".join("\n".join(table) for table in (rows, throughput, coverage))


def factor_comparison(report):
    """Keep the interpretation's quoted values tied to the measured fixtures."""
    scenarios = {scenario["name"]: scenario for scenario in report["scenarios"]}
    areas = [
        scenarios[name]
        for name in ("100k-votes", "100k-votes-1k-areas", "100k-votes-10k-areas")
    ]
    area_medians = ", ".join(
        f"{compact_count(s['area_count'])} areas: {s['results'][1]['p50_ms']:.2f} ms"
        for s in areas
    )
    endpoints = scenarios["100k-votes-200-elections-400-schedules"]["results"]
    all_tasks = scenarios["100k-votes-200-elections"]["results"]
    return (
        "### Comparing the factors\n\n"
        "Holding the 100k votes table, 8 concurrent voters, 10 elections and 100 "
        f"schedules constant, revised-path p50 was **{area_medians}**. "
        "This tests populated area cardinality for point lookups and per-voter history; "
        "it does not measure an area-list response or larger area payloads.\n\n"
        "Holding 200 elections, 100 areas, 100k votes and 8 concurrent voters constant, "
        f"400 versus 2,000 schedules produced baseline p50 **{endpoints[0]['p50_ms']:.2f} "
        f"versus {all_tasks[0]['p50_ms']:.2f} ms**, and revised p50 "
        f"**{endpoints[1]['p50_ms']:.2f} versus {all_tasks[1]['p50_ms']:.2f} ms**. "
        "The broad baseline query transfers every schedule in the event, while the "
        "revised cast reads its election's keyed window. These single-run comparisons "
        "show observed sensitivity, not statistical significance or a capacity guarantee.\n"
    )


def schedule_tables(report):
    by_placement = {}
    for scenario in report["scenarios"]:
        by_placement.setdefault(scenario["placement"], {})[scenario["indexed"]] = (
            scenario["queries"]
        )
    broad = [
        "| Schedule population | Rows returned | Broad query without index p50 (ms) | With index p50 (ms) |",
        "|---|---:|---:|---:|",
    ]
    selection = [
        "| Schedule population | Two-endpoint query without index p50 (ms) | With index p50 (ms) | Projection p50 (ms) |",
        "|---|---:|---:|---:|",
    ]
    writes = [
        "| Schedule population | Reschedule without index p50 (ms) | With index p50 (ms) |",
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
    reference = next(s for s in report["scenarios"] if s["name"] == "100k-votes")
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
        + "\n\n"
        + factor_comparison(report)
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
            "repeats selected area and combined-load workloads on this release branch. "
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
