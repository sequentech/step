# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Render an aggregate performance report; protocol details stay in private JSON."""
from __future__ import annotations

import html
import base64
import io
import math
from pathlib import Path
import sqlite3
from string import Template

LABELS = {
    "status_ms": "Voter status",
    "cast_ms": "Cast acceptance",
    "journey_ms": "Complete journey",
}


def number(value: float | None, unit: str = "ms") -> str:
    """Absent stages are unavailable, never a fictitious zero-duration response."""
    return "—" if value is None else f"{value:,.1f} {unit}"


def render(directory: Path, db: sqlite3.Connection, config: dict, result: dict) -> None:
    """Generate responsive HTML and an SVG with bounded aggregate data only."""
    from aggregate import percentile
    import matplotlib

    matplotlib.use("Agg")
    matplotlib.rcParams.update(
        {
            "svg.hashsalt": "sequent-voting-load",
            "svg.fonttype": "none",
            "font.family": "DejaVu Sans",
            "font.size": 9,
            "text.color": "#142d3a",
            "axes.labelcolor": "#607680",
            "xtick.color": "#607680",
            "ytick.color": "#607680",
            "axes.spines.top": False,
            "axes.spines.right": False,
            "axes.edgecolor": "#dce5e8",
        }
    )
    import matplotlib.pyplot as plt

    figure, axes = plt.subplots(1, 2, figsize=(11, 2.7), constrained_layout=True)
    status_only = result["mode"] == "status"
    for column, color in (("status_ms", "#087f76"), ("cast_ms", "#4576b8")):
        points = [(percentile(db, column, p / 100), p) for p in range(0, 101, 2)]
        points = [(x, y) for x, y in points if x is not None]
        if points:
            axes[0].plot(*zip(*points), label=LABELS[column], color=color, linewidth=2)
    axes[0].set(
        xlabel="Response time · milliseconds",
        ylabel="Percent of responses",
        ylim=(0, 100),
    )
    axes[0].grid(axis="y", alpha=0.12)
    if axes[0].lines:
        axes[0].legend(frameon=False, loc="lower right")
    if result["completed"]:
        start, end = db.execute("SELECT min(start), max(end) FROM samples").fetchone()
        bucket_count = min(
            config.get("reporting", {}).get("bins", 60),
            max(1, math.ceil((end - start) / 1000)),
        )
        width = max(1, end - start) / bucket_count
        predicate = "passed = 1" if status_only else "receipt IS NOT NULL"
        bins = dict(
            db.execute(
                f"SELECT min(cast((end-?)/? AS INTEGER), ?), count(*) FROM samples WHERE {predicate} GROUP BY 1",
                (start, width, bucket_count - 1),
            )
        )
        positions = range(bucket_count)
        axes[1].bar(
            [(b + 0.5) * width / 1000 for b in positions],
            [bins.get(b, 0) * 1000 / width for b in positions],
            width=width / 1000 * 0.85,
            color="#087f76",
        )
        axes[1].set_xlim(0, bucket_count * width / 1000)
    axes[1].set(
        xlabel="Elapsed time · seconds",
        ylabel="Successful journeys / s" if status_only else "Accepted casts / s",
    )
    axes[1].grid(axis="y", alpha=0.12)
    chart = io.StringIO()
    figure.savefig(chart, format="svg", metadata={"Date": None})
    plt.close(figure)
    svg = "<svg" + chart.getvalue().split("<svg", 1)[1]
    (directory / "performance.svg").write_text(svg)
    latency_rows = "".join(
        f'<tr><td>{LABELS[name]}</td><td class="num">{number(values["p50"])}</td><td class="num">{number(values["p99"])}</td></tr>'
        for name, values in result["latency"].items()
        if values["p50"] is not None
    )
    goals = []
    for stage, limits in config.get("goals", {}).items():
        if stage == "min_casts_per_second":
            actual = result["casts_per_second"]
            goals.append(("Accepted casts / s", f"≥ {limits:g}", actual >= limits))
        else:
            for quantile, limit in limits.items():
                actual = result["latency"][stage][quantile]
                goals.append(
                    (
                        f"{LABELS[stage]} {quantile}",
                        f"≤ {limit:g} ms",
                        actual is not None and actual <= limit,
                    )
                )
    goals_html = (
        '<table><thead><tr><th>Measure</th><th class="num">Target</th><th class="num">Result</th></tr></thead><tbody>'
        + "".join(
            f'<tr><td>{name}</td><td class="num">{target}</td><td class="num {"pass" if passed else "fail"}">{"Passed" if passed else "Missed"}</td></tr>'
            for name, target, passed in goals
        )
        + "</tbody></table>"
        if goals
        else "<p>No latency or throughput thresholds configured.</p>"
    )
    goals_html += (
        '<p class="note">Every planned journey must succeed'
        + ("." if status_only else " and return a unique ballot receipt.")
        + "</p>"
    )
    throughput = (
        result["passed"] / result["elapsed_seconds"]
        if status_only and result["elapsed_seconds"]
        else result["casts_per_second"]
    )
    cards = [
        (
            "Successful journeys",
            f'{result["passed"]:,} / {result["planned"]:,}',
            "Distinct synthetic voters",
        ),
        (
            "Journeys / second" if status_only else "Accepted casts / second",
            f"{throughput:,.2f}",
            "Across the complete measured interval",
        ),
        (
            "Journey p99",
            number(result["latency"]["journey_ms"]["p99"]),
            "Login through final response",
        ),
        (
            "Measured duration",
            number(result["elapsed_seconds"], "s"),
            f'{result["planned"] - result["passed"]:,} missing or failed journeys',
        ),
    ]
    cards_html = "".join(
        f'<div class="card"><div class="label">{label}</div><div class="value">{value}</div><div class="note">{note}</div></div>'
        for label, value, note in cards
    )
    errors = (
        '<section class="errors"><strong>Needs attention</strong><ul>'
        + "".join(f"<li>{html.escape(error)}</li>" for error in result["errors"])
        + "</ul></section>"
        if result["errors"]
        else ""
    )
    browser = result["engine"] == "chromium"
    verdict = "Passed" if not result["errors"] else "Failed"
    content = Template((Path(__file__).parent / "report.html").read_text()).substitute(
        regular_font=base64.b64encode(
            (
                Path(__file__).resolve().parents[2]
                / "packages/admin-portal/public/roboto/Roboto_latin_400.woff2"
            ).read_bytes()
        ).decode(),
        bold_font=base64.b64encode(
            (
                Path(__file__).resolve().parents[2]
                / "packages/admin-portal/public/roboto/Roboto_latin_700.woff2"
            ).read_bytes()
        ).decode(),
        font_license=html.escape(
            "Roboto Copyright 2015 Google Inc.\n"
            + (
                Path(__file__).resolve().parents[2] / "LICENSES/Apache-2.0.txt"
            ).read_text()
        ),
        verdict=verdict,
        title=(
            "Voter status performance" if status_only else "Voting journey performance"
        ),
        subtitle=f'{"Chromium · Browser journey" if browser else "k6 · HTTP journey"} · {config["vus"]} concurrent voters per worker',
        badge_background="#e5f4ee" if not errors else "#fce9e4",
        badge_color="#14704b" if not errors else "#a6372c",
        cards=cards_html,
        chart=svg,
        latency_rows=latency_rows,
        goals=goals_html,
        errors=errors,
        coverage=(
            "Includes login, rendering, ballot encryption and cast acceptance."
            if browser
            else (
                "Includes login and voter-status responses."
                if status_only
                else "Includes login, voter status, publication downloads and cast acceptance. Encryption is prepared beforehand."
            )
        ),
        verification=html.escape(result["persistence_verification"]),
    )
    (directory / "report.html").write_text(content)
    (directory / "performance.md").write_text(
        f'# Voting performance · {verdict}\n\n{result["passed"]:,}/{result["planned"]:,} successful journeys.\n\n![Performance](performance.svg)\n'
    )
