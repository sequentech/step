# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Focused tests: ``step-dev test <package | check | file | story> [name]``.

Runs the narrowest existing command for a package, a test or source file, a
story, a Playwright spec or title, a Rust crate or test target, or the Python
tooling, from the checks in scripts/dev/affected.toml. --affected runs the fast
checks that the current changes select; step-dev validate runs the broader ones.
The scope is printed before anything runs. Coverage and service stacks never
start unless a selected check needs them.
"""

from __future__ import annotations

import argparse
import difflib
import enum
import hashlib
import json
import os
import re
import shlex
import shutil
import signal
import subprocess
import sys
import time
import tomllib
from collections import deque
from collections.abc import Iterator, Sequence
from dataclasses import dataclass, field, replace
from pathlib import Path, PurePosixPath
from typing import NoReturn

from .affected.changes import GitError, Scope
from .affected.cli import add_change_options, caller_cwd, changes_for
from .affected.config import (
    CheckKind,
    CheckSpec,
    ConfigError,
    Cost,
    Requirement,
    Runner,
    WorkspaceKind,
)
from .affected.model import Model, load_model
from .affected.workspaces import Unit, UnitKind

ROOT = Path(__file__).resolve().parents[2]
POLL_SECONDS = 0.5
# Claims a directory as if it held a file of this name.
DIRECTORY_PROBE = ".step-dev-directory"
STORY_FILE = re.compile(r"\.stories\.[cm]?[jt]sx?$")
SCRIPT_TEST_FILE = re.compile(r"\.(test|spec)\.[cm]?[jt]sx?$")
SCRIPT_FILE = re.compile(r"\.[cm]?[jt]sx?$")
STORYBOOK_PORT = re.compile(r"storybook dev\b.*?-p\s+(\d+)")
STORYBOOK_URL = re.compile(r"https?://[^/:]+:(\d+)")
CRATE_CFG = re.compile(r"#!\[cfg\((.*?)\)\]", re.S)
CFG_FEATURE = re.compile(r'feature\s*=\s*"([^"]+)"')
ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")
# Build output and installed dependencies never trigger a watched rerun.
UNWATCHED = frozenset(
    {"node_modules", "target", "dist", "test-results", "coverage", "storybook-static"}
)
TARGET_DIR_NAME = re.compile(r"rust-.*-target")
CHROMIUM_NAMES = ("chromium", "chromium-browser", "google-chrome")
CHROMIUM_ENV = ("CHROMIUM_EXECUTABLE_PATH", "WORKBENCH_TEST_CHROME_PATH")
PLAYWRIGHT_BROWSERS = Path.home() / ".cache" / "ms-playwright"


class Depth(enum.Enum):
    """How far a package or ``--affected`` run goes beyond the fast checks."""

    FAST = "fast"
    BROAD = "broad"
    FULL = "full"

    def allows(self, cost: Cost) -> bool:
        return cost.rank <= list(Depth).index(self)

    def __str__(self) -> str:
        return self.value


# The depth just below each, to name the checks a depth adds.
DEPTH_BELOW = {Depth.BROAD: Depth.FAST, Depth.FULL: Depth.BROAD}


class Outcome(enum.Enum):
    PASSED = "passed"
    FAILED = "failed"
    # Exited successfully without running a single test.
    EMPTY = "no tests ran"
    UNAVAILABLE = "unavailable"


class SelectionError(Exception):
    """The target names nothing runnable; the message says what would be."""


@dataclass
class Step:
    """One command of a focused run, from ``cwd`` relative to the repository."""

    check: CheckSpec
    what: str
    cwd: str
    # An argument list for a narrowed run, or the check's own shell command.
    command: list[str] | str
    env: dict[str, str] = field(default_factory=dict)
    # Built first when a file the run needs is missing.
    prepare: str | None = None
    needs: tuple[str, ...] = ()
    # Narrowed runs fail when their filter selects no test at all.
    narrowed: bool = False
    # The command watches by itself; otherwise --watch polls these paths.
    watches: bool = False
    watch_paths: list[str] = field(default_factory=list)
    reason: str | None = None

    def shell(self) -> str:
        if isinstance(self.command, str):
            return self.command
        return shlex.join(self.command)

    def display(self) -> str:
        prefix = "" if self.cwd == "." else f"cd {self.cwd} && "
        return prefix + self.shell()


@dataclass
class Plan:
    heading: str
    unit: Unit | None = None
    steps: list[Step] = field(default_factory=list)
    not_run: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class Narrowing:
    """What a focused run selects inside a check."""

    files: tuple[str, ...] = ()
    related: bool = False
    story: str | None = None
    test_target: str | None = None
    features: tuple[str, ...] = ()

    def __bool__(self) -> bool:
        return bool(self.files or self.story or self.test_target)


NOTHING = Narrowing()


def relative_to(path: str, directory: str) -> str:
    """A repository path as seen from another repository directory."""
    return PurePosixPath(os.path.relpath(path, directory)).as_posix()


def owned_checks(model: Model, unit: str) -> list[CheckSpec]:
    """Checks a unit owns, including those that cover a whole workspace equally."""
    return by_cost(
        [
            check
            for check in model.checks
            if check.owner == unit
            or (check.owner is None and unit in model.check_units[check.id])
        ]
    )


def by_cost(checks: Sequence[CheckSpec]) -> list[CheckSpec]:
    return sorted(checks, key=lambda check: (check.cost.rank, check.id))


def static_prefix(pattern: str) -> str:
    """The leading path of a glob before its first wildcard component."""
    parts = []
    for part in pattern.split("/"):
        if "*" in part or "?" in part:
            break
        parts.append(part)
    return "/".join(parts)


def dependency_dirs(model: Model, unit_ids: Sequence[str]) -> list[str]:
    """Directories of the units and of everything they depend on."""
    seen: set[str] = set()
    pending = deque(unit_ids)
    while pending:
        unit = pending.popleft()
        if unit not in seen:
            seen.add(unit)
            pending.extend(sorted(model.units[unit].depends))
    directories = set()
    for unit in seen:
        if model.units[unit].path:
            directories.add(str(model.units[unit].path))
            continue
        for rule, units in zip(model.config.rules, model.rule_units, strict=True):
            if unit in units:
                directories.update(
                    filter(None, (static_prefix(glob.pattern) for glob in rule.globs))
                )
    return sorted(directories)


def target_dir(model: Model, check: CheckSpec) -> Path | None:
    """The per-checkout Cargo target directory of the workspace a check runs in."""
    cargo = [
        spec
        for spec in model.config.workspaces
        if spec.kind is WorkspaceKind.CARGO and spec.target_dir
    ]
    for spec in cargo:
        if PurePosixPath(spec.manifest).parent.as_posix() == check.cwd:
            return model.root / str(spec.target_dir)
    return model.root / str(cargo[0].target_dir) if cargo else None


def cfg_features(source: str) -> tuple[list[str], bool]:
    """Features a test file's crate-level ``#![cfg(...)]`` requires.

    The second value is False when the condition is not a plain conjunction of
    features, so no feature list is certain to enable the file.
    """
    match = CRATE_CFG.search(source)
    if match is None:
        return [], True
    features = CFG_FEATURE.findall(condition := match.group(1))
    plain = not features or not re.search(r"\b(any|not)\s*\(", condition)
    return features, plain


def test_target_features(
    root: Path, unit: Unit, name: str
) -> tuple[list[str], str | None]:
    """Features a Cargo test target needs to compile its tests, and any caveat."""
    manifest = tomllib.loads((root / str(unit.path) / "Cargo.toml").read_text())
    features: list[str] = []
    for target in manifest.get("test", []):
        if target.get("name") == name:
            features += target.get("required-features", [])
    source = root / str(unit.path) / "tests" / f"{name}.rs"
    found, plain = cfg_features(source.read_text(errors="replace"))
    features += found
    caveat = None
    if not plain:
        caveat = (
            f"tests/{name}.rs is enabled by a cfg condition with any() or not(); "
            "pass the features it needs after --"
        )
    return list(dict.fromkeys(features)), caveat


def whole(model: Model, check: CheckSpec, extra: Sequence[str] = ()) -> Step:
    command = check.command + "".join(f" {shlex.quote(item)}" for item in extra)
    return Step(
        check=check,
        what=check.id,
        cwd=check.cwd,
        command=command,
        env=dict(check.env),
        watch_paths=dependency_dirs(model, sorted(model.check_units[check.id])),
    )


def python_command(root: Path, check: CheckSpec, files: Sequence[str]) -> list[str]:
    """Test modules of Python packages run by dotted name, flat directories by file."""
    command = list(check.focus or ("python3", "-m", "unittest"))
    for file in files:
        path = PurePosixPath(file)
        if (root / path.parent / "__init__.py").is_file():
            command.append(".".join(path.with_suffix("").parts))
        else:
            command += ["discover", "-s", path.parent.as_posix(), "-p", path.name]
    return command


def narrow(
    model: Model,
    check: CheckSpec,
    what: str,
    narrowing: Narrowing,
    name: str | None,
    watch: bool,
    extra: Sequence[str],
) -> Step:
    """The check's focus command, narrowed with the runner's own options.

    Files are repository paths, passed relative to the check's directory, where
    the command runs.
    """
    if check.focus is None:
        raise SelectionError(f"{check.id} runs as a whole: step-dev test {check.id}")
    local = [relative_to(file, check.cwd) for file in narrowing.files]
    command = list(check.focus)
    runner = check.runner
    watches = False
    extra = list(extra)
    if runner is Runner.JEST:
        command += ["--findRelatedTests"] if narrowing.related else []
        command += local + (["-t", name] if name else [])
        command += ["--watch"] if watch else []
        watches = watch
    elif runner is Runner.VITEST:
        if narrowing.related:
            command += ["related", *local, "--watch" if watch else "--run"]
        else:
            command += ["watch" if watch else "run", *local]
        command += ["-t", name] if name else []
        watches = watch
    elif runner is Runner.STORYBOOK:
        command += [narrowing.story] if narrowing.story else local
        command += [f"--testNamePattern={name}"] if name else []
        command += ["--watch"] if watch else []
        watches = watch
    elif runner is Runner.PLAYWRIGHT:
        command += local + (["-g", name] if name else [])
    elif runner is Runner.CARGO:
        command += ["--test", narrowing.test_target] if narrowing.test_target else []
        if narrowing.features:
            if "--features" in command:
                index = command.index("--features") + 1
                listed = [*command[index].split(","), *narrowing.features]
                command[index] = ",".join(dict.fromkeys(listed))
            else:
                command += ["--features", ",".join(narrowing.features)]
        # Cargo options go before the test binary's filter.
        command += extra + (["--", name] if name else [])
        extra = []
    elif runner is Runner.PYTHON:
        command = python_command(model.root, check, narrowing.files)
        command += ["-k", name] if name else []
    elif runner is Runner.NODE:
        command += [f"--test-name-pattern={name}"] if name else []
        command += local
    else:
        raise SelectionError(f"{check.id} cannot be narrowed; run it as a whole")
    watched = dependency_dirs(model, sorted(model.check_units[check.id]))
    return Step(
        check=check,
        what=what,
        cwd=check.cwd,
        command=command + extra,
        env=dict(check.env),
        prepare=check.prepare,
        needs=check.needs,
        narrowed=bool(name or narrowing),
        watches=watches,
        watch_paths=sorted({*watched, *narrowing.files}),
    )


class Resolver:
    """Turns a target into the steps of a focused run."""

    def __init__(self, model: Model, arguments: argparse.Namespace) -> None:
        self.model = model
        self.arguments = arguments
        self.depth: Depth = arguments.depth
        self.name: str | None = arguments.name
        self.watch: bool = arguments.watch
        self.extra: list[str] = arguments.extra

    def step(self, check: CheckSpec, what: str, narrowing: Narrowing = NOTHING) -> Step:
        """A narrowed step when something narrows the check, else the whole check.

        Jest and Vitest watch by themselves; other runners are rerun by --watch.
        """
        native = self.watch and check.runner in (Runner.JEST, Runner.VITEST)
        if narrowing or self.name or (native and check.focus is not None):
            return narrow(
                self.model, check, what, narrowing, self.name, self.watch, self.extra
            )
        return whole(self.model, check, self.extra)

    def plan(self) -> Plan:
        if self.arguments.affected:
            return self.affected()
        target = self.arguments.target
        if target is None:
            raise SelectionError("name a package, check, file or story, or --affected")
        if self.arguments.story:
            return self.story(target, self.arguments.story)
        if STORYBOOK_URL.match(target):
            return self.story(None, target)
        if any(check.id == target for check in self.model.checks):
            return self.check(target)
        if target in self.model.units:
            return self.unit(target)
        path = self.find_path(target)
        if path is not None:
            return self.path(path)
        choices = [*self.model.units, *(check.id for check in self.model.checks)]
        close = difflib.get_close_matches(target, choices, n=5)
        hint = f"; did you mean {', '.join(close)}?" if close else ""
        raise SelectionError(f"no package, check, file or story matches {target}{hint}")

    def find_path(self, target: str) -> str | None:
        for base in (caller_cwd(), self.model.root):
            candidate = Path(os.path.normpath(base / target))
            if candidate.exists():
                relative = PurePosixPath(os.path.relpath(candidate, self.model.root))
                if relative.parts[:1] == ("..",):
                    raise SelectionError(f"{target} is outside the repository")
                return relative.as_posix()
        return None

    def not_run(self, plan: Plan, unit: str) -> None:
        """Mention the unit's other tests and its consumers, and how to run them."""
        ran = {step.check.id for step in plan.steps}
        for depth in Depth:
            others = [
                check.id
                for check in owned_checks(self.model, unit)
                if check.kind is CheckKind.TEST
                and check.id not in ran
                and depth.allows(check.cost)
                and (depth is Depth.FAST or not DEPTH_BELOW[depth].allows(check.cost))
            ]
            if others:
                option = "" if depth is Depth.FAST else f" --depth {depth.value}"
                plan.not_run.append(
                    f"{', '.join(others)}: step-dev test {unit}{option}"
                )
        consumers = sorted(self.model.consumers[unit])
        if consumers:
            plan.not_run.append(
                f"consumers {', '.join(consumers)}: step-dev test --affected"
            )

    def unit(self, unit_id: str) -> Plan:
        plan = Plan(f"package {unit_id}", self.model.units[unit_id])
        checks = [
            check
            for check in owned_checks(self.model, unit_id)
            if check.kind is CheckKind.TEST and self.depth.allows(check.cost)
        ]
        if not checks:
            owned = ", ".join(check.id for check in owned_checks(self.model, unit_id))
            raise SelectionError(
                f"{unit_id} has no {self.depth.value} tests"
                + (f"; its checks: {owned}" if owned else "")
            )
        plan.steps += [self.step(check, check.id) for check in checks]
        self.not_run(plan, unit_id)
        return plan

    def check(self, check_id: str) -> Plan:
        check = next(check for check in self.model.checks if check.id == check_id)
        return Plan(f"check {check_id}", steps=[self.step(check, check.id)])

    def storybook(self, unit: str) -> CheckSpec | None:
        """The Storybook tests that collect a unit's stories, its own first."""
        checks = [
            check
            for check in self.model.checks
            if check.runner is Runner.STORYBOOK
            and unit in self.model.check_units[check.id]
        ]
        return min(checks, key=lambda check: check.owner != unit, default=None)

    def story(self, unit_id: str | None, story: str) -> Plan:
        unit_id = unit_id or self.storybook_unit(story)
        if unit_id not in self.model.units:
            raise SelectionError(f"unknown package {unit_id}")
        check = self.storybook(unit_id)
        if check is None:
            raise SelectionError(f"{unit_id} has no Storybook tests")
        plan = Plan(f"story {story}", self.model.units[unit_id])
        plan.steps.append(
            self.step(check, f"{check.id}: story {story}", Narrowing(story=story))
        )
        return plan

    def storybook_unit(self, url: str) -> str:
        match = STORYBOOK_URL.match(url)
        if match is None:
            raise SelectionError(
                "name the story's package: step-dev test <package> --story <id>"
            )
        for unit in self.model.units.values():
            if unit.kind is UnitKind.YARN:
                manifest = self.model.root / str(unit.path) / "package.json"
                script = json.loads(manifest.read_text()).get("scripts", {})
                port = STORYBOOK_PORT.search(script.get("storybook", ""))
                if port and port.group(1) == match.group(1):
                    return unit.id
        raise SelectionError(
            f"no package serves its Storybook on port {match.group(1)}"
        )

    def path(self, path: str) -> Plan:
        directory = (self.model.root / path).is_dir()
        claim = self.model.claim(f"{path}/{DIRECTORY_PROBE}" if directory else path)
        if claim is None or not claim.units:
            raise SelectionError(
                f"no package or area claims {path}; step-dev test --affected "
                f"--files {path} runs every check"
            )
        units = [self.model.units[unit] for unit in claim.units]
        unit = next(
            (unit for unit in units if unit.kind is not UnitKind.AREA), units[0]
        )
        if directory:
            return self.directory(unit, path)
        plan = Plan(path, unit)
        owned = owned_checks(self.model, unit.id)
        specs = [
            check
            for check in self.model.checks
            if any(glob.matches(path) for glob in check.tests)
        ]
        if STORY_FILE.search(path):
            check = self.storybook(unit.id)
            if check is None:
                raise SelectionError(f"{unit.id} has no Storybook tests for {path}")
            what = f"{check.id}: stories in {path}"
            plan.steps.append(self.step(check, what, Narrowing(files=(path,))))
        elif specs:
            for check in specs:
                what = f"{check.id}: {path}"
                plan.steps.append(self.step(check, what, Narrowing(files=(path,))))
        elif path.endswith(".rs"):
            self.rust_file(unit, path, plan, owned)
        elif SCRIPT_FILE.search(path) and first(owned, Runner.JEST, Runner.VITEST):
            check = first(owned, Runner.JEST, Runner.VITEST)
            assert check is not None
            test_file = bool(SCRIPT_TEST_FILE.search(path))
            what = (
                f"{check.id}: {'tests in' if test_file else 'tests related to'} {path}"
            )
            narrowing = Narrowing(files=(path,), related=not test_file)
            plan.steps.append(self.step(check, what, narrowing))
        elif path.endswith(".py") and first(owned, Runner.PYTHON):
            check = first(owned, Runner.PYTHON)
            assert check is not None
            tests = python_tests(self.model.root, path, check)
            if tests:
                what = f"{check.id}: {', '.join(tests)}"
                plan.steps.append(self.step(check, what, Narrowing(files=tuple(tests))))
            else:
                plan.steps.append(whole(self.model, check, self.extra))
                plan.notes.append(f"no test module is named after {path}")
        elif any(check.kind is CheckKind.TEST for check in owned):
            return self.unit(unit.id)
        else:
            others = ", ".join(check.id for check in owned)
            raise SelectionError(
                f"{path} belongs to {unit.id}, which has no tests"
                + (f"; run one of its checks: {others}" if others else "")
                + f"; step-dev test --affected --files {path} runs what it selects"
            )
        self.not_run(plan, unit.id)
        return plan

    def rust_file(
        self, unit: Unit, path: str, plan: Plan, owned: list[CheckSpec]
    ) -> None:
        check = first(owned, Runner.CARGO)
        if check is None:
            raise SelectionError(f"{unit.id} has no Cargo tests in the model")
        local = PurePosixPath(relative_to(path, str(unit.path)))
        if local.parent == PurePosixPath("tests"):
            features, caveat = test_target_features(self.model.root, unit, local.stem)
            if caveat:
                plan.notes.append(caveat)
            what = f"{check.id}: test target {local.stem}"
            narrowing = Narrowing(test_target=local.stem, features=tuple(features))
            plan.steps.append(self.step(check, what, narrowing))
        elif self.name:
            plan.steps.append(self.step(check, f"{check.id} matching {self.name!r}"))
        else:
            # Any test of the crate may exercise a source file, so none is skipped.
            plan.steps.append(whole(self.model, check, self.extra))
            plan.notes.append(f"{path} is crate source: every test of {check.id} runs")

    def directory(self, unit: Unit, path: str) -> Plan:
        check = first(owned_checks(self.model, unit.id), Runner.JEST)
        if unit.path == path or unit.kind is UnitKind.AREA or check is None:
            return self.unit(unit.id)
        plan = Plan(path, unit)
        what = f"{check.id}: tests under {path}"
        plan.steps.append(self.step(check, what, Narrowing(files=(path,))))
        self.not_run(plan, unit.id)
        return plan

    def affected(self) -> Plan:
        if self.arguments.target:
            raise SelectionError("--affected takes no target")
        if self.watch:
            raise SelectionError("--watch needs a target")
        selection = self.model.select(changes_for(self.model, self.arguments))
        changes = selection.changeset
        count = len(changes.changes)
        heading = (
            f"{count} changed file{'s' if count != 1 else ''} ({changes.scope.value}"
        )
        heading += (
            f" since the merge base with {changes.base_ref})"
            if changes.base_ref
            else ")"
        )
        plan = Plan(f"checks affected by {heading}")
        plan.notes += [f"broad selection: {reason}" for reason in selection.fallback]
        for decision in selection.selected():
            check = decision.check
            if self.depth.allows(check.cost):
                step = whole(self.model, check, self.extra)
                step.reason = decision.reasons[0]
                plan.steps.append(step)
            else:
                plan.not_run.append(f"{check.id} ({check.cost.value}): {check.command}")
        plan.steps.sort(key=lambda step: (step.check.cost.rank, step.check.id))
        if not plan.steps and not plan.not_run:
            plan.notes.append("the changes select no check")
        return plan


