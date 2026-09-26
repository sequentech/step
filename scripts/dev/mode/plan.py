# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""What a mode starts, what stands in its way and when its services are ready."""

from __future__ import annotations

import os
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from enum import Enum
from typing import Any

from .docker import ContainerState, ContainerSummary, PortBinding


class PlanError(ValueError):
    """The mode refers to services the Compose files do not define."""


def dependencies(service: Mapping[str, Any]) -> list[str]:
    """Services started before this one: depends_on and volumes_from."""
    names = list(service.get("depends_on") or {})
    for entry in service.get("volumes_from") or []:
        kind, _, rest = str(entry).partition(":")
        if kind == "container":
            continue
        name = rest.partition(":")[0] if kind == "service" else kind
        if name not in names:
            names.append(name)
    return names


def closure(
    services: Mapping[str, Mapping[str, Any]], requested: Iterable[str]
) -> list[str]:
    """The requested services and everything they need, dependencies first."""
    ordered: list[str] = []
    visiting: set[str] = set()

    def visit(name: str, needed_by: str | None) -> None:
        if name in ordered or name in visiting:
            return
        if name not in services:
            reason = f" (needed by {needed_by})" if needed_by else ""
            raise PlanError(f"service {name!r}{reason} is not in the Compose files")
        visiting.add(name)
        for dependency in dependencies(services[name]):
            visit(dependency, name)
        visiting.discard(name)
        ordered.append(name)

    for name in requested:
        visit(name, None)
    return ordered


def published_ports(service: Mapping[str, Any]) -> list[PortBinding]:
    bindings = []
    for entry in service.get("ports") or []:
        published = str(entry.get("published") or "")
        if not published:
            continue
        first, _, last = published.partition("-")
        for port in range(int(first), int(last or first) + 1):
            bindings.append(
                PortBinding(
                    entry.get("host_ip") or "", port, entry.get("protocol") or "tcp"
                )
            )
    return bindings


class ConflictKind(Enum):
    CONTAINER_NAME = "container name"
    HOST_PORT = "host port"
    CHECKOUT = "checkout"


@dataclass(frozen=True)
class Conflict:
    kind: ConflictKind
    service: str
    detail: str
    resolution: str

    def __str__(self) -> str:
        return f"{self.service}: {self.kind.value} {self.detail}; {self.resolution}"


def _owner(container: ContainerSummary) -> str:
    if container.project:
        return f"container {container.name} of Compose project {container.project}"
    return f"container {container.name}"


def _stop_hint(container: ContainerSummary) -> str:
    if container.project:
        return f"stop that project (docker compose -p {container.project} stop)"
    return f"stop that container (docker stop {container.name})"


def find_conflicts(
    project: str,
    host_folder: str,
    services: Mapping[str, Mapping[str, Any]],
    names: Sequence[str],
    containers: Sequence[ContainerSummary],
    devcontainer: ContainerState | None,
) -> list[Conflict]:
    """Why ``names`` cannot start in ``project`` next to ``containers``.

    Two checkouts with the same folder name derive the same project; the Dev
    Containers CLI labels each devcontainer with the folder it serves.
    """
    conflicts = []
    if (
        devcontainer is not None
        and devcontainer.local_folder
        and os.path.realpath(devcontainer.local_folder) != os.path.realpath(host_folder)
    ):
        conflicts.append(
            Conflict(
                ConflictKind.CHECKOUT,
                devcontainer.service or "devcontainer",
                f"project {project} belongs to {devcontainer.local_folder}",
                "rename one of the two checkout folders",
            )
        )
    by_name = {container.name: container for container in containers}
    others = [
        container
        for container in containers
        if container.project != project and container.state == "running"
    ]
    for name in names:
        service = services[name]
        container_name = service.get("container_name")
        holder = by_name.get(container_name) if container_name else None
        if holder is not None and holder.project != project:
            conflicts.append(
                Conflict(
                    ConflictKind.CONTAINER_NAME,
                    name,
                    f"{container_name} is taken by {_owner(holder)}",
                    _stop_hint(holder) + " and remove it",
                )
            )
        for binding in published_ports(service):
            for container in others:
                if any(binding.overlaps(used) for used in container.ports):
                    conflicts.append(
                        Conflict(
                            ConflictKind.HOST_PORT,
                            name,
                            f"{binding} is published by {_owner(container)}",
                            _stop_hint(container) + " or use the ui-only mode here",
                        )
                    )
    return conflicts


class Readiness(Enum):
    MISSING = "missing"
    STARTING = "starting"
    READY = "ready"
    COMPLETED = "completed"
    FAILED = "failed"

    @property
    def done(self) -> bool:
        return self in (Readiness.READY, Readiness.COMPLETED)


def readiness(
    state: ContainerState | None, probe_ok: bool | None
) -> tuple[Readiness, str]:
    """A container's readiness from its state, health check and probe.

    ``probe_ok`` is None when the service has no probe or it was not run.
    """
    if state is None:
        return Readiness.MISSING, "not created"
    if state.status == "running":
        if state.health is not None and state.health != "healthy":
            return Readiness.STARTING, state.health
        if probe_ok is False:
            return Readiness.STARTING, "probe failing"
        return Readiness.READY, state.health or "running"
    if state.status in ("exited", "restarting") and state.exit_code == 0:
        # A one-shot job such as a volume or bucket initializer.
        return Readiness.COMPLETED, "completed"
    if state.status == "restarting":
        return Readiness.STARTING, f"restarting after exit code {state.exit_code}"
    if state.status in ("exited", "dead"):
        return Readiness.FAILED, f"exited with code {state.exit_code}"
    return Readiness.STARTING, state.status
