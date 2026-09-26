# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The mode manifest, ``.devcontainer/modes.json``."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any

MANIFEST_FILE = Path(".devcontainer/modes.json")
DEVCONTAINER_SERVICE = "devcontainer"


class ManifestError(ValueError):
    """The manifest is malformed or refers to something it does not define."""


@dataclass(frozen=True)
class Server:
    """A dev server run inside the devcontainer from the repository root."""

    name: str
    port: int
    command: tuple[str, ...]
    ready_path: str


class ReadyWhen(Enum):
    """What a service must reach to count as up."""

    RUNNING = "running"
    # A job such as a volume or bucket initializer: done once it exits with 0.
    EXITED = "exited"


@dataclass(frozen=True)
class ServiceSettings:
    """Where a Compose service is reached from the host, and when it is ready."""

    name: str
    url: str | None = None
    # Run inside the service's container; exit status 0 means ready.
    probe: tuple[str, ...] | None = None
    ready_when: ReadyWhen = ReadyWhen.RUNNING


@dataclass(frozen=True)
class Mode:
    name: str
    summary: str
    # devcontainer.json path, relative to .devcontainer.
    config: str
    compose_files: tuple[str, ...]
    services: tuple[str, ...]
    servers: tuple[str, ...]
    default_servers: tuple[str, ...]
    ready_timeout: float


@dataclass(frozen=True)
class Manifest:
    modes: tuple[Mode, ...]
    servers: dict[str, Server] = field(default_factory=dict)
    services: dict[str, ServiceSettings] = field(default_factory=dict)

    def mode_names(self) -> list[str]:
        return [mode.name for mode in self.modes]

    def mode(self, name: str) -> Mode:
        for mode in self.modes:
            if mode.name == name:
                return mode
        raise ManifestError(
            f"unknown mode {name!r}; choose one of {', '.join(self.mode_names())}"
        )

    def settings(self, service: str) -> ServiceSettings:
        return self.services.get(service, ServiceSettings(service))


def _field(entry: dict[str, Any], key: str, kind: type, where: str) -> Any:
    if key not in entry:
        raise ManifestError(f"{where}: missing {key!r}")
    value = entry[key]
    if not isinstance(value, kind) or isinstance(value, bool):
        raise ManifestError(f"{where}: {key!r} must be a {kind.__name__}")
    return value


def _strings(entry: dict[str, Any], key: str, where: str) -> tuple[str, ...]:
    values = _field(entry, key, list, where)
    if not all(isinstance(value, str) and value for value in values):
        raise ManifestError(f"{where}: {key!r} must list non-empty strings")
    if len(set(values)) != len(values):
        raise ManifestError(f"{where}: {key!r} repeats an entry")
    return tuple(values)


def _unique(names: list[str], what: str) -> None:
    seen: set[str] = set()
    for name in names:
        if name in seen:
            raise ManifestError(f"{what} {name!r} is defined twice")
        seen.add(name)


def _server(entry: dict[str, Any]) -> Server:
    name = _field(entry, "name", str, "server")
    where = f"server {name!r}"
    port = _field(entry, "port", int, where)
    if not 0 < port < 65536:
        raise ManifestError(f"{where}: port {port} is out of range")
    ready_path = _field(entry, "readyPath", str, where)
    if not ready_path.startswith("/"):
        raise ManifestError(f"{where}: readyPath must start with '/'")
    command = _strings(entry, "command", where)
    if not command:
        raise ManifestError(f"{where}: empty command")
    return Server(name, port, command, ready_path)


def _service(entry: dict[str, Any]) -> ServiceSettings:
    name = _field(entry, "name", str, "service")
    where = f"service {name!r}"
    url = _field(entry, "url", str, where) if "url" in entry else None
    probe = _strings(entry, "probe", where) if "probe" in entry else None
    if probe == ():
        raise ManifestError(f"{where}: empty probe")
    ready_when = (
        _field(entry, "readyWhen", str, where) if "readyWhen" in entry else None
    )
    try:
        when = ReadyWhen(ready_when) if ready_when else ReadyWhen.RUNNING
    except ValueError:
        choices = ", ".join(value.value for value in ReadyWhen)
        raise ManifestError(f"{where}: readyWhen must be one of {choices}") from None
    if probe is not None and when is ReadyWhen.EXITED:
        raise ManifestError(f"{where}: a job that exits cannot be probed")
    return ServiceSettings(name, url, probe, when)


def _mode(entry: dict[str, Any], servers: dict[str, Server]) -> Mode:
    name = _field(entry, "name", str, "mode")
    where = f"mode {name!r}"
    services = _strings(entry, "services", where)
    if DEVCONTAINER_SERVICE not in services:
        raise ManifestError(f"{where}: services must include {DEVCONTAINER_SERVICE!r}")
    mode_servers = _strings(entry, "servers", where)
    unknown = [server for server in mode_servers if server not in servers]
    if unknown:
        raise ManifestError(f"{where}: undefined servers {', '.join(unknown)}")
    default_servers = _strings(entry, "defaultServers", where)
    stray = [server for server in default_servers if server not in mode_servers]
    if stray:
        raise ManifestError(f"{where}: default servers {', '.join(stray)} not listed")
    timeout = _field(entry, "readyTimeoutSeconds", int, where)
    if timeout <= 0:
        raise ManifestError(f"{where}: readyTimeoutSeconds must be positive")
    compose_files = _strings(entry, "composeFiles", where)
    if not compose_files:
        raise ManifestError(f"{where}: no compose files")
    return Mode(
        name=name,
        summary=_field(entry, "summary", str, where),
        config=_field(entry, "config", str, where),
        compose_files=compose_files,
        services=services,
        servers=mode_servers,
        default_servers=default_servers,
        ready_timeout=float(timeout),
    )


def parse_manifest(document: Any) -> Manifest:
    if not isinstance(document, dict):
        raise ManifestError("the manifest must be a JSON object")
    server_entries = _field(document, "servers", list, "manifest")
    servers = [_server(entry) for entry in server_entries]
    _unique([server.name for server in servers], "server")
    _unique([str(server.port) for server in servers], "server port")
    server_map = {server.name: server for server in servers}
    service_entries = _field(document, "services", list, "manifest")
    services = [_service(entry) for entry in service_entries]
    _unique([service.name for service in services], "service")
    mode_entries = _field(document, "modes", list, "manifest")
    modes = tuple(_mode(entry, server_map) for entry in mode_entries)
    if not modes:
        raise ManifestError("the manifest defines no mode")
    _unique([mode.name for mode in modes], "mode")
    return Manifest(modes, server_map, {service.name: service for service in services})


def load_manifest(root: Path) -> Manifest:
    path = root / MANIFEST_FILE
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ManifestError(f"cannot read {path}: {error}") from error
    return parse_manifest(document)
