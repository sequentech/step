# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Regenerate aggregate tables and plots; publish only the current selected run."""
from collections import Counter
import json
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[2]
MARKER = "<!-- generated-load-results -->"
LICENSE = "SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>\nSPDX-License-Identifier: AGPL-3.0-only\n"


def generate(output: Path, publish: bool = False):
    """Render aggregate load results and optionally refresh the documented example."""
    import matplotlib

    matplotlib.use("Agg")
    matplotlib.rcParams["svg.hashsalt"] = "voting-load"
    matplotlib.rcParams["svg.fonttype"] = "none"
    import matplotlib.pyplot as plt

    result = json.loads((output / "results.json").read_text())
    run = json.loads((output / "run.json").read_text())
    samples = json.loads((output / "samples.json").read_text())
    casts = [s for s in samples if s["kind"] == "cast"]
    fig, axes = plt.subplots(1, 2, figsize=(10, 3.4), constrained_layout=True)
    durations = sorted(s["duration_ms"] for s in casts)
    if durations:
        axes[0].plot(
            durations,
            [(i + 1) * 100 / len(durations) for i in range(len(durations))],
            color="#2563eb",
        )
    axes[0].axvline(
        run["goals"]["p99_ms"], color="#ea580c", linestyle="--", label="p99 budget"
    )
    axes[0].set(
        xlabel="Cast response time (ms)",
        ylabel="Completed attempts (%)",
        title="Cast latency distribution",
    )
    axes[0].legend()
    buckets = Counter(
        max(0, int((s["ended_at_ms"] - run["start_at_ms"]) / 1000))
        for s in casts
        if s["accepted"]
    )
    seconds = list(range(max(buckets, default=0) + 1))
    axes[1].bar(seconds, [buckets[s] for s in seconds], color="#0d9488")
    axes[1].axhline(
        run["goals"]["min_casts_per_second"],
        color="#ea580c",
        linestyle="--",
        label="global rate budget",
    )
    axes[1].set(
        xlabel="Seconds from scheduled start",
        ylabel="Accepted casts / second",
        title="Accepted vote rate",
    )
    axes[1].legend()
    image = output / "performance.svg"
    fig.savefig(image, metadata={"Date": None})
    image.write_text(
        "\n".join(line.rstrip() for line in image.read_text().splitlines()) + "\n"
    )
    plt.close(fig)

    def number(value):
        return "—" if value is None else f"{value:.2f}"

    lines = [
        "## Current measurement",
        "",
        f"**{'PASS' if result['passed'] else 'FAIL'}** · {run['engine']} {run.get('mode', 'cast-only')} · {run['nodes']} workers × {run['rate_per_node']} arrivals/s × {run['duration_seconds']} s.",
        "",
        "| Measurement | Result | Budget |",
        "|---|---:|---:|",
        f"| Accepted casts | {result['accepted_casts']}/{result['expected_casts']} | All |",
        f"| Cast p50 | {number(result['p50_ms'])} ms | ≤ {run['goals']['p50_ms']} ms |",
        f"| Cast p99 | {number(result['p99_ms'])} ms | ≤ {run['goals']['p99_ms']} ms |",
        f"| Accepted casts/s | {number(result['accepted_casts_per_second'])} | ≥ {run['goals']['min_casts_per_second']} |",
    ]
    if "journey" in result:
        for p in ("p50", "p99"):
            lines.append(
                f"| Journey {p} | {number(result['journey'][p + '_ms'])} ms | ≤ {run['goals']['journey_' + p + '_ms']} ms |"
            )
    lines += [
        "",
        f"Persistence verified: **{result['persistence_verified']}**. Failed casts: {result['failed_casts']}; missing attempts: {result['missing_attempts']}; scheduler drops: {result['scheduler_dropped_iterations']}.",
        "",
        f"Local {run['architecture']}, {run['logical_cpus']} logical CPUs. Generator and services share the machine. Tail percentiles describe this sample; they are not a production capacity estimate.",
        "",
    ]
    if "observed_http_requests" in result:
        lines += [
            f"Profile coverage: **{result['observed_http_requests']} HTTP requests**, {result['profile_requests_per_journey']} per journey. Request counts match the Chromium recipe: **{result['profile_request_count_matched']}**.",
            "",
        ]
    summary = "\n".join(lines)
    (output / "performance.md").write_text(
        summary + "\n![Performance](performance.svg)\n"
    )
    if publish:
        destination = ROOT / "docs/docusaurus/static/img/voting-load-performance.svg"
        shutil.copyfile(image, destination)
        Path(str(destination) + ".license").write_text(LICENSE)
        document = (
            ROOT
            / "docs/docusaurus/docs/07-developers/05-voting-portal/prepared-vote-load.md"
        )
        text = document.read_text()
        if MARKER not in text:
            raise ValueError("Missing current-results marker")
        document.write_text(
            text.split(MARKER)[0]
            + MARKER
            + "\n\n"
            + summary
            + "\n![Current voting performance](/img/voting-load-performance.svg)\n"
        )
