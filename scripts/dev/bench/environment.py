# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Conditions recorded with every result: revision, tools and machine."""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

HARNESS_ROOT = Path(__file__).resolve().parents[3]
MAX_LISTED_CHANGES = 50

TOOL_COMMANDS: dict[str, tuple[str, ...]] = {
    "node": ("node", "--version"),
    "yarn": ("yarn", "--version"),
    "rustc": ("rustc", "--version"),
    "cargo": ("cargo", "--version"),
    "wasm-pack": ("wasm-pack", "--version"),
    "docker": ("docker", "version", "--format", "{{.Client.Version}}"),
    "devcontainer": ("devcontainer", "--version"),
}


def _output(
    command: Sequence[str],
    cwd: Path | None = None,
    env: Mapping[str, str] | None = None,
) -> str | None:
    if shutil.which(command[0], path=(env or os.environ).get("PATH")) is None:
        return None
    try:
        completed = subprocess.run(
            list(command),
            cwd=cwd,
            env=dict(env) if env is not None else None,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if completed.returncode != 0:
        return None
    return completed.stdout.strip().splitlines()[0] if completed.stdout.strip() else ""


def tool_versions(
    extra: Mapping[str, Sequence[str]] | None = None,
    env: Mapping[str, str] | None = None,
) -> dict[str, str | None]:
    """Version of each tool on PATH; ``None`` when absent."""
    commands: dict[str, Sequence[str]] = dict(TOOL_COMMANDS)
    commands.update(extra or {})
    versions = {name: _output(command, env=env) for name, command in commands.items()}
    versions["python"] = sys.version.split()[0]
    return versions


def git_state(path: Path) -> dict[str, Any]:
    """Revision, working-tree changes and submodule revisions of a checkout."""
    commit = _output(("git", "-C", str(path), "rev-parse", "HEAD"))
    status = subprocess.run(
        ["git", "-C", str(path), "status", "--porcelain=v1"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout.splitlines()
    submodules: dict[str, str] = {}
    listing = subprocess.run(
        ["git", "-C", str(path), "submodule", "status"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    for line in listing.splitlines():
        fields = line.strip().split()
        if len(fields) >= 2:
            submodules[fields[1]] = fields[0].lstrip("+-U")
    return {
        "path": str(path),
        "commit": commit,
        "dirty": bool(status),
        "change_count": len(status),
        "changes": status[:MAX_LISTED_CHANGES],
        "submodules": submodules,
    }


def host_info() -> dict[str, Any]:
    memory_kib = None
    meminfo = Path("/proc/meminfo")
    if meminfo.exists():
        for line in meminfo.read_text().splitlines():
            if line.startswith("MemTotal:"):
                memory_kib = int(line.split()[1])
    return {
        "hostname": platform.node(),
        "machine": platform.machine(),
        "kernel": platform.release(),
        "cpus": os.cpu_count(),
        "mem_total_kib": memory_kib,
        "in_container": Path("/.dockerenv").exists(),
    }


def harness_info() -> dict[str, Any]:
    state = git_state(HARNESS_ROOT)
    return {"path": state["path"], "commit": state["commit"], "dirty": state["dirty"]}
