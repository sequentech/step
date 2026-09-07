# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Render reproducible, standalone SVG figures from measured benchmark JSON."""

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator

COLORS = ("#475569", "#087f8c")
METRICS = (
    ("p50_ms", "p50 latency", "Milliseconds · lower is better"),
    ("p99_ms", "p99 latency", "Milliseconds · lower is better"),
    ("requests_per_second", "Accepted votes per second", "Votes/s · higher is better"),
)
LICENSE = """SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
"""


def figure(title, subtitle):
    plt.rcParams.update(
        {
            "font.size": 10,
            "axes.spines.top": False,
            "axes.spines.right": False,
            "svg.fonttype": "none",
            # Stable identifiers and no timestamp keep regeneration reviewable.
            "svg.hashsalt": "sequent-voting-flow",
        }
    )
    fig, axes = plt.subplots(1, 3, figsize=(13, 3.9))
    fig.suptitle(title, x=0.065, y=0.98, ha="left", fontsize=16, weight="bold")
    fig.text(0.065, 0.87, subtitle, color="#475569", fontsize=10)
    fig.subplots_adjust(left=0.065, right=0.98, bottom=0.22, top=0.72, wspace=0.4)
    return fig, axes


def style_axis(axis, title, ylabel):
    axis.set_title(title, loc="left", fontsize=11, pad=10)
    axis.set_ylabel(ylabel, fontsize=9)
    axis.set_ylim(bottom=0)
    axis.yaxis.set_major_locator(MaxNLocator(nbins=5))
    axis.grid(axis="y", color="#e2e8f0", linewidth=0.8)
    axis.set_axisbelow(True)


def save(fig, path, description):
    fig.savefig(
        path,
        facecolor="white",
        metadata={
            "Date": None,
            "Creator": "Sequent benchmark report",
            "Description": description,
        },
    )
    plt.close(fig)
    path.with_suffix(path.suffix + ".license").write_text(LICENSE)


def comparison_chart(scenarios, labels, title, subtitle, path, combined=False):
    fig, axes = figure(title, subtitle)
    for axis, (key, title, ylabel) in zip(axes, METRICS):
        if combined:
            values = [result[key] for result in scenarios[0]["results"]]
            bars = axis.bar((0, 1), values, color=COLORS, width=0.55)
            axis.set_xticks((0, 1), ("Before", "After"))
            axis.bar_label(
                bars,
                labels=[f"{value:,.2f}" for value in values],
                padding=5,
                fontsize=10,
            )
            axis.margins(y=0.22)
        else:
            for variant, (label, color) in enumerate(zip(("Before", "After"), COLORS)):
                values = [scenario["results"][variant][key] for scenario in scenarios]
                axis.plot(
                    range(len(labels)),
                    values,
                    marker="o",
                    linewidth=2,
                    linestyle="--" if variant == 0 else "-",
                    color=color,
                    label=label,
                )
            # Categorical positions compare the tested populations, without
            # implying interpolation or an arrival-rate model between points.
            axis.set_xticks(range(len(labels)), labels)
            axis.margins(x=0.15, y=0.18)
        style_axis(axis, title, ylabel)
    if not combined:
        fig.legend(
            *axes[0].get_legend_handles_labels(),
            loc="lower center",
            bbox_to_anchor=(0.5, 0.005),
            ncol=2,
            frameon=False,
        )
    save(
        fig,
        path,
        subtitle + ". Local SQL-client measurements; not production capacity.",
    )


def voting_charts(report, directory, prefix="voting-flow"):
    """Only render comparisons present in this report; never fill missing data."""
    scenarios = {scenario["name"]: scenario for scenario in report["scenarios"]}
    specifications = (
        (
            "votes",
            "Vote-table size",
            "8 concurrent voters · 10 elections · 100 areas · 100 total schedules",
            ("10k-votes", "100k-votes", "1m-votes"),
            ("10k votes", "100k votes", "1M votes"),
        ),
        (
            "concurrency",
            "Concurrent voters",
            "100k votes table · 10 elections · 100 areas · 100 total schedules",
            ("100k-votes", "100k-votes-32-voters", "100k-votes-64-voters"),
            ("8 voters", "32 voters", "64 voters"),
        ),
        (
            "areas",
            "Populated areas",
            "100k votes table · 8 concurrent voters · 10 elections · 100 total schedules",
            ("100k-votes", "100k-votes-1k-areas", "100k-votes-10k-areas"),
            ("100 areas", "1k areas", "10k areas"),
        ),
        (
            "schedules",
            "Schedules within a 200-election event",
            "100k votes table · 8 concurrent voters · 200 elections · 100 areas",
            ("100k-votes-200-elections-400-schedules", "100k-votes-200-elections"),
            ("400 schedules", "2k schedules"),
        ),
        (
            "combined",
            "Combined workload",
            "1M votes table · 64 concurrent voters · 200 elections · 10k areas · 2k total schedules",
            ("1m-votes-64-voters-200-elections-10k-areas",),
            (),
        ),
    )
    images = []
    for name, title, subtitle, names, labels in specifications:
        if not all(name in scenarios for name in names):
            continue
        path = directory / f"{prefix}-{name}.svg"
        comparison_chart(
            [scenarios[name] for name in names],
            labels,
            title,
            subtitle,
            path,
            combined=name == "combined",
        )
        images.append(
            f"![{title}: before and after p50, p99 and accepted votes per second. {subtitle}.](/benchmarks/{path.name})"
        )
    return "\n\n".join(images)


def schedule_chart(report, directory):
    pairs = {}
    for scenario in report["scenarios"]:
        pairs.setdefault(scenario["placement"], {})[scenario["indexed"]] = scenario[
            "queries"
        ]
    fig, axes = figure(
        "Schedule indexing within deployment bounds",
        "200 elections and 2k schedules/event · 1 event = 2k rows; 15 events = 30k rows in one database",
    )
    labels = ("1 event", "15 events\nsame tenant", "15 events\ndifferent tenants")
    for axis, (key, title) in zip(
        axes,
        (
            ("broad_event", "Broad event query"),
            ("two_endpoints", "Two-endpoint query"),
            ("reschedule", "Reschedule UPDATE"),
        ),
    ):
        for indexed, color, label in zip(
            (False, True), COLORS, ("Without index", "With index")
        ):
            offset = 0.18 if indexed else -0.18
            axis.bar(
                [i + offset for i in range(len(pairs))],
                [variants[indexed][key]["p50_ms"] for variants in pairs.values()],
                width=0.34,
                color=color,
                label=label,
            )
        axis.set_xticks(range(len(labels)), labels, fontsize=9)
        style_axis(axis, title, "p50 milliseconds · lower is better")
    fig.legend(
        *axes[0].get_legend_handles_labels(),
        loc="lower center",
        bbox_to_anchor=(0.5, -0.005),
        ncol=2,
        frameon=False,
    )
    path = directory / "schedule-indexes.svg"
    save(
        fig,
        path,
        "Single-client query/update costs; the broad query returns 2,000 rows in every case.",
    )
    return "![Schedule index comparison: broad event query, two-endpoint query and rescheduling p50, with and without the index.](/benchmarks/schedule-indexes.svg)"
