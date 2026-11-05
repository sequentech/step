# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Packages and their dependencies, read from the Yarn and Cargo manifests."""

from __future__ import annotations

import enum
import json
import os
import re
import tomllib
from collections.abc import Iterable, Iterator, Mapping
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath
from typing import Any

from .config import ConfigError, Glob, WorkspaceKind, WorkspaceSpec

YARN_DEPENDENCIES = (
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
)
CARGO_DEPENDENCIES = ("dependencies", "dev-dependencies", "build-dependencies")
LOCAL_PROTOCOLS = ("file:", "link:")
YARN_LOCK = "yarn.lock"
CARGO_LOCK = "Cargo.lock"
# Directories that hold build output or installed dependencies, never sources.
SKIPPED_DIRS = frozenset({"target", "node_modules", ".git"})
TARGET_DIR = re.compile(r"rust-.*-target")
INCLUDE = re.compile(
    r'(?:include_str|include_bytes)!\s*\(\s*"([^"\\]+)"\s*\)|#\[path\s*=\s*"([^"\\]+)"\]'
)


class UnitKind(enum.Enum):
    YARN = "yarn"
    CARGO = "cargo"
    AREA = "area"


@dataclass
class Unit:
    id: str
    kind: UnitKind
    # Package name for Yarn and Cargo units, the id for areas.
    name: str
    # Directory relative to the repository root; areas have none.
    path: str | None = None
    summary: str = ""
    # Manifest of the workspace the package belongs to.
    workspace: str | None = None
    depends: set[str] = field(default_factory=set)
    inputs: list[Glob] = field(default_factory=list)
    test_inputs: list[Glob] = field(default_factory=list)


class FileRole(enum.Enum):
    MANIFEST = "manifest"
    LOCKFILE = "lockfile"


@dataclass(frozen=True)
class WorkspaceFile:
    """A workspace manifest or lockfile, which affects the workspace's packages."""

    path: str
    role: FileRole
    kind: WorkspaceKind
    workspace: str


def relative(root: Path, path: Path) -> str:
    return PurePosixPath(os.path.relpath(path, root)).as_posix()


def inside(root: Path, path: Path) -> bool:
    return not relative(root, path).startswith("../")


def read_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ConfigError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise ConfigError(f"{path}: expected a JSON object")
    return document


def read_toml(path: Path) -> dict[str, Any]:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ConfigError(f"{path}: {error}") from error


def expand_members(base: Path, patterns: Iterable[str], manifest: str) -> list[Path]:
    """Directories named by workspace member patterns, which may use ``*``."""
    found: list[Path] = []
    for pattern in patterns:
        matches = sorted(base.glob(pattern)) if "*" in pattern else [base / pattern]
        directories = [
            path.resolve() for path in matches if (path / manifest).is_file()
        ]
        if not directories and "*" not in pattern:
            raise ConfigError(f"{base}: workspace member {pattern} has no {manifest}")
        found += directories
    return found


def discover_yarn(
    root: Path, spec: WorkspaceSpec
) -> tuple[list[Unit], list[WorkspaceFile]]:
    manifest = root / spec.manifest
    base = manifest.parent
    document = read_json(manifest)
    workspaces = document.get("workspaces", [])
    if isinstance(workspaces, dict):
        workspaces = workspaces.get("packages", [])
    packages = {
        path: read_json(path / "package.json")
        for path in expand_members(base, workspaces, "package.json")
    }
    by_name = {str(data.get("name")): path for path, data in packages.items()}
    units: dict[Path, Unit] = {}
    for path, data in packages.items():
        name = str(data.get("name"))
        unit_id = spec.prefix + name.rsplit("/", 1)[-1]
        units[path] = Unit(
            id=unit_id,
            kind=UnitKind.YARN,
            name=name,
            path=relative(root, path),
            workspace=spec.manifest,
        )
    # Packages resolved from a file elsewhere in the workspace, such as a
    # committed tgz, by name: a plain range for that name gets the same files.
    local_files: dict[str, set[str]] = {}
    for path, data in packages.items():
        for table in YARN_DEPENDENCIES:
            for dependency, version in (data.get(table) or {}).items():
                if dependency in by_name or not str(version).startswith(
                    LOCAL_PROTOCOLS
                ):
                    continue
                target = (path / str(version).split(":", 1)[1]).resolve()
                if target.is_file() and inside(root, target):
                    local_files.setdefault(dependency, set()).add(
                        relative(root, target)
                    )
    for path, data in packages.items():
        unit = units[path]
        for table in YARN_DEPENDENCIES:
            for dependency, version in sorted((data.get(table) or {}).items()):
                version = str(version)
                if dependency in by_name and not version.startswith(LOCAL_PROTOCOLS):
                    unit.depends.add(units[by_name[dependency]].id)
                elif version.startswith(LOCAL_PROTOCOLS):
                    target = (path / version.split(":", 1)[1]).resolve()
                    if target in units:
                        unit.depends.add(units[target].id)
                    elif inside(root, target) and not inside(path, target):
                        pattern = relative(root, target)
                        unit.inputs.append(
                            Glob(pattern if target.is_file() else f"{pattern}/**")
                        )
                elif dependency in local_files:
                    unit.inputs += [
                        Glob(file) for file in sorted(local_files[dependency])
                    ]
    files = [
        WorkspaceFile(
            spec.manifest, FileRole.MANIFEST, WorkspaceKind.YARN, spec.manifest
        ),
        WorkspaceFile(
            relative(root, base / YARN_LOCK),
            FileRole.LOCKFILE,
            WorkspaceKind.YARN,
            spec.manifest,
        ),
    ]
    return list(units.values()), files


