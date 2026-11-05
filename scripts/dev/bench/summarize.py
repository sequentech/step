# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Markdown tables of result files: median, range, sample count and conditions."""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from pathlib import Path
from typing import Any

from .results import read_result, summarize_samples

RANGE = "\N{EN DASH}"


def result_files(paths: Sequence[Path]) -> list[Path]:
    files: list[Path] = []
    for path in paths:
        if path.is_dir():
            files.extend(sorted(path.rglob("*.json")))
        else:
            files.append(path)
    return files


def seconds(value: float | None) -> str:
    if value is None:
        return "-"
    return f"{value:.1f}" if value >= 10 else f"{value:.2f}"


def value_range(summary: dict[str, Any]) -> str:
    if summary.get("min") is None:
        return "-"
    return f"{seconds(summary['min'])}{RANGE}{seconds(summary['max'])}"


def load(summary: dict[str, Any]) -> str:
    if summary.get("median") is None:
        return "-"
    return f"{summary['median']:.1f} ({summary['min']:.1f}{RANGE}{summary['max']:.1f})"


def row(cells: Iterable[str]) -> str:
    return "| " + " | ".join(cell.replace("|", "\\|") for cell in cells) + " |"


def conditions(document: dict[str, Any]) -> str:
    checkout = document.get("checkout") or {}
    commit = (checkout.get("commit") or "")[:10]
    dirty = " dirty" if checkout.get("dirty") else ""
    parts = [f"{commit}{dirty}".strip()] if commit else []
    parts.extend(document.get("parameters", {}).get("conditions", []))
    return "; ".join(parts)


def table(documents: Sequence[dict[str, Any]], with_phases: bool) -> str:
    header = [
        "Scenario",
        "Target",
        "Label",
        "Cache",
        "n",
        "Median (s)",
        "Range (s)",
        "Failed",
        "Load 1m median (range)",
        "Conditions",
    ]
    lines = [row(header), row(["---"] * len(header))]
    for document in documents:
        summary = summarize_samples(document.get("samples", []))
        lines.append(
            row(
                [
                    document["scenario"],
                    document["target"],
                    document["label"],
                    document["cache"],
                    str(summary["n"]),
                    seconds(summary["median"]),
                    value_range(summary),
                    str(summary["failed"]),
                    load(summary["load_1m"]),
                    conditions(document),
                ]
            )
        )
        if with_phases:
            for name, phase in summary["phases"].items():
                lines.append(
                    row(
                        [
                            "",
                            f"↳ {name}",
                            "",
                            "",
                            str(phase["n"]),
                            seconds(phase["median"]),
                            value_range(phase),
                            "",
                            "",
                            "",
                        ]
                    )
                )
    return "\n".join(lines)


def summarize(paths: Sequence[Path], with_phases: bool) -> str:
    documents = []
    for path in result_files(paths):
        try:
            documents.append(read_result(path))
        except ValueError:
            # Result directories also hold other JSON evidence, such as timings.
            continue
    documents.sort(key=lambda d: (d["scenario"], d["target"], d["label"], d["cache"]))
    return table(documents, with_phases)
