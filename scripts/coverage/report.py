# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Validate LLVM exports and calculate coverage for one package's source tree.

LLVM measures compiled code. A disabled module can be absent altogether, so a
percentage alone is insufficient: every source file must also be accounted for.
Scope exceptions explain unmeasured files. Explicit exclusions remove reviewed
test/support files and retain their counters separately for inspection.
"""

import re
from pathlib import Path
from typing import Any


class CoverageError(ValueError):
    """The report cannot support a coverage decision."""


METRICS = ("lines", "functions", "regions")


def read_counts(summary: dict[str, Any], metric: str) -> tuple[int, int]:
    """Read integer counters, ignoring LLVM's rounded percentage field."""
    counts = summary.get(metric)
    if not isinstance(counts, dict):
        raise CoverageError(f"Missing {metric} counters")

    covered, count = counts.get("covered"), counts.get("count")
    # bool is an int subclass in Python, but not a valid coverage counter.
    if type(covered) is not int or type(count) is not int:
        raise CoverageError(f"Non-integer {metric} counters")
    if not 0 <= covered <= count:
        raise CoverageError(f"Invalid {metric} counters: {covered}/{count}")
    return covered, count


def validate_exclusions(
    package: Path, excluded_files: dict[str, str], exceptions: dict[str, str]
) -> None:
    """Require exact source files and a reason; patterns cannot hide new code.

    Exclusions and inventory exceptions have different meanings. A file cannot
    simultaneously be omitted from the report and claimed to have no executable
    code. Renaming or deleting a file requires updating this reviewed policy.
    """
    if not isinstance(excluded_files, dict):
        raise CoverageError("Coverage exclusions must map exact files to reasons")
    inventory = {
        path.relative_to(package).as_posix() for path in (package / "src").rglob("*.rs")
    }
    for name, reason in excluded_files.items():
        if name not in inventory or not isinstance(reason, str) or not reason.strip():
            raise CoverageError(
                f"Coverage exclusion needs an existing source file and a reason: {name}"
            )
        if name in exceptions:
            raise CoverageError(
                f"File cannot be both excluded and a scope exception: {name}"
            )


def exclusion_arguments(package: Path, excluded_files: dict[str, str]) -> list[str]:
    """Apply the same exact filenames to LLVM JSON, HTML, LCOV and missing lines."""
    if not excluded_files:
        return []
    paths = [
        re.escape(str((package / name).resolve())) for name in sorted(excluded_files)
    ]
    # Anchor both ends: excluding fixtures.rs must not hide fixtures.rs.bak or
    # another package's file. LLVM accepts a POSIX extended regular expression.
    return ["--ignore-filename-regex", "^(" + "|".join(paths) + ")$"]


def summarize(
    payload: dict[str, Any],
    package: Path,
    minimum: int,
    exceptions: dict[str, str],
    excluded_files: dict[str, str] | None = None,
) -> dict[str, Any]:
    """Return per-file counters, scope gaps and an exact threshold decision.

    Paths in the export must refer to the current checkout. Results from other
    workspace packages are ignored; test targets outside ``src`` do not inflate
    the package denominator. Inline unit tests remain part of stable LLVM source
    coverage and are disclosed by the runner.
    """
    if type(minimum) is not int or not 0 <= minimum <= 100:
        raise CoverageError("The line threshold must be an integer from 0 to 100")
    if (
        not isinstance(payload, dict)
        or payload.get("type") != "llvm.coverage.json.export"
    ):
        raise CoverageError("Expected an LLVM coverage JSON export")
    # Rust 1.96 exports LLVM schema 3.1. Counter fields used here are also
    # present in schema 2; reject future major versions until reviewed.
    if str(payload.get("version", "")).split(".")[0] not in {"2", "3"}:
        raise CoverageError("Unsupported LLVM export schema version")

    data = payload.get("data")
    if not isinstance(data, list) or len(data) != 1 or not isinstance(data[0], dict):
        raise CoverageError("Expected one combined LLVM coverage export")
    entries = data[0].get("files")
    if not isinstance(entries, list):
        raise CoverageError("Missing LLVM file measurements")

    package = package.resolve()
    excluded_files = {} if excluded_files is None else excluded_files
    validate_exclusions(package, excluded_files, exceptions)
    source = package / "src"
    inventory = {path.relative_to(package).as_posix() for path in source.rglob("*.rs")}
    for name, reason in exceptions.items():
        if name not in inventory or not isinstance(reason, str) or not reason.strip():
            raise CoverageError(
                f"Scope exception needs an existing source file and a reason: {name}"
            )

    totals = {metric: {"covered": 0, "count": 0} for metric in METRICS}
    files: dict[str, Any] = {}
    excluded_measurements: dict[str, Any] = {}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("filename"), str):
            raise CoverageError("Invalid LLVM file record")
        path = Path(entry["filename"])
        if not path.is_absolute():
            raise CoverageError(
                "LLVM source paths must be absolute; do not remap report paths"
            )
        path = path.resolve()
        if not path.is_relative_to(source):
            continue

        name = path.relative_to(package).as_posix()
        if name not in inventory or name in files or name in excluded_measurements:
            raise CoverageError(f"Unknown or duplicate source file: {name}")
        summary = entry.get("summary")
        if not isinstance(summary, dict):
            raise CoverageError(f"Missing file summary: {name}")

        file_metrics = {}
        for metric in METRICS:
            covered, count = read_counts(summary, metric)
            file_metrics[metric] = {"covered": covered, "count": count}
        if name in excluded_files:
            excluded_measurements[name] = file_metrics
            continue
        for metric, counts in file_metrics.items():
            totals[metric]["covered"] += counts["covered"]
            totals[metric]["count"] += counts["count"]
        if name in exceptions and any(
            value["count"] > 0 for value in file_metrics.values()
        ):
            raise CoverageError(f"An exception cannot exclude measured code: {name}")
        files[name] = file_metrics

    if totals["lines"]["count"] == 0:
        raise CoverageError("No measured package source lines; coverage is unknown")

    metrics = {
        metric: {
            **counts,
            "percent": 100 * counts["covered"] / counts["count"]
            if counts["count"]
            else None,
        }
        for metric, counts in totals.items()
    }
    unaccounted = sorted(
        inventory - files.keys() - exceptions.keys() - excluded_files.keys()
    )
    failures = []
    if totals["lines"]["covered"] * 100 < minimum * totals["lines"]["count"]:
        failures.append(f"Measured line coverage is below {minimum}%")
    if unaccounted:
        failures.append(
            f"{len(unaccounted)} files need a measurement or reviewed explanation"
        )

    return {
        "metrics": metrics,
        "files": dict(sorted(files.items())),
        "source_files": sorted(inventory),
        "unaccounted_files": unaccounted,
        "scope_exceptions": exceptions,
        "excluded_files": dict(sorted(excluded_files.items())),
        "excluded_measurements": dict(sorted(excluded_measurements.items())),
        "minimum_lines": minimum,
        "passes": not failures,
        "failures": failures,
    }