def cargo_path_dependencies(
    manifest: Mapping[str, Any], workspace_paths: Mapping[str, Path], directory: Path
) -> Iterator[Path]:
    """Directories of every path dependency, of any kind and target."""
    tables: list[Mapping[str, Any]] = [
        manifest.get(kind) or {} for kind in CARGO_DEPENDENCIES
    ]
    for target in (manifest.get("target") or {}).values():
        tables += [target.get(kind) or {} for kind in CARGO_DEPENDENCIES]
    for table in tables:
        for name, value in table.items():
            if not isinstance(value, dict):
                continue
            if "path" in value:
                yield (directory / value["path"]).resolve()
            elif value.get("workspace") and name in workspace_paths:
                yield workspace_paths[name]


def discover_cargo(
    root: Path, spec: WorkspaceSpec
) -> tuple[list[Unit], list[WorkspaceFile]]:
    manifest_path = root / spec.manifest
    base = manifest_path.parent
    workspace = read_toml(manifest_path).get("workspace", {})
    excluded = {(base / path).resolve() for path in workspace.get("exclude", [])}
    workspace_paths = {
        name: (base / value["path"]).resolve()
        for name, value in (workspace.get("dependencies") or {}).items()
        if isinstance(value, dict) and "path" in value
    }
    pending = [
        path
        for path in expand_members(base, workspace.get("members", []), "Cargo.toml")
        if path not in excluded
    ]
    manifests: dict[Path, dict[str, Any]] = {}
    edges: dict[Path, set[Path]] = {}
    # Path dependencies outside the member list are compiled as packages too.
    while pending:
        directory = pending.pop()
        if directory in manifests:
            continue
        manifest = read_toml(directory / "Cargo.toml")
        manifests[directory] = manifest
        edges[directory] = set(
            cargo_path_dependencies(manifest, workspace_paths, directory)
        )
        pending += sorted(edges[directory] - set(manifests))
    units: dict[Path, Unit] = {}
    for directory, manifest in sorted(manifests.items()):
        name = str((manifest.get("package") or {}).get("name", directory.name))
        units[directory] = Unit(
            id=spec.prefix + name,
            kind=UnitKind.CARGO,
            name=name,
            path=relative(root, directory),
            workspace=spec.manifest,
        )
    for directory, targets in edges.items():
        for target in targets:
            if target not in units:
                raise ConfigError(
                    f"{directory}: path dependency {target} has no Cargo.toml"
                )
            units[directory].depends.add(units[target].id)
    files = [
        WorkspaceFile(
            spec.manifest, FileRole.MANIFEST, WorkspaceKind.CARGO, spec.manifest
        ),
        WorkspaceFile(
            relative(root, base / CARGO_LOCK),
            FileRole.LOCKFILE,
            WorkspaceKind.CARGO,
            spec.manifest,
        ),
    ]
    return list(units.values()), files


def skipped_directory(name: str) -> bool:
    return (
        name.startswith(".") or name in SKIPPED_DIRS or bool(TARGET_DIR.fullmatch(name))
    )


