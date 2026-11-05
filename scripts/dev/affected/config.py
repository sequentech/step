# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The declarative part of the model, ``scripts/dev/affected.toml``."""

from __future__ import annotations

import enum
import functools
import re
import tomllib
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, TypeVar

CONFIG_FILE = Path("scripts/dev/affected.toml")
SCHEMA = 1
UNIT_FIELD = "{unit}"
PATH_FIELD = "{path}"

E = TypeVar("E", bound=enum.Enum)


class ConfigError(ValueError):
    """The model file is malformed or names something that does not exist."""


class WorkspaceKind(enum.Enum):
    YARN = "yarn"
    CARGO = "cargo"


class Cost(enum.Enum):
    """What a check needs beyond the installed workspace, cheapest first.

    fast: dependencies, compilers and a headless browser for small suites.
    slow: whole-catalogue browser suites, production, release or cross-target
    builds, the JVM and the documentation site. integration: services such as
    Docker stacks or a database server.
    """

    FAST = "fast"
    SLOW = "slow"
    INTEGRATION = "integration"

    @property
    def rank(self) -> int:
        return list(Cost).index(self)


class Trigger(enum.Enum):
    """Whether a check follows its units' dependencies or only their own files."""

    AFFECTED = "affected"
    CHANGED = "changed"


class CheckKind(enum.Enum):
    TEST = "test"
    TYPES = "types"
    LINT = "lint"
    BUILD = "build"


class Runner(enum.Enum):
    """The tool behind a check; it decides how ``step-dev test`` narrows it."""

    JEST = "jest"
    VITEST = "vitest"
    STORYBOOK = "storybook"
    PLAYWRIGHT = "playwright"
    CARGO = "cargo"
    PYTHON = "python"
    NODE = "node"
    MAVEN = "maven"
    SHELL = "shell"


class Requirement(enum.Enum):
    """A tool or service a check needs that the workspace install does not provide."""

    CHROMIUM = "chromium"
    DOCKER = "docker"
    MAVEN = "mvn"
    REUSE = "reuse"
    RUFF = "ruff"
    POSTGRES_SERVER = "postgres-server"


def translate(pattern: str) -> str:
    """A regular expression for a glob: ``**`` spans directories, ``*`` does not."""
    parts = []
    index = 0
    while index < len(pattern):
        if pattern.startswith("**/", index):
            parts.append("(?:[^/]+/)*")
            index += 3
        elif pattern.startswith("**", index):
            parts.append(".*")
            index += 2
        elif pattern[index] == "*":
            parts.append("[^/]*")
            index += 1
        elif pattern[index] == "?":
            parts.append("[^/]")
            index += 1
        else:
            parts.append(re.escape(pattern[index]))
            index += 1
    return "".join(parts)


@functools.cache
def compiled(pattern: str) -> re.Pattern[str]:
    return re.compile(translate(pattern))


@dataclass(frozen=True)
class Glob:
    """A pattern over repository-relative paths with ``/`` separators."""

    pattern: str

    def __post_init__(self) -> None:
        if not self.pattern or self.pattern.startswith("/"):
            raise ConfigError(
                f"glob must be relative to the repository: {self.pattern!r}"
            )
        if ".." in self.pattern.split("/"):
            raise ConfigError(f"glob must stay inside the repository: {self.pattern!r}")

    def matches(self, path: str) -> bool:
        return compiled(self.pattern).fullmatch(path) is not None


@dataclass(frozen=True)
class WorkspaceSpec:
    kind: WorkspaceKind
    manifest: str
    # Prepended to package names, for workspaces whose names clash with another's.
    prefix: str = ""
    # Per-checkout Cargo target directory for local runs.
    target_dir: str | None = None


@dataclass(frozen=True)
class UnitSpec:
    """A ``[units.<id>]`` table: an area, or edges and inputs of a package."""

    id: str
    summary: str | None = None
    depends: tuple[str, ...] = ()
    consumers: tuple[str, ...] = ()
    inputs: tuple[Glob, ...] = ()
    test_inputs: tuple[Glob, ...] = ()


@dataclass(frozen=True)
class PathRule:
    globs: tuple[Glob, ...]
    units: tuple[str, ...]
    # A reason to select every check when the rule claims a path.
    broad: str | None = None

    def match(self, path: str) -> Glob | None:
        return next((glob for glob in self.globs if glob.matches(path)), None)


@dataclass(frozen=True)
class CheckSpec:
    """One check, with families expanded."""

    id: str
    # The unit the check tests: the family member, or the first unit it names.
    owner: str | None
    # Unit ids or selectors (``cargo:*``, ``yarn:*``, ``cargo:<manifest>``).
    units: tuple[str, ...]
    kind: CheckKind
    runner: Runner
    cost: Cost
    trigger: Trigger
    cwd: str
    command: str
    env: Mapping[str, str] = field(default_factory=dict)
    # Command prefix that ``step-dev test`` extends with files, stories or names.
    focus: tuple[str, ...] | None = None
    # Files that must exist before a focused run, and the command that makes them.
    needs: tuple[str, ...] = ()
    prepare: str | None = None
    tests: tuple[Glob, ...] = ()
    paths: tuple[Glob, ...] = ()
    requires: tuple[Requirement, ...] = ()
    workflows: tuple[str, ...] = ()
    actions: tuple[str, ...] = ()