def first(checks: Sequence[CheckSpec], *runners: Runner) -> CheckSpec | None:
    """The cheapest test check of the given runners."""
    return next(
        (
            check
            for check in by_cost(checks)
            if check.runner in runners and check.kind is CheckKind.TEST
        ),
        None,
    )


def python_tests(root: Path, path: str, check: CheckSpec) -> list[str]:
    """The file itself if it is a test, else ``test_<name>.py`` up its package."""
    file = PurePosixPath(path)
    if file.name.startswith("test_"):
        return [path]
    name = file.stem
    for directory in [file.parent, *file.parent.parents]:
        candidate = (directory / f"test_{name}.py").as_posix()
        inside = any(glob.matches(candidate) for glob in check.tests)
        if not inside:
            break
        if (root / candidate).is_file():
            return [candidate]
        name = directory.name
    return []


# Running ---------------------------------------------------------------------------


def missing(model: Model, step: Step) -> str | None:
    """Why a step cannot run here, or None."""
    for requirement in step.check.requires:
        if requirement is Requirement.CHROMIUM:
            configured = any(
                os.environ.get(name) and Path(os.environ[name]).exists()
                for name in CHROMIUM_ENV
            )
            installed = any(shutil.which(name) for name in CHROMIUM_NAMES)
            if not (configured or installed or PLAYWRIGHT_BROWSERS.is_dir()):
                return "needs Chromium: set CHROMIUM_EXECUTABLE_PATH"
        elif requirement is not Requirement.POSTGRES_SERVER and not shutil.which(
            requirement.value
        ):
            return f"needs {requirement.value}"
    try:
        program = shlex.split(step.shell())[0]
    except (ValueError, IndexError):
        return None
    if "/" in program:
        found = (model.root / step.cwd / program).exists()
        return None if found else f"{program} not found"
    if shutil.which(program) is None:
        return f"{program} not found; run inside the devenv shell (devenv shell)"
    return None


