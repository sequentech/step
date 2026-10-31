# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Isolated Docker daemons and disposable checkout copies for workspace runs.

The repository's devcontainer uses fixed container and compose project names, so
running it on a shared daemon would recreate another developer's stack. Every
workspace measurement therefore targets a Docker-in-Docker daemon that this
harness either created or was explicitly given, never the default daemon.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import time
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

DEFAULT_DAEMON_SOCKETS = frozenset({"/var/run/docker.sock", "/run/docker.sock"})
OWNER_LABEL = "org.sequentech.step-bench.owner"
DIND_READY_TIMEOUT = 120.0
DOCKER_TIMEOUT = 300.0


class IsolationError(RuntimeError):
    pass


def socket_path(docker_host: str) -> Path:
    """The socket of a ``unix://`` Docker host; other transports are refused."""
    prefix = "unix://"
    if not docker_host.startswith(prefix):
        raise IsolationError(f"only unix:// Docker hosts are supported: {docker_host}")
    path = Path(docker_host[len(prefix) :])
    if not path.is_absolute():
        raise IsolationError(f"Docker socket path must be absolute: {docker_host}")
    return path


def docker_environment(docker_host: str | None) -> dict[str, str]:
    """The process environment addressing exactly one daemon."""
    environment = dict(os.environ)
    environment.pop("DOCKER_CONTEXT", None)
    if docker_host is None:
        environment.pop("DOCKER_HOST", None)
    else:
        environment["DOCKER_HOST"] = docker_host
    return environment


def parse_memory(value: str) -> int | None:
    """Bytes in a ``docker stats`` size such as ``1.5GiB`` or ``512kB``."""
    units = {
        "b": 1,
        "kb": 10**3,
        "mb": 10**6,
        "gb": 10**9,
        "tb": 10**12,
        "kib": 2**10,
        "mib": 2**20,
        "gib": 2**30,
        "tib": 2**40,
    }
    text = value.strip().lower()
    number = text.rstrip("abcdefghijklmnopqrstuvwxyz")
    unit = text[len(number) :].strip()
    if unit not in units:
        return None
    try:
        return int(float(number) * units[unit])
    except ValueError:
        return None