@dataclass(frozen=True)
class Config:
    default_base: str
    workspaces: tuple[WorkspaceSpec, ...]
    units: dict[str, UnitSpec]
    rules: tuple[PathRule, ...]
    checks: tuple[dict[str, Any], ...]


TOP_LEVEL = {"schema", "default_base", "workspaces", "units", "paths", "checks"}
UNIT_KEYS = {"summary", "depends", "consumers", "inputs", "test_inputs"}
RULE_KEYS = {"globs", "units", "broad"}
WORKSPACE_KEYS = {"kind", "manifest", "prefix", "target_dir"}
CHECK_KEYS = {
    "id",
    "for_each",
    "units",
    "also",
    "also_workflows",
    "commands",
    "prepares",
    "kind",
    "runner",
    "cost",
    "trigger",
    "cwd",
    "command",
    "env",
    "focus",
    "needs",
    "prepare",
    "tests",
    "paths",
    "requires",
    "workflows",
    "actions",
}
CHECK_REQUIRED = ("id", "kind", "runner", "cost", "cwd", "command")


def _keys(table: Mapping[str, Any], allowed: set[str], where: str) -> None:
    unknown = sorted(set(table) - allowed)
    if unknown:
        raise ConfigError(f"{where}: unknown keys {', '.join(unknown)}")


def _string(table: Mapping[str, Any], key: str, where: str) -> str:
    value = table.get(key)
    if not isinstance(value, str) or not value:
        raise ConfigError(f"{where}: {key!r} must be a non-empty string")
    return value


def _optional_string(table: Mapping[str, Any], key: str, where: str) -> str | None:
    return _string(table, key, where) if key in table else None


def _strings(table: Mapping[str, Any], key: str, where: str) -> tuple[str, ...]:
    value = table.get(key, [])
    if not isinstance(value, list) or not all(
        isinstance(item, str) and item for item in value
    ):
        raise ConfigError(f"{where}: {key!r} must be a list of non-empty strings")
    return tuple(value)


def _enum(
    table: Mapping[str, Any],
    key: str,
    kind: type[E],
    where: str,
    default: E | None = None,
) -> E:
    if key not in table and default is not None:
        return default
    try:
        return kind(table.get(key))
    except ValueError:
        choices = ", ".join(member.value for member in kind)
        raise ConfigError(f"{where}: {key!r} must be one of {choices}") from None


def _string_map(table: Mapping[str, Any], key: str, where: str) -> dict[str, str]:
    value = table.get(key, {})
    if not isinstance(value, dict) or not all(
        isinstance(item, str) for item in value.values()
    ):
        raise ConfigError(f"{where}: {key!r} must be a table of strings")
    return dict(value)


def parse_unit(unit_id: str, table: Any) -> UnitSpec:
    where = f"units.{unit_id}"
    if not isinstance(table, dict):
        raise ConfigError(f"{where}: expected a table")
    _keys(table, UNIT_KEYS, where)
    return UnitSpec(
        id=unit_id,
        summary=_optional_string(table, "summary", where),
        depends=_strings(table, "depends", where),
        consumers=_strings(table, "consumers", where),
        inputs=tuple(Glob(pattern) for pattern in _strings(table, "inputs", where)),
        test_inputs=tuple(
            Glob(pattern) for pattern in _strings(table, "test_inputs", where)
        ),
    )


def parse_rule(index: int, table: Any) -> PathRule:
    where = f"paths[{index}]"
    if not isinstance(table, dict):
        raise ConfigError(f"{where}: expected a table")
    _keys(table, RULE_KEYS, where)
    globs = _strings(table, "globs", where)
    units = _strings(table, "units", where)
    if not globs or not units:
        raise ConfigError(f"{where}: needs globs and units")
    return PathRule(
        globs=tuple(Glob(pattern) for pattern in globs),
        units=units,
        broad=_optional_string(table, "broad", where),
    )


def parse_workspace(index: int, table: Any) -> WorkspaceSpec:
    where = f"workspaces[{index}]"
    if not isinstance(table, dict):
        raise ConfigError(f"{where}: expected a table")
    _keys(table, WORKSPACE_KEYS, where)
    return WorkspaceSpec(
        kind=_enum(table, "kind", WorkspaceKind, where),
        manifest=_string(table, "manifest", where),
        prefix=table.get("prefix", ""),
        target_dir=_optional_string(table, "target_dir", where),
    )