def environment(model: Model, step: Step) -> dict[str, str]:
    env = dict(os.environ)
    env.update(step.env)
    if step.check.runner is Runner.CARGO:
        current = env.get("CARGO_TARGET_DIR")
        directory = target_dir(model, step.check)
        # A relative target directory would follow the command's directory.
        if directory is not None and not (current and Path(current).is_absolute()):
            env["CARGO_TARGET_DIR"] = str(directory)
    return env


def tests_ran(runner: Runner, output: str) -> bool:
    """Whether a runner's output reports at least one executed test."""
    text = ANSI.sub("", output)
    if runner is Runner.CARGO:
        counts = re.findall(r"test result: \w+\. (\d+) passed; (\d+) failed", text)
        return any(int(passed) + int(failed) for passed, failed in counts)
    if runner is Runner.PYTHON:
        return re.search(r"^Ran [1-9]\d* tests?", text, re.M) is not None
    if runner is Runner.NODE:
        return re.search(r"^# tests [1-9]", text, re.M) is not None
    if runner in (Runner.JEST, Runner.VITEST, Runner.STORYBOOK):
        return (
            re.search(r"^\s*Tests:?\s.*\b\d+ (passed|failed)", text, re.M) is not None
        )
    return True


def execute(model: Model, step: Step) -> Outcome:
    cwd = model.root / step.cwd
    env = environment(model, step)
    if step.prepare and any(not (model.root / path).exists() for path in step.needs):
        print(f"\n==> prepare {step.check.id}\n$ {step.prepare}", flush=True)
        if subprocess.run(["bash", "-c", step.prepare], cwd=cwd, env=env).returncode:
            return Outcome.FAILED
    print(f"\n==> {step.what}\n$ {step.display()}", flush=True)
    command = (
        step.command if isinstance(step.command, list) else ["bash", "-c", step.command]
    )
    if not step.narrowed or step.watches:
        returncode = subprocess.run(command, cwd=cwd, env=env).returncode
        return Outcome.PASSED if returncode == 0 else Outcome.FAILED
    # Narrowed runs are read as they stream, to catch a filter that ran nothing.
    output = []
    with subprocess.Popen(
        command,
        cwd=cwd,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        errors="replace",
    ) as process:
        assert process.stdout is not None
        for line in process.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()
            output.append(line)
    if process.returncode != 0:
        return Outcome.FAILED
    return (
        Outcome.PASSED
        if tests_ran(step.check.runner, "".join(output))
        else Outcome.EMPTY
    )


