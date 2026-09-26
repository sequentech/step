# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Docker and Compose calls, and the containers they report."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from typing import Any

from .checkout import Checkout

PROJECT_LABEL = "com.docker.compose.project"
SERVICE_LABEL = "com.docker.compose.service"
# Set by the Dev Containers CLI on the container it creates for a checkout.
LOCAL_FOLDER_LABEL = "devcontainer.local_folder"
PS_FORMAT = "\t".join(
    (
        "{{.ID}}",
        "{{.Names}}",
        "{{.State}}",
        f'{{{{.Label "{PROJECT_LABEL}"}}}}',
        f'{{{{.Label "{SERVICE_LABEL}"}}}}',
        "{{.Ports}}",
    )
)
PS_FIELDS = 6
# "0.0.0.0:8090->8090/tcp", "[::]:3000-3001->3000-3001/tcp"; unpublished
# ports ("5432/tcp") carry no arrow and are skipped.
PORT_PATTERN = re.compile(
    r"(?:(?P<ip>\[[^\]]*\]|[0-9.]+):)?(?P<first>\d+)(?:-(?P<last>\d+))?->[\d-]+/(?P<proto>\w+)"
)
ANY_ADDRESSES = frozenset({"", "0.0.0.0", "::", "[::]"})
PROBE_TIMEOUT_SECONDS = 20


class DockerError(RuntimeError):
    """A Docker command failed or produced output it should not."""


def docker(
    args: Sequence[str],
    *,
    env: dict[str, str] | None = None,
    timeout: float | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    try:
        completed = subprocess.run(
            ["docker", *args],
            capture_output=True,
            text=True,
            env=env,
            timeout=timeout,
            check=False,
        )
    except FileNotFoundError as error:
        raise DockerError("docker is not on PATH") from error
    if check and completed.returncode != 0:
        detail = (completed.stderr or completed.stdout).strip()
        raise DockerError(f"docker {' '.join(args[:2])} failed: {detail}")
    return completed


@dataclass(frozen=True)
class PortBinding:
    host_ip: str
    port: int
    protocol: str

    def overlaps(self, other: PortBinding) -> bool:
        if self.port != other.port or self.protocol != other.protocol:
            return False
        return (
            self.host_ip in ANY_ADDRESSES
            or other.host_ip in ANY_ADDRESSES
            or self.host_ip == other.host_ip
        )

    def __str__(self) -> str:
        address = "" if self.host_ip in ANY_ADDRESSES else f"{self.host_ip}:"
        return f"{address}{self.port}/{self.protocol}"


def parse_ports(text: str) -> tuple[PortBinding, ...]:
    bindings: list[PortBinding] = []
    for match in PORT_PATTERN.finditer(text):
        first = int(match.group("first"))
        last = int(match.group("last") or first)
        address = (match.group("ip") or "").strip("[]") or ""
        for port in range(first, last + 1):
            binding = PortBinding(address, port, match.group("proto"))
            if binding not in bindings:
                bindings.append(binding)
    return tuple(bindings)


@dataclass(frozen=True)
class ContainerSummary:
    id: str
    name: str
    state: str
    project: str
    service: str
    ports: tuple[PortBinding, ...]


def parse_ps(text: str) -> list[ContainerSummary]:
    containers = []
    for line in text.splitlines():
        if not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) != PS_FIELDS:
            raise DockerError(f"unexpected docker ps line: {line!r}")
        identifier, name, state, project, service, ports = fields
        containers.append(
            ContainerSummary(
                identifier, name, state, project, service, parse_ports(ports)
            )
        )
    return containers


def list_containers() -> list[ContainerSummary]:
    return parse_ps(docker(["ps", "--all", "--no-trunc", "--format", PS_FORMAT]).stdout)


@dataclass(frozen=True)
class ContainerState:
    id: str
    name: str
    service: str
    status: str
    health: str | None
    exit_code: int
    restart_policy: str
    local_folder: str | None
    # The health check test the container was created with, if any.
    healthcheck: tuple[str, ...] | None = None


def container_state(document: dict[str, Any]) -> ContainerState:
    state = document.get("State") or {}
    labels = (document.get("Config") or {}).get("Labels") or {}
    health = state.get("Health") or {}
    policy = ((document.get("HostConfig") or {}).get("RestartPolicy") or {}).get("Name")
    test = ((document.get("Config") or {}).get("Healthcheck") or {}).get("Test")
    return ContainerState(
        id=document.get("Id", ""),
        name=document.get("Name", "").lstrip("/"),
        service=labels.get(SERVICE_LABEL, ""),
        status=state.get("Status", ""),
        health=health.get("Status") or None,
        exit_code=int(state.get("ExitCode") or 0),
        restart_policy=policy or "no",
        local_folder=labels.get(LOCAL_FOLDER_LABEL),
        healthcheck=tuple(test) if test else None,
    )


def inspect_containers(identifiers: Iterable[str]) -> list[ContainerState]:
    identifiers = list(identifiers)
    if not identifiers:
        return []
    documents = json.loads(docker(["inspect", *identifiers]).stdout)
    return [container_state(document) for document in documents]


def project_states(project: str) -> dict[str, ContainerState]:
    """The project's containers by service; one per service in this stack."""
    listed = docker(
        [
            "ps",
            "--all",
            "--quiet",
            "--no-trunc",
            "--filter",
            f"label={PROJECT_LABEL}={project}",
        ]
    ).stdout.split()
    return {state.service: state for state in inspect_containers(listed)}


def probe(container: str, command: Sequence[str]) -> bool:
    try:
        completed = docker(
            ["exec", container, *command], timeout=PROBE_TIMEOUT_SECONDS, check=False
        )
    except subprocess.TimeoutExpired:
        return False
    return completed.returncode == 0


def ensure_volumes(names: Iterable[str]) -> None:
    for name in names:
        docker(["volume", "create", name])


@dataclass(frozen=True)
class Compose:
    """``docker compose`` for one checkout and one set of Compose files."""

    checkout: Checkout
    files: tuple[str, ...]

    def environment(self) -> dict[str, str]:
        # The checkout's .env decides interpolation and the project, not values
        # exported by a shell that may have loaded an older .env.
        return {
            key: value
            for key, value in os.environ.items()
            if key not in self.checkout.env and not key.startswith("COMPOSE_")
        }

    def argv(self, *args: str) -> list[str]:
        directory = self.checkout.compose_dir
        command = [
            "compose",
            "--project-name",
            self.checkout.project,
            "--project-directory",
            str(directory),
        ]
        for name in self.files:
            command += ["--file", str(directory / name)]
        return [*command, *args]

    def config(self) -> dict[str, Any]:
        completed = docker(
            self.argv("--profile", "*", "config", "--format", "json"),
            env=self.environment(),
        )
        try:
            document = json.loads(completed.stdout)
        except json.JSONDecodeError as error:
            raise DockerError(
                f"docker compose config printed no JSON: {error}"
            ) from error
        return document.get("services") or {}

    def run(self, *args: str) -> int:
        """Runs with all output on stderr, where the user sees progress."""
        try:
            return subprocess.run(
                ["docker", *self.argv(*args)],
                env=self.environment(),
                stdout=sys.stderr,
                check=False,
            ).returncode
        except FileNotFoundError as error:
            raise DockerError("docker is not on PATH") from error