def parse_config(document: Mapping[str, Any]) -> Config:
    _keys(document, TOP_LEVEL, "model")
    if document.get("schema") != SCHEMA:
        raise ConfigError(f"model: schema must be {SCHEMA}")
    units = document.get("units", {})
    rules = document.get("paths", [])
    checks = document.get("checks", [])
    workspaces = document.get("workspaces", [])
    if not all(isinstance(value, list) for value in (rules, checks, workspaces)):
        raise ConfigError("model: paths, checks and workspaces must be arrays")
    for index, check in enumerate(checks):
        where = f"checks[{index}]"
        if not isinstance(check, dict):
            raise ConfigError(f"{where}: expected a table")
        _keys(check, CHECK_KEYS, check.get("id", where))
        for key in CHECK_REQUIRED:
            _string(check, key, check.get("id", where))
    return Config(
        default_base=_string(document, "default_base", "model"),
        workspaces=tuple(
            parse_workspace(index, table) for index, table in enumerate(workspaces)
        ),
        units={unit_id: parse_unit(unit_id, table) for unit_id, table in units.items()},
        rules=tuple(parse_rule(index, table) for index, table in enumerate(rules)),
        checks=tuple(checks),
    )


def load_config(path: Path) -> Config:
    try:
        document = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ConfigError(f"{path}: {error}") from error
    return parse_config(document)


def _fill(value: str, unit: str | None, path: str | None, where: str) -> str:
    if unit is None:
        if UNIT_FIELD in value or PATH_FIELD in value:
            raise ConfigError(f"{where}: {{unit}} and {{path}} need for_each")
        return value
    if PATH_FIELD in value and path is None:
        raise ConfigError(f"{where}: {unit} has no directory for {{path}}")
    return value.replace(UNIT_FIELD, unit).replace(PATH_FIELD, path or "")


def expand_check(
    table: Mapping[str, Any], unit: str | None, path: str | None
) -> CheckSpec:
    """One check of a table; ``unit`` is the family member, if it is a family."""
    where = _fill(table["id"], unit, path, table["id"])

    def fill(value: str) -> str:
        return _fill(value, unit, path, where)

    extra = table.get("also", {}).get(unit, []) if unit else []
    workflows = table.get("also_workflows", {}).get(unit, []) if unit else []
    command = table.get("commands", {}).get(unit, table["command"])
    prepare = table.get("prepares", {}).get(unit, table.get("prepare"))
    units = ((unit,) if unit else ()) + _strings(table, "units", where) + tuple(extra)
    focus = _strings(table, "focus", where)
    requires = []
    for name in _strings(table, "requires", where):
        try:
            requires.append(Requirement(name))
        except ValueError:
            raise ConfigError(f"{where}: unknown requirement {name!r}") from None
    return CheckSpec(
        id=where,
        owner=unit or next((name for name in units if ":" not in name), None),
        units=tuple(dict.fromkeys(units)),
        kind=_enum(table, "kind", CheckKind, where),
        runner=_enum(table, "runner", Runner, where),
        cost=_enum(table, "cost", Cost, where),
        trigger=_enum(table, "trigger", Trigger, where, Trigger.AFFECTED),
        cwd=fill(table["cwd"]),
        command=fill(command),
        env=_string_map(table, "env", where),
        focus=tuple(fill(part) for part in focus) or None,
        needs=tuple(fill(item) for item in _strings(table, "needs", where)),
        prepare=fill(prepare) if prepare else None,
        tests=tuple(Glob(fill(item)) for item in _strings(table, "tests", where)),
        paths=tuple(Glob(fill(item)) for item in _strings(table, "paths", where)),
        requires=tuple(requires),
        workflows=_strings(table, "workflows", where) + tuple(workflows),
        actions=_strings(table, "actions", where),
    )


def expand_checks(
    tables: tuple[dict[str, Any], ...], unit_paths: Mapping[str, str | None]
) -> list[CheckSpec]:
    """Every check, with one per family member; ``unit_paths`` maps ids to dirs."""
    checks: list[CheckSpec] = []
    for table in tables:
        members = _strings(table, "for_each", table["id"])
        for key in ("also", "also_workflows", "commands", "prepares"):
            overrides = table.get(key, {})
            if not isinstance(overrides, dict):
                raise ConfigError(f"{table['id']}: {key!r} must be a table")
            strays = sorted(set(overrides) - set(members))
            if strays:
                raise ConfigError(
                    f"{table['id']}: {key} names units outside for_each: "
                    + ", ".join(strays)
                )
        if not members:
            checks.append(expand_check(table, None, None))
            continue
        if UNIT_FIELD not in table["id"]:
            raise ConfigError(f"{table['id']}: a family id must contain {{unit}}")
        for member in members:
            if member not in unit_paths:
                raise ConfigError(f"{table['id']}: unknown unit {member!r}")
            checks.append(expand_check(table, member, unit_paths[member]))
    seen: set[str] = set()
    for check in checks:
        if check.id in seen:
            raise ConfigError(f"duplicate check id {check.id!r}")
        seen.add(check.id)
    return sorted(checks, key=lambda check: check.id)