def cargo_watch(step: Step) -> Step:
    """Rerun a Cargo step with cargo-watch when the crates it builds change."""
    command = ["cargo", "watch"]
    for path in step.watch_paths:
        command += ["-w", relative_to(path, step.cwd)]
    command += ["-s", step.shell()]
    return replace(step, command=command, watches=True, narrowed=False)


def watched_files(root: Path, paths: Sequence[str]) -> Iterator[Path]:
    for path in paths:
        start = root / path
        if start.is_file():
            yield start
            continue
        for current, dirs, files in os.walk(start):
            dirs[:] = sorted(
                name
                for name in dirs
                if not name.startswith(".")
                and name not in UNWATCHED
                and not TARGET_DIR_NAME.fullmatch(name)
            )
            yield from (Path(current) / name for name in sorted(files))


def fingerprint(root: Path, paths: Sequence[str]) -> str:
    digest = hashlib.sha256()
    for file in watched_files(root, paths):
        try:
            stat = file.stat()
        except OSError:
            continue
        digest.update(f"{file}\0{stat.st_mtime_ns}\0{stat.st_size}\n".encode())
    return digest.hexdigest()


def outermost(paths: Sequence[str]) -> list[str]:
    """The paths that no other path in the list contains."""
    unique = sorted(set(paths))
    return [
        path
        for path in unique
        if not any(path.startswith(f"{other}/") for other in unique if other != path)
    ]


