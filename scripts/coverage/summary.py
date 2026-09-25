# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Collect a workflow run's paired coverage verdicts into one Markdown table.

Each comparison job keeps its gate and its own step summary. This table only
repeats the fractions and statuses that their verdict.json files record: it
never recalculates a decision and never fails the run. Errors, initialized
scopes, unreadable verdicts and expected jobs without a verdict stay visible.
"""

import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

from ratchet import fraction
from report import CoverageError, read_counts

# Only identically named metrics share a column: LLVM regions are not branches.
ORDER = ("lines", "statements", "functions", "regions", "branches")
SIDES = ("base", "head")
ABSENT = "—"


def read_verdict(path: Path) -> dict[str, Any]:
    """Check only what the table repeats; ci.py has already decided."""
    verdict = json.loads(path.read_text())
    fields = ("scope", "status", "base_revision", "head_revision")
    if not isinstance(verdict, dict) or any(
        not isinstance(verdict.get(field), str) for field in fields
    ):
        raise CoverageError("Expected a scope, a status and both revisions")
    metrics = verdict.setdefault("metrics", {})
    failures = verdict.setdefault("failures", [])
    if (
        not isinstance(metrics, dict)
        or not isinstance(failures, list)
        or not all(isinstance(failure, str) for failure in failures)
        or not isinstance(verdict.get("note", ""), str)
    ):
        raise CoverageError("Invalid metrics, failures or note")
    for name, change in metrics.items():
        if not isinstance(change, dict) or type(change.get("decreased")) is not bool:
            raise CoverageError(f"Invalid {name} change")
        for side in SIDES:
            read_counts(change, side)
    return verdict


def collect(root: Path, expected: list[str]) -> list[dict[str, Any]]:
    """Sort rows by package, then artifact path, whatever the download order."""
    rows = []
    for path in root.rglob("verdict.json"):
        source = path.parent.relative_to(root).as_posix()
        row = {"scope": source, "source": source, "revisions": None, "metrics": {}}
        try:
            verdict = read_verdict(path)
        except (OSError, ValueError) as error:  # Includes CoverageError.
            rows.append({**row, "status": "unreadable", "details": [str(error)]})
            continue
        details = list(verdict["failures"])
        if verdict.get("note"):
            details.append(verdict["note"])
        rows.append(
            {
                **row,
                "scope": verdict["scope"],
                "status": verdict["status"],
                "revisions": (verdict["base_revision"], verdict["head_revision"]),
                "metrics": verdict["metrics"],
                "details": details,
            }
        )
    # A job that stopped before ci.py ran uploads no verdict, or nothing at all.
    found = {row["scope"] for row in rows}
    rows.extend(
        {
            "scope": scope,
            "source": "",
            "revisions": None,
            "metrics": {},
            "status": "missing",
            "details": ["No readable verdict.json was downloaded for this job."],
        }
        for scope in set(expected) - found
    )
    return sorted(rows, key=lambda row: (row["scope"], row["source"]))


def cell(text: str) -> str:
    """Keep recorded text on one line and inside its table cell."""
    return " ".join(text.split()).replace("|", "\\|")


def value(change: dict[str, Any] | None, side: str) -> str:
    if change is None:
        return ABSENT
    text = fraction(change[side])
    return f"{text} decreased" if side == "head" and change["decreased"] else text


def markdown(rows: list[dict[str, Any]]) -> str:
    """Render one table; a package measured by several runs names each path."""
    lines = [
        "## Coverage summary",
        "",
        "Each row repeats one comparison job's recorded verdict; those jobs decide CI.",
        "",
    ]
    pairs = sorted({row["revisions"] for row in rows if row["revisions"]})
    lines.extend(f"- Base `{cell(base)}` → head `{cell(head)}`" for base, head in pairs)
    if pairs:
        lines.append("")
    if not rows:
        lines.append("No paired coverage verdicts were found.")
        return "\n".join(lines) + "\n"

    names = sorted(
        {name for row in rows for name in row["metrics"]},
        key=lambda name: (ORDER.index(name) if name in ORDER else len(ORDER), name),
    )
    header = [f"{name.capitalize()} {side}" for name in names for side in SIDES]
    lines.extend(
        [
            "| " + " | ".join(["Package", *header, "Verdict"]) + " |",
            "| " + " | ".join(["---", *["---:"] * len(header), "---"]) + " |",
        ]
    )
    repeated = Counter(row["scope"] for row in rows)
    notes = []
    for row in rows:
        label = cell(row["scope"])
        if repeated[row["scope"]] > 1:
            label += f" (`{cell(row['source'])}`)"
        status = cell(row["status"].upper())
        values = [
            value(row["metrics"].get(name), side) for name in names for side in SIDES
        ]
        lines.append("| " + " | ".join([label, *values, status]) + " |")
        notes.extend(
            f"- {label} ({status}): {cell(detail)}" for detail in row["details"]
        )
    if notes:
        lines.extend(["", *notes])
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifacts", type=Path, help="Downloaded artifact directory")
    parser.add_argument(
        "--expect",
        nargs="+",
        action="extend",
        default=[],
        metavar="SCOPE",
        help="Package or profile whose job must have published a verdict",
    )
    args = parser.parse_args()
    sys.stdout.write(markdown(collect(args.artifacts, args.expect)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
