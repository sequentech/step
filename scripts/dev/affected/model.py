# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The unit graph, which unit owns a path, and which checks a change selects."""

from __future__ import annotations

import enum
import tomllib
from collections import deque
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path

from .changes import ChangeSet
from .config import (
    CONFIG_FILE,
    CheckSpec,
    Config,
    ConfigError,
    Trigger,
    WorkspaceKind,
    expand_checks,
    load_config,
)
from .workspaces import (
    FileRole,
    Unit,
    UnitKind,
    WorkspaceFile,
    cargo_lock_impact,
    discover,
)

WORKFLOWS_DIR = ".github/workflows"
ACTIONS_DIR = ".github/actions"
ALL_UNITS = "*"
# Reasons shown per check in text output; the JSON has all of them.
SHOWN_FILES = 1


class Impact(enum.Enum):
    """Why a unit is affected, strongest first."""

    CHANGED = "changed"
    # A file only its checks read changed; consumers are not affected.
    TEST_INPUT = "test-input"
    DEPENDENCY = "dependency"


@dataclass(frozen=True)
class Claim:
    units: tuple[str, ...]
    # What claimed the path, for the explanation.
    source: str
    broad: str | None = None


@dataclass
class FileImpact:
    path: str
    status: str
    claim: Claim | None
    # Units that list the file among their inputs or test inputs.
    inputs: tuple[str, ...] = ()
    test_inputs: tuple[str, ...] = ()


@dataclass
class UnitImpact:
    id: str
    impact: Impact
    # Its own files and inputs that changed, when the impact is CHANGED.
    files: list[str] = field(default_factory=list)
    # From a changed unit to this one along consumer edges.
    chain: list[str] = field(default_factory=list)
    # Changed files that only the unit's own checks read.
    test_files: list[str] = field(default_factory=list)

    def describe(self) -> str:
        if self.impact is Impact.DEPENDENCY:
            return f"{self.id} depends on {self.chain[0]} ({' -> '.join(self.chain)})"
        if self.impact is Impact.TEST_INPUT:
            return self.describe_tests()
        return f"{self.id} changed: {listing(self.files)}"

    def describe_tests(self) -> str:
        return f"{self.id} test input changed: {listing(self.test_files)}"


def listing(paths: list[str]) -> str:
    more = len(paths) - SHOWN_FILES
    return ", ".join(paths[:SHOWN_FILES]) + (f" and {more} more" if more > 0 else "")


@dataclass
class Decision:
    check: CheckSpec
    selected: bool
    reasons: list[str]


@dataclass
class Selection:
    changeset: ChangeSet
    files: list[FileImpact]
    units: dict[str, UnitImpact]
    decisions: list[Decision]
    fallback: list[str]

    def selected(self) -> list[Decision]:
        return [decision for decision in self.decisions if decision.selected]