def poll(model: Model, steps: Sequence[Step]) -> NoReturn:
    """Rerun the steps whenever a watched file changes, until interrupted."""
    paths = outermost([path for step in steps for path in step.watch_paths])
    print(f"\nwatching {', '.join(paths)} (Ctrl-C stops)", flush=True)
    last = None
    while True:
        current = fingerprint(model.root, paths)
        if current != last:
            outcomes = [execute(model, step) for step in steps]
            good = all(outcome is Outcome.PASSED for outcome in outcomes)
            print(
                f"\n{'passed' if good else 'FAILED'}; waiting for changes", flush=True
            )
            last = fingerprint(model.root, paths)
        time.sleep(POLL_SECONDS)


def watch(model: Model, plan: Plan) -> int:
    if len(plan.steps) == 1:
        step = plan.steps[0]
        if step.check.runner is Runner.CARGO and shutil.which("cargo-watch"):
            step = cargo_watch(step)
        if step.watches:
            return 0 if execute(model, step) is Outcome.PASSED else 1
    return poll(model, plan.steps)


def print_plan(model: Model, plan: Plan, prog: str) -> None:
    print(f"{prog}: {plan.heading}")
    if plan.unit is not None:
        where = plan.unit.path or plan.unit.summary
        print(f"  unit      {plan.unit.id} ({plan.unit.kind.value}: {where})")
    for note in plan.notes:
        print(f"  note      {note}")
    for step in plan.steps:
        print(f"  runs      {step.what} [{step.check.cost.value}]")
        if step.reason:
            print(f"            because {step.reason}")
        print(f"            {step.display()}")
        if step.check.runner is Runner.CARGO:
            directory = environment(model, step)["CARGO_TARGET_DIR"]
            print(f"            CARGO_TARGET_DIR={directory}")
        for needed in step.needs:
            state = (
                "present" if (model.root / needed).exists() else "missing, built first"
            )
            print(f"  needs     {needed} ({state}): {step.prepare}")
    for line in plan.not_run:
        print(f"  not run   {line}")