def rust_sources(directory: Path, nested: set[Path]) -> Iterator[Path]:
    for current, dirs, files in os.walk(directory):
        base = Path(current)
        dirs[:] = sorted(
            name
            for name in dirs
            if not skipped_directory(name) and (base / name).resolve() not in nested
        )
        yield from (base / name for name in sorted(files) if name.endswith(".rs"))


def external_includes(root: Path, unit: Unit, nested: set[Path]) -> list[str]:
    """Files outside the crate that its sources compile in with include macros."""
    directory = (root / str(unit.path)).resolve()
    found = set()
    for source in rust_sources(directory, nested):
        try:
            text = source.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for match in INCLUDE.finditer(text):
            target = (source.parent / (match.group(1) or match.group(2))).resolve()
            if inside(root, target) and not inside(directory, target):
                found.add(relative(root, target))
    return sorted(found)


def add_includes(root: Path, units: Iterable[Unit]) -> None:
    cargo = [unit for unit in units if unit.kind is UnitKind.CARGO]
    directories = {(root / str(unit.path)).resolve() for unit in cargo}
    for unit in cargo:
        own = (root / str(unit.path)).resolve()
        nested = {path for path in directories if path != own and inside(own, path)}
        unit.inputs += [Glob(path) for path in external_includes(root, unit, nested)]


def discover(
    root: Path, specs: Iterable[WorkspaceSpec]
) -> tuple[list[Unit], list[WorkspaceFile]]:
    units: list[Unit] = []
    files: list[WorkspaceFile] = []
    for spec in specs:
        discovered, workspace_files = (
            discover_yarn(root, spec)
            if spec.kind is WorkspaceKind.YARN
            else discover_cargo(root, spec)
        )
        units += discovered
        files += workspace_files
    add_includes(root, units)
    return units, files


# Cargo.lock ------------------------------------------------------------------------

LockKey = tuple[str, str, str]


def lock_packages(text: str) -> dict[LockKey, dict[str, Any]]:
    packages = tomllib.loads(text).get("package", [])
    return {
        (str(entry["name"]), str(entry["version"]), str(entry.get("source", ""))): entry
        for entry in packages
    }


def lock_graph(
    packages: Mapping[LockKey, Mapping[str, Any]],
) -> dict[LockKey, set[LockKey]]:
    """Each locked package's dependencies, resolving the lockfile's short forms."""
    by_name: dict[str, list[LockKey]] = {}
    for key in packages:
        by_name.setdefault(key[0], []).append(key)
    graph: dict[LockKey, set[LockKey]] = {}
    for key, entry in packages.items():
        graph[key] = set()
        for reference in entry.get("dependencies", []):
            name, _, rest = str(reference).partition(" ")
            version, _, source = rest.partition(" ")
            candidates = [
                candidate
                for candidate in by_name.get(name, [])
                if (not version or candidate[1] == version)
                and (not source or f"({candidate[2]})" == source)
            ]
            graph[key].update(candidates)
    return graph


def reaching(
    graph: Mapping[LockKey, set[LockKey]], targets: set[LockKey]
) -> set[LockKey]:
    """Packages that depend on any target, directly or transitively, and the targets."""
    consumers: dict[LockKey, set[LockKey]] = {}
    for key, dependencies in graph.items():
        for dependency in dependencies:
            consumers.setdefault(dependency, set()).add(key)
    found = set(targets)
    pending = list(targets)
    while pending:
        for consumer in consumers.get(pending.pop(), ()):
            if consumer not in found:
                found.add(consumer)
                pending.append(consumer)
    return found


def cargo_lock_impact(old: str, new: str) -> set[str]:
    """Names of local packages whose locked dependency closure differs.

    Local packages have no source. Removed packages are followed in the old
    graph and added ones in the new, so both directions of a change count.
    """
    before = lock_packages(old)
    after = lock_packages(new)
    changed = {
        key
        for key in before.keys() | after.keys()
        if key not in before
        or key not in after
        or before[key].get("checksum") != after[key].get("checksum")
        or sorted(before[key].get("dependencies", []))
        != sorted(after[key].get("dependencies", []))
    }
    found = reaching(lock_graph(before), changed & before.keys())
    found |= reaching(lock_graph(after), changed & after.keys())
    return {key[0] for key in found if not key[2]}
