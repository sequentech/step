# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Render a backend E2E run's results as Markdown, e.g. for $GITHUB_STEP_SUMMARY.

    python3 scripts/e2e/summary.py <output directory>

Reads the driver's bootstrap.json and journeys.json and, after a coverage run,
coverage/summary.md. Prints the Markdown and saves it as summary.md.
"""

import json
import sys
from collections import Counter
from pathlib import Path


def cell(text, limit=160):
    text = " ".join(str(text).split())
    text = text if len(text) <= limit else text[: limit - 3] + "..."
    return text.replace("|", "\\|")


def render(output):
    lines = ["## Backend E2E journeys", ""]
    journeys = output / "journeys.json"
    if not journeys.is_file():
        lines.append(
            "No journey results: the run stopped before the journeys finished."
            " The job log and the uploaded service logs show why."
        )
    else:
        records = json.loads(journeys.read_text())
        lines += [
            "| Journey or check | Status | Duration | Detail |",
            "| --- | --- | ---: | --- |",
        ]
        bootstrap = output / "bootstrap.json"
        if bootstrap.is_file():
            seconds = max(json.loads(bootstrap.read_text())["seconds"].values())
            lines.append(f"| bootstrap | pass | {seconds:.1f} s | |")
        for record in records:
            status = record["outcome"]
            status = status if status == "pass" else f"**{status}**"
            detail = record["detail"].splitlines()[0] if record["detail"] else ""
            lines.append(
                f"| `{record['test']}` | {status} | {record['seconds']:.1f} s"
                f" | {cell(detail)} |"
            )
        outcomes = Counter(record["outcome"] for record in records)
        passed = outcomes["pass"] == len(records) > 0
        counts = ", ".join(f"{count} {outcome}" for outcome, count in outcomes.items())
        total = sum(record["seconds"] for record in records)
        lines += [
            "",
            f"**{'Passed' if passed else 'Failed'}:** {len(records)} journeys"
            f" ({counts or 'none run'}) in {total:.1f} s.",
        ]
    coverage = output / "coverage/summary.md"
    if coverage.is_file():
        lines += ["", coverage.read_text().strip()]
    return "\n".join(lines) + "\n"


def main():
    output = Path(sys.argv[1])
    text = render(output)
    print(text, end="")
    if output.is_dir():
        (output / "summary.md").write_text(text)


if __name__ == "__main__":
    main()