def run(
    model: Model, plan: Plan, dry_run: bool, watching: bool, prog: str = "step-dev test"
) -> int:
    print_plan(model, plan, prog)
    if dry_run or not plan.steps:
        return 0
    results = []
    try:
        if watching:
            return watch(model, plan)
        for step in plan.steps:
            started = time.monotonic()
            reason = missing(model, step)
            outcome = Outcome.UNAVAILABLE if reason else execute(model, step)
            results.append((step, outcome, time.monotonic() - started, reason))
    except KeyboardInterrupt:
        print("\ninterrupted", file=sys.stderr)
        return 130
    print("\nSummary:")
    width = max(len(step.check.id) for step, _, _, _ in results)
    for step, outcome, seconds, reason in results:
        detail = f"  {reason}" if reason else ""
        print(
            f"  {outcome.value:<12} {step.check.id:<{width}}  {seconds:7.1f} s{detail}"
        )
    passed = all(outcome is Outcome.PASSED for _, outcome, _, _ in results)
    return 0 if passed else 1


def parser(prog: str) -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(
        prog=prog,
        description=" ".join(__doc__.split("\n\n")[1].split()),
        epilog="Examples: step-dev test voting-portal; step-dev test "
        "packages/ui-essentials/src/components/Header/Header.tsx; step-dev test "
        "admin-portal --story screens-admin-tally-ceremony--populated; step-dev test "
        "packages/voting-portal/test/journeys/review.spec.ts 'shows the review'; "
        "step-dev test windmill services::probe; step-dev test --affected. "
        "Arguments after -- go to the underlying runner.",
    )
    root.add_argument(
        "target",
        nargs="?",
        help="package or area (voting-portal, windmill, scripts-dev), check "
        "(jest:voting-portal), file or directory, or Storybook URL",
    )
    root.add_argument(
        "name",
        nargs="?",
        help="run only tests whose name matches: Jest and Vitest -t, Playwright -g, "
        "the Cargo filter, unittest -k",
    )
    root.add_argument(
        "-t", "-g", "-k", dest="name_option", metavar="NAME", help="same as name"
    )
    root.add_argument(
        "--story", metavar="ID", help="a story or title ID of the package"
    )
    root.add_argument("--watch", action="store_true", help="rerun when sources change")
    depth = root.add_mutually_exclusive_group()
    depth.add_argument(
        "--depth",
        type=Depth,
        choices=list(Depth),
        default=Depth.FAST,
        help="fast (default): fast checks; broad: also slow ones such as browser "
        "suites and production builds; full: also integration checks with services",
    )
    depth.add_argument(
        "--broad",
        dest="depth",
        action="store_const",
        const=Depth.BROAD,
        help="same as --depth broad",
    )
    root.add_argument(
        "--affected",
        action="store_true",
        help="run the checks the current changes select, including the working tree",
    )
    add_change_options(root, Scope.WORKTREE)
    root.add_argument(
        "--dry-run", action="store_true", help="print the scope without running it"
    )
    root.add_argument("--list", action="store_true", help="list every check")
    return root