class Model:
    """Units from the manifests plus the declared areas, edges, rules and checks."""

    def __init__(
        self,
        root: Path,
        config: Config,
        units: Iterable[Unit],
        workspace_files: Iterable[WorkspaceFile],
    ) -> None:
        self.root = root
        self.config = config
        self.units: dict[str, Unit] = {}
        for unit in units:
            if unit.id in self.units:
                raise ConfigError(f"two packages are called {unit.id}; set a prefix")
            self.units[unit.id] = unit
        self.workspace_files = {file.path: file for file in workspace_files}
        self._apply_unit_specs()
        self.rule_units = [
            tuple(sorted(self.resolve(rule.units, f"paths[{index}]")))
            for index, rule in enumerate(config.rules)
        ]
        self.checks = expand_checks(
            config.checks, {unit.id: unit.path for unit in self.units.values()}
        )
        self.check_units = {
            check.id: self.resolve(check.units, check.id) for check in self.checks
        }
        self.consumers: dict[str, set[str]] = {unit: set() for unit in self.units}
        for unit in self.units.values():
            for dependency in unit.depends:
                self.consumers[dependency].add(unit.id)
        self.owners = sorted(
            (unit for unit in self.units.values() if unit.path),
            key=lambda unit: len(str(unit.path)),
            reverse=True,
        )

    def _apply_unit_specs(self) -> None:
        for spec in self.config.units.values():
            unit = self.units.get(spec.id)
            if unit is None:
                if spec.summary is None:
                    raise ConfigError(
                        f"units.{spec.id}: no package has this name; an area needs "
                        "a summary"
                    )
                unit = Unit(id=spec.id, kind=UnitKind.AREA, name=spec.id)
                self.units[spec.id] = unit
            unit.summary = spec.summary or unit.summary
            unit.inputs += spec.inputs
            unit.test_inputs += spec.test_inputs
        for spec in self.config.units.values():
            where = f"units.{spec.id}"
            self.units[spec.id].depends |= self.resolve(spec.depends, where)
            for consumer in self.resolve(spec.consumers, where):
                self.units[consumer].depends.add(spec.id)

    def resolve(self, names: Iterable[str], where: str) -> set[str]:
        """Unit ids for ids and selectors.

        ``*`` is every unit, ``<kind>:*`` every unit of a kind and
        ``<kind>:<manifest>`` the packages of one workspace.
        """
        found: set[str] = set()
        for name in names:
            if name == ALL_UNITS:
                found |= set(self.units)
                continue
            kind, separator, scope = name.partition(":")
            if not separator:
                if name not in self.units:
                    raise ConfigError(f"{where}: unknown unit {name!r}")
                found.add(name)
                continue
            try:
                unit_kind = UnitKind(kind)
            except ValueError:
                raise ConfigError(f"{where}: unknown selector {name!r}") from None
            matched = {
                unit.id
                for unit in self.units.values()
                if unit.kind is unit_kind and scope in (ALL_UNITS, unit.workspace)
            }
            if not matched:
                raise ConfigError(f"{where}: selector {name!r} matches no unit")
            found |= matched
        return found

    def workspace_units(self, manifest: str) -> set[str]:
        return {unit.id for unit in self.units.values() if unit.workspace == manifest}

    def owner(self, path: str) -> Unit | None:
        return next(
            (unit for unit in self.owners if path.startswith(f"{unit.path}/")), None
        )

    def claim(self, path: str, changeset: ChangeSet | None = None) -> Claim | None:
        """The units a changed path belongs to, or None when nothing claims it."""
        file = self.workspace_files.get(path)
        if file is not None:
            return self._claim_workspace_file(file, changeset)
        for rule, units in zip(self.config.rules, self.rule_units, strict=True):
            glob = rule.match(path)
            if glob is not None:
                return Claim(units, f"rule {glob.pattern}", rule.broad)
        unit = self.owner(path)
        if unit is not None:
            return Claim((unit.id,), f"{unit.kind.value} package {unit.path}")
        return None

    def _claim_workspace_file(
        self, file: WorkspaceFile, changeset: ChangeSet | None
    ) -> Claim:
        units = self.workspace_units(file.workspace)
        everything = Claim(
            tuple(sorted(units)), f"{file.role.value} of {file.workspace}"
        )
        if file.role is FileRole.MANIFEST or file.kind is WorkspaceKind.YARN:
            # Hoisting lets any package load any locked module, so a Yarn lock
            # change cannot be narrowed by the declared dependencies.
            return everything
        old = changeset.old_text(file.path) if changeset else None
        new = changeset.new_text(file.path) if changeset else None
        if old is None or new is None:
            return Claim(
                everything.units, f"{everything.source}, no base version to compare"
            )
        try:
            names = cargo_lock_impact(old, new)
        except (tomllib.TOMLDecodeError, KeyError, TypeError):
            return Claim(everything.units, f"{everything.source}, unreadable")
        by_name = {self.units[unit].name: unit for unit in units}
        unknown = names - by_name.keys()
        if unknown:
            return Claim(
                everything.units,
                f"{everything.source}, unknown packages {', '.join(sorted(unknown))}",
            )
        return Claim(
            tuple(sorted(by_name[name] for name in names)),
            f"{file.role.value} of {file.workspace}: locked dependencies changed",
        )

    def input_units(self, path: str) -> tuple[tuple[str, ...], tuple[str, ...]]:
        inputs = sorted(
            unit.id
            for unit in self.units.values()
            if any(glob.matches(path) for glob in unit.inputs)
        )
        test_inputs = sorted(
            unit.id
            for unit in self.units.values()
            if any(glob.matches(path) for glob in unit.test_inputs)
        )
        return tuple(inputs), tuple(test_inputs)

    def select(self, changeset: ChangeSet) -> Selection:
        fallback = list(changeset.problems)
        files: list[FileImpact] = []
        changed: dict[str, list[str]] = {}
        tested: dict[str, list[str]] = {}
        for change in changeset.changes:
            claim = self.claim(change.path, changeset)
            inputs, test_inputs = self.input_units(change.path)
            files.append(
                FileImpact(change.path, change.status.value, claim, inputs, test_inputs)
            )
            if claim is None:
                fallback.append(f"no rule or package claims {change.path}")
            elif claim.broad:
                fallback.append(f"{claim.broad}: {change.path}")
            for unit in (*(claim.units if claim else ()), *inputs):
                changed.setdefault(unit, []).append(change.path)
            for unit in test_inputs:
                tested.setdefault(unit, []).append(change.path)
        units = self.closure(changed, tested)
        decisions = [
            self.decide(check, units, changeset, fallback) for check in self.checks
        ]
        return Selection(changeset, files, units, decisions, fallback)

    def closure(
        self, changed: dict[str, list[str]], tested: dict[str, list[str]]
    ) -> dict[str, UnitImpact]:
        """Changed units, then their consumers breadth first for the shortest chains."""
        impacts = {
            unit: UnitImpact(unit, Impact.CHANGED, sorted(set(paths)), [unit])
            for unit, paths in sorted(changed.items())
        }
        pending = deque(sorted(impacts))
        while pending:
            unit = pending.popleft()
            for consumer in sorted(self.consumers[unit]):
                if consumer not in impacts:
                    chain = [*impacts[unit].chain, consumer]
                    impacts[consumer] = UnitImpact(
                        consumer, Impact.DEPENDENCY, chain=chain
                    )
                    pending.append(consumer)
        for unit, paths in sorted(tested.items()):
            impact = impacts.setdefault(
                unit, UnitImpact(unit, Impact.TEST_INPUT, chain=[unit])
            )
            impact.test_files = sorted(set(paths))
        return dict(
            sorted(impacts.items(), key=lambda item: (len(item[1].chain), item[0]))
        )

    def decide(
        self,
        check: CheckSpec,
        units: dict[str, UnitImpact],
        changeset: ChangeSet,
        fallback: list[str],
    ) -> Decision:
        paths = [change.path for change in changeset.changes]
        reasons = []
        if fallback:
            reasons.append("broad selection")
        for glob in check.paths:
            path = next((path for path in paths if glob.matches(path)), None)
            if path is not None:
                reasons.append(f"{path} matches {glob.pattern}")
        for workflow in check.workflows:
            if f"{WORKFLOWS_DIR}/{workflow}" in paths:
                reasons.append(f"workflow {workflow} changed")
        for action in check.actions:
            if any(path.startswith(f"{ACTIONS_DIR}/{action}/") for path in paths):
                reasons.append(f"action {action} changed")
        covered = sorted(self.check_units[check.id])
        follows = (Impact.CHANGED, Impact.DEPENDENCY)
        if check.trigger is Trigger.CHANGED:
            follows = (Impact.CHANGED,)
        for unit in covered:
            impact = units.get(unit)
            if impact is None:
                continue
            if impact.impact in follows:
                reasons.append(impact.describe())
            # Test inputs are read by the unit's own tests, not by its consumers.
            if (
                impact.test_files
                and check.owner == unit
                and check.trigger is Trigger.AFFECTED
            ):
                reasons.append(impact.describe_tests())
        if reasons:
            return Decision(check, True, reasons)
        return Decision(check, False, [skip_reason(check, covered, units, paths)])


def skip_reason(
    check: CheckSpec, covered: list[str], units: dict[str, UnitImpact], paths: list[str]
) -> str:
    if not paths:
        return "no changes"
    through = [unit for unit in covered if unit in units]
    if through and check.trigger is Trigger.CHANGED:
        return (
            f"{', '.join(through)} affected only through dependencies; the check "
            "reads the units' own files"
        )
    if through:
        return (
            f"only test inputs of {', '.join(through)} changed, which this check "
            "does not read"
        )
    if not covered:
        return "no changed path matches"
    shown = ", ".join(covered[:3]) + (
        f" and {len(covered) - 3} more" if len(covered) > 3 else ""
    )
    return f"{shown} not affected"


def load_model(root: Path, config_path: Path | None = None) -> Model:
    config = load_config(config_path or root / CONFIG_FILE)
    units, files = discover(root, config.workspaces)
    return Model(root, config, units, files)
