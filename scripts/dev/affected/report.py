# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The selection as text for people and as versioned JSON for CI."""

from __future__ import annotations

from typing import Any

from .changes import Scope
from .config import Cost
from .model import Decision, Impact, Model, Selection, UnitImpact
from .workspaces import Unit

JSON_SCHEMA = 1
SHORT = 10


def short(commit: str | None) -> str:
    return commit[:SHORT] if commit else "unknown"


def chain_text(selection: Selection, impact: UnitImpact) -> str:
    if impact.impact is not Impact.DEPENDENCY:
        return impact.describe()
    origin = selection.units[impact.chain[0]]
    first = origin.files[0] if origin.files else impact.chain[0]
    return " -> ".join([first, *impact.chain])


def ordered(decisions: list[Decision]) -> list[Decision]:
    return sorted(
        decisions, key=lambda decision: (decision.check.cost.rank, decision.check.id)
    )


def cost_counts(decisions: list[Decision]) -> dict[str, int]:
    counts = {cost.value: 0 for cost in Cost}
    for decision in decisions:
        counts[decision.check.cost.value] += 1
    return counts


def describe_base(selection: Selection) -> str:
    changes = selection.changeset
    count = len(changes.changes)
    files = f"{count} file{'s' if count != 1 else ''}"
    if changes.scope is Scope.FILES:
        return f"{files}, as listed"
    what = (
        "committed"
        if changes.scope is Scope.COMMITTED
        else "in commits and the working tree"
    )
    source = changes.base_source.value
    return (
        f"{files} {what} since the merge base {short(changes.merge_base)} of HEAD "
        f"{short(changes.head)} and {changes.base_ref} ({source} base)"
    )


def text_report(selection: Selection) -> str:
    lines = [f"Changes: {describe_base(selection)}"]
    width = max((len(file.path) for file in selection.files), default=0)
    for file in selection.files:
        claim = file.claim
        target = (
            f"{', '.join(claim.units) or 'no unit'} ({claim.source})"
            if claim
            else "UNKNOWN"
        )
        extra = [f"input of {', '.join(file.inputs)}"] if file.inputs else []
        if file.test_inputs:
            extra.append(f"test input of {', '.join(file.test_inputs)}")
        suffix = f"; {'; '.join(extra)}" if extra else ""
        lines.append(f"  {file.status:<12} {file.path:<{width}}  -> {target}{suffix}")
    if selection.fallback:
        lines.append("")
        lines.append("Broad selection, every check is selected:")
        lines += [f"  {reason}" for reason in selection.fallback]
    lines.append("")
    lines.append(f"Affected units ({len(selection.units)}):")
    width = max((len(unit) for unit in selection.units), default=0)
    for unit, impact in selection.units.items():
        lines.append(f"  {unit:<{width}}  {chain_text(selection, impact)}")
    selected = ordered(selection.selected())
    counts = ", ".join(
        f"{cost} {count}" for cost, count in cost_counts(selected).items()
    )
    lines.append("")
    lines.append(f"Selected checks ({len(selected)}: {counts}):")
    width = max((len(decision.check.id) for decision in selected), default=0)
    for decision in selected:
        check = decision.check
        cwd = "" if check.cwd == "." else f"cd {check.cwd} && "
        lines.append(
            f"  {check.cost.value:<11} {check.id:<{width}}  {cwd}{check.command}"
        )
        shown = decision.reasons[:3]
        more = len(decision.reasons) - len(shown)
        reasons = "; ".join(shown) + (f"; and {more} more" if more > 0 else "")
        lines.append(f"  {'':<11} {'':<{width}}  because {reasons}")
    skipped = ordered(
        [decision for decision in selection.decisions if not decision.selected]
    )
    lines.append("")
    lines.append(f"Skipped checks ({len(skipped)}):")
    width = max((len(decision.check.id) for decision in skipped), default=0)
    for decision in skipped:
        lines.append(f"  {decision.check.id:<{width}}  {decision.reasons[0]}")
    return "\n".join(lines)


def graph_report(model: Model) -> str:
    lines = []
    width = max(len(unit) for unit in model.units)
    for unit in sorted(model.units.values(), key=lambda unit: unit.id):
        where = unit.path or unit.summary
        lines.append(f"{unit.id:<{width}}  {unit.kind.value:<5}  {where}")
        if unit.depends:
            lines.append(f"{'':<{width}}  depends on {', '.join(sorted(unit.depends))}")
        inputs = [glob.pattern for glob in unit.inputs]
        if inputs:
            lines.append(f"{'':<{width}}  inputs {', '.join(inputs)}")
        tests = [glob.pattern for glob in unit.test_inputs]
        if tests:
            lines.append(f"{'':<{width}}  test inputs {', '.join(tests)}")
    return "\n".join(lines)


def unit_entry(unit: Unit, impact: UnitImpact | None) -> dict[str, Any]:
    return {
        "id": unit.id,
        "kind": unit.kind.value,
        "path": unit.path,
        "workspace": unit.workspace,
        "summary": unit.summary,
        "depends": sorted(unit.depends),
        "inputs": [glob.pattern for glob in unit.inputs],
        "test_inputs": [glob.pattern for glob in unit.test_inputs],
        "affected": impact is not None,
        "impact": impact.impact.value if impact else None,
        "files": list(impact.files) if impact else [],
        "test_files": list(impact.test_files) if impact else [],
        "chain": list(impact.chain) if impact else [],
    }


def json_report(model: Model, selection: Selection) -> dict[str, Any]:
    changes = selection.changeset
    selected = selection.selected()
    return {
        "schema": JSON_SCHEMA,
        "base": {
            "scope": changes.scope.value,
            "ref": changes.base_ref,
            "source": changes.base_source.value,
            "commit": changes.base_commit,
            "merge_base": changes.merge_base,
            "head": changes.head,
        },
        "fallback": {
            "active": bool(selection.fallback),
            "reasons": list(selection.fallback),
        },
        "files": [
            {
                "path": file.path,
                "status": file.status,
                "claimed_by": file.claim.source if file.claim else None,
                "units": list(file.claim.units) if file.claim else [],
                "inputs": list(file.inputs),
                "test_inputs": list(file.test_inputs),
            }
            for file in selection.files
        ],
        "units": [
            unit_entry(unit, selection.units.get(unit.id))
            for unit in sorted(model.units.values(), key=lambda unit: unit.id)
        ],
        "checks": [
            {
                "id": decision.check.id,
                "owner": decision.check.owner,
                "kind": decision.check.kind.value,
                "runner": decision.check.runner.value,
                "cost": decision.check.cost.value,
                "trigger": decision.check.trigger.value,
                "cwd": decision.check.cwd,
                "command": decision.check.command,
                "env": dict(decision.check.env),
                "requires": [
                    requirement.value for requirement in decision.check.requires
                ],
                "paths": [glob.pattern for glob in decision.check.paths],
                "workflows": list(decision.check.workflows),
                "actions": list(decision.check.actions),
                "units": sorted(model.check_units[decision.check.id]),
                "selected": decision.selected,
                "reasons": list(decision.reasons),
            }
            for decision in selection.decisions
        ],
        "summary": {
            "files": len(selection.files),
            "affected_units": len(selection.units),
            "selected": cost_counts(selected),
            "skipped": len(selection.decisions) - len(selected),
        },
    }
