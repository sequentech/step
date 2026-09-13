# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Compare exact coverage fractions; improvement targets do not decide CI.

A package cannot trade fewer covered branches for more covered lines. Compare
metrics independently, without a rounding tolerance or a cross-package average.
The caller measures both revisions in the same run with the same instrumentation.
"""

from typing import Any

from report import CoverageError, read_counts


def python_metrics(report: dict[str, Any]) -> dict[str, Any]:
    """Keep Python line and branch coverage separate, unlike its combined score."""
    if not report.get("meta", {}).get("branch_coverage") or not report.get("files"):
        raise CoverageError("Expected a nonempty Python branch-coverage report")
    totals = report["totals"]
    return {
        "lines": {
            "covered": totals["covered_lines"],
            "count": totals["num_statements"],
        },
        "branches": {
            "covered": totals["covered_branches"],
            "count": totals["num_branches"],
        },
    }


def rust_metrics(report: dict[str, Any]) -> dict[str, Any]:
    """An unmet local 95% target is still a usable, completed measurement."""
    if report.get("status") != "measured" or report.get("tests_passed", 0) <= 0:
        raise CoverageError("Rust coverage did not complete with passing tests")
    if set(report.get("metrics", {})) != {"lines", "functions", "regions"}:
        raise CoverageError("Rust coverage must report lines, functions and regions")
    return report["metrics"]


def compare(base: dict[str, Any], head: dict[str, Any]) -> dict[str, Any]:
    """Return a verdict for matching metrics; malformed measurements raise.

    A zero denominator means that a metric has no opportunities (for example,
    no branches). Treat that as fully covered. Losing an entire line inventory
    is different: there is no valid package measurement to compare.
    """
    if "lines" not in base or base.keys() != head.keys():
        raise CoverageError("Base and head must measure the same metrics")

    failures = []
    changes = {}
    for metric in base:
        old_covered, old_count = read_counts(base, metric)
        new_covered, new_count = read_counts(head, metric)
        if metric == "lines" and (old_count == 0 or new_count == 0):
            raise CoverageError("Empty line coverage is unknown, not a passing result")

        old_fraction = (old_covered, old_count) if old_count else (1, 1)
        new_fraction = (new_covered, new_count) if new_count else (1, 1)
        decreased = (
            new_fraction[0] * old_fraction[1] < old_fraction[0] * new_fraction[1]
        )
        changes[metric] = {
            "base": {"covered": old_covered, "count": old_count},
            "head": {"covered": new_covered, "count": new_count},
            "decreased": decreased,
        }
        if decreased:
            failures.append(
                f"{metric} coverage decreased: "
                f"{old_covered}/{old_count} → {new_covered}/{new_count}"
            )

    return {"passes": not failures, "failures": failures, "metrics": changes}


def compare_rust(base: dict[str, Any], head: dict[str, Any]) -> dict[str, Any]:
    """Do not let a feature change or newly omitted file look like improvement."""
    for field in ("profile", "features", "tools", "config_sha256"):
        if field not in base or base[field] != head.get(field):
            raise CoverageError(f"Incompatible Rust measurements: {field}")
    for field in ("excluded_files", "scope_exceptions"):
        if base.get(field, {}) != head.get(field, {}):
            raise CoverageError(f"Incompatible Rust measurements: {field}")
    verdict = compare(rust_metrics(base), rust_metrics(head))
    # Existing scope gaps remain disclosed. Opening another one fails CI even
    # if excluding that source makes the measured percentage increase.
    for report in (base, head):
        if not isinstance(report.get("unaccounted_files"), list):
            raise CoverageError("Missing Rust source inventory")
    missing = sorted(set(head["unaccounted_files"]) - set(base["unaccounted_files"]))
    if missing:
        verdict["passes"] = False
        verdict["failures"].append("New unmeasured source files: " + ", ".join(missing))
    return verdict


def markdown(result: dict[str, Any]) -> str:
    """Explain the actual merge decision and retain both commit identities."""
    status = result["status"].upper()
    lines = [
        f"## {result['scope']}: {status}",
        "",
        f"Base: `{result['base_revision']}`  ",
        f"Head: `{result['head_revision']}`",
        "",
        "CI requires no coverage decrease. 95% remains an improvement target.",
        "",
    ]
    if result.get("metrics"):
        lines.extend(
            [
                "| Metric | Base | Head | Result |",
                "| --- | ---: | ---: | --- |",
            ]
        )
        for name, change in result["metrics"].items():
            values = []
            for side in ("base", "head"):
                covered, count = change[side]["covered"], change[side]["count"]
                percent = f"{100 * covered / count:.4f}%" if count else "n/a"
                values.append(f"{covered}/{count} ({percent})")
            decision = "decreased" if change["decreased"] else "maintained / increased"
            lines.append(f"| {name} | {' | '.join(values)} | {decision} |")
    lines.extend(f"- {failure}" for failure in result.get("failures", []))
    if result.get("note"):
        lines.extend(["", result["note"]])
    return "\n".join(lines) + "\n"