def list_checks(model: Model) -> None:
    width = max(len(check.id) for check in model.checks)
    for check in by_cost(model.checks):
        cwd = "" if check.cwd == "." else f"cd {check.cwd} && "
        print(f"{check.cost.value:<11} {check.id:<{width}}  {cwd}{check.command}")


def main(
    argv: Sequence[str] | None = None,
    prog: str = "step-dev test",
    root: Path = ROOT,
    defaults: Sequence[str] = (),
) -> int:
    arguments = [*defaults, *(sys.argv[1:] if argv is None else argv)]
    extra: list[str] = []
    if "--" in arguments:
        index = arguments.index("--")
        arguments, extra = arguments[:index], arguments[index + 1 :]
    options = parser(prog).parse_args(arguments)
    options.extra = extra
    options.name = options.name_option or options.name
    try:
        model = load_model(root)
        if options.list:
            list_checks(model)
            return 0
        plan = Resolver(model, options).plan()
        if options.watch and len(plan.steps) > 1:
            # Several steps rerun together, so none may watch by itself.
            options.watch = False
            plan = Resolver(model, options).plan()
            options.watch = True
    except (ConfigError, GitError, SelectionError, OSError, ValueError) as error:
        print(f"{prog}: {error}", file=sys.stderr)
        return 2
    return run(model, plan, options.dry_run, options.watch, prog)


if __name__ == "__main__":
    # Output piped into head or less may be cut short; that is not an error.
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    sys.exit(main())