class Docker:
    """The ``docker`` CLI bound to one daemon."""

    def __init__(self, docker_host: str | None) -> None:
        self.docker_host = docker_host
        self.environment = docker_environment(docker_host)

    def run(
        self, *arguments: str, check: bool = True, timeout: float = DOCKER_TIMEOUT
    ) -> subprocess.CompletedProcess[str]:
        completed = subprocess.run(
            ["docker", *arguments],
            env=self.environment,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
        if check and completed.returncode != 0:
            raise IsolationError(
                f"docker {' '.join(arguments)} failed: {completed.stderr.strip()}"
            )
        return completed

    def daemon_id(self) -> str | None:
        try:
            completed = self.run("info", "--format", "{{.ID}}", check=False, timeout=30)
        except subprocess.TimeoutExpired:
            return None
        identifier = completed.stdout.strip()
        return identifier if completed.returncode == 0 and identifier else None

    def names(self, *, running_only: bool = False) -> list[str]:
        arguments = ["ps", "--format", "{{.Names}}"]
        if not running_only:
            arguments.insert(1, "--all")
        return sorted(self.run(*arguments).stdout.split())

    def inspect(self, name: str) -> dict[str, Any] | None:
        completed = self.run("inspect", name, check=False)
        if completed.returncode != 0:
            return None
        documents = json.loads(completed.stdout)
        return documents[0] if documents else None

    def memory_usage(self, *names: str) -> dict[str, int]:
        """Current memory of the named (default: all running) containers, in bytes."""
        completed = self.run(
            "stats",
            "--no-stream",
            "--format",
            "{{json .}}",
            *names,
            check=False,
            timeout=60,
        )
        usage: dict[str, int] = {}
        for line in completed.stdout.splitlines():
            row = json.loads(line)
            used = parse_memory(str(row.get("MemUsage", "")).split("/")[0])
            if used is not None:
                usage[str(row.get("Name"))] = used
        return usage

    def disk_usage(self) -> list[dict[str, Any]]:
        completed = self.run("system", "df", "--format", "{{json .}}", check=False)
        return [json.loads(line) for line in completed.stdout.splitlines() if line]


def require_isolated(docker_host: str) -> Docker:
    """A client for ``docker_host`` after proving it is not the default daemon."""
    if str(socket_path(docker_host)) in DEFAULT_DAEMON_SOCKETS:
        raise IsolationError(f"refusing the default Docker socket: {docker_host}")
    isolated = Docker(docker_host)
    isolated_id = isolated.daemon_id()
    if isolated_id is None:
        raise IsolationError(f"no Docker daemon answers at {docker_host}")
    if Docker(None).daemon_id() == isolated_id:
        raise IsolationError(f"{docker_host} is the default Docker daemon; refusing")
    return isolated


def _socket_group() -> int:
    for candidate in DEFAULT_DAEMON_SOCKETS:
        if Path(candidate).exists():
            return Path(candidate).stat().st_gid
    return os.getgid()


@dataclass
class Dind:
    """A privileged Docker-in-Docker daemon with its storage under ``root``."""

    name: str
    root: Path
    image: str

    @property
    def socket(self) -> Path:
        return self.root / "run" / "docker.sock"

    @property
    def docker_host(self) -> str:
        return f"unix://{self.socket}"

    def create(self, shared: Sequence[Path], empty: Sequence[Path], owner: str) -> None:
        """Starts the daemon; ``shared`` paths appear at identical paths inside it."""
        host = Docker(None)
        if host.inspect(self.name) is not None:
            raise IsolationError(f"container {self.name} already exists")
        storage = self.root / "var-lib-docker"
        run_directory = self.root / "run"
        storage.mkdir(parents=True, exist_ok=False)
        run_directory.mkdir(parents=True, exist_ok=True)
        volumes = [f"{storage}:/var/lib/docker", f"{run_directory}:/var/run/dind"]
        volumes += [f"{path}:{path}" for path in shared]
        for index, target in enumerate(empty):
            placeholder = self.root / "empty" / str(index)
            placeholder.mkdir(parents=True, exist_ok=True)
            volumes.append(f"{placeholder}:{target}")
        arguments = ["run", "--detach", "--privileged", "--name", self.name]
        arguments += [
            "--label",
            f"{OWNER_LABEL}={owner}",
            "--env",
            "DOCKER_TLS_CERTDIR=",
        ]
        for volume in volumes:
            arguments += ["--volume", volume]
        arguments += [
            self.image,
            "dockerd",
            "--host=unix:///var/run/docker.sock",
            "--host=unix:///var/run/dind/docker.sock",
            "--group",
            str(_socket_group()),
        ]
        host.run(*arguments)
        deadline = time.monotonic() + DIND_READY_TIMEOUT
        while Docker(self.docker_host).daemon_id() is None:
            if time.monotonic() > deadline:
                raise IsolationError(f"{self.name} did not start its daemon")
            time.sleep(1)

    def owner(self) -> str | None:
        info = Docker(None).inspect(self.name)
        if info is None:
            return None
        return (info.get("Config", {}).get("Labels") or {}).get(OWNER_LABEL)

    def remove(self, owner: str) -> None:
        """Removes a daemon this harness created, and its root-owned storage."""
        if self.owner() != owner:
            raise IsolationError(
                f"{self.name} was not created by this run; not removing"
            )
        host = Docker(None)
        host.run("rm", "--force", "--volumes", self.name)
        remove_as_root(self.image, [self.root / "var-lib-docker", self.root / "run"])
        shutil.rmtree(self.root, ignore_errors=True)


def parse_seed(text: str) -> tuple[str, str]:
    """``SOURCE=TARGET`` image references for :func:`seed_image`."""
    source, separator, target = text.partition("=")
    if not separator or not source or not target:
        raise IsolationError(f"seed must be SOURCE=TARGET image references: {text!r}")
    return source, target


def seed_image(target: Docker, source: str, reference: str) -> str:
    """Copies an image from the default daemon, read-only, into ``target``."""
    saver = subprocess.Popen(
        ["docker", "save", source],
        env=docker_environment(None),
        stdout=subprocess.PIPE,
    )
    assert saver.stdout is not None
    loader = subprocess.run(
        ["docker", "load", "--quiet"],
        env=target.environment,
        stdin=saver.stdout,
        capture_output=True,
        text=True,
        check=False,
    )
    saver.stdout.close()
    if saver.wait() != 0 or loader.returncode != 0:
        raise IsolationError(f"could not seed {source}: {loader.stderr.strip()}")
    if source != reference:
        target.run("tag", source, reference)
        # Only the seeded reference may exist, as it would on a fresh machine.
        target.run("image", "rm", source)
    return target.run(
        "image", "inspect", "--format", "{{.Id}}", reference
    ).stdout.strip()


def daemon_memory(container: str) -> int | None:
    """Memory of a daemon container on the host, nested containers included."""
    return Docker(None).memory_usage(container).get(container)


def daemon_storage(container: str) -> int | None:
    completed = Docker(None).run(
        "exec", container, "du", "-sxb", "/var/lib/docker", check=False, timeout=3600
    )
    fields = completed.stdout.split()
    return int(fields[0]) if completed.returncode == 0 and fields else None


def daemon_address(container: str) -> str | None:
    """The daemon container's address on the host network, for published ports."""
    info = Docker(None).inspect(container) or {}
    for network in info.get("NetworkSettings", {}).get("Networks", {}).values():
        if network.get("IPAddress"):
            return str(network["IPAddress"])
    return None


def remove_as_root(image: str, paths: Sequence[Path]) -> None:
    """Deletes container-created files through a throwaway container."""
    existing = [path for path in paths if path.exists()]
    if not existing:
        return
    arguments = ["run", "--rm", "--entrypoint", "rm"]
    for path in existing:
        arguments += ["--volume", f"{path.parent}:{path.parent}"]
    Docker(None).run(*arguments, image, "-rf", *map(str, existing), timeout=3600)


def _git(*arguments: str, cwd: Path, stdin: bytes | None = None) -> bytes:
    completed = subprocess.run(
        ["git", *arguments], cwd=cwd, input=stdin, capture_output=True, check=False
    )
    if completed.returncode != 0:
        message = completed.stderr.decode(errors="replace").strip()
        raise IsolationError(f"git {' '.join(arguments)} failed in {cwd}: {message}")
    return completed.stdout


def materialize_checkout(source: Path, destination: Path) -> dict[str, Any]:
    """A fresh clone of ``source`` with its uncommitted changes and no build outputs.

    This is what a new developer has before the first container start: the same
    sources, submodules included, without target directories or node_modules.
    """
    commit = _git("rev-parse", "HEAD", cwd=source).decode().strip()
    destination.parent.mkdir(parents=True, exist_ok=True)
    _git("clone", "--quiet", "--no-checkout", str(source), str(destination), cwd=source)
    _git("checkout", "--quiet", "--detach", commit, cwd=destination)
    submodules = []
    listing = _git("submodule", "status", cwd=source).decode()
    for line in listing.splitlines():
        fields = line.strip().split()
        if len(fields) < 2 or line.startswith("-"):
            continue
        path = fields[1]
        _git("submodule", "init", path, cwd=destination)
        _git("config", f"submodule.{path}.url", str(source / path), cwd=destination)
        # Local clones of submodules need the file transport, off by default.
        _git(
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
            "--quiet",
            path,
            cwd=destination,
        )
        _git("submodule", "sync", "--quiet", path, cwd=destination)
        submodules.append(path)
    diff = _git("diff", "--binary", "HEAD", cwd=source)
    if diff:
        _git("apply", "--binary", "-", cwd=destination, stdin=diff)
    untracked = [
        name
        for name in _git("ls-files", "--others", "--exclude-standard", "-z", cwd=source)
        .decode()
        .split("\0")
        if name
    ]
    for name in untracked:
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / name, target)
    return {
        "source": str(source),
        "copy": str(destination),
        "commit": commit,
        "submodules": submodules,
        "diff_sha256": hashlib.sha256(diff).hexdigest() if diff else None,
        "untracked_files": len(untracked),
    }


def read_env_file(path: Path) -> dict[str, str]:
    """``KEY=value`` lines of a dotenv file, the last definition winning."""
    values: dict[str, str] = {}
    if not path.exists():
        return values
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.removeprefix("export ").strip()
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
            value = value[1:-1]
        values[key] = value
    return values
