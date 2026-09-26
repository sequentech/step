# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Readiness probes for a started workspace: ``name=kind:arguments``.

A name ending in ``?`` marks an optional probe: it is recorded when it passes
but readiness does not wait for it.

Kinds:
  http:URL                  2xx from the harness process
  exec:CONTAINER:URL        2xx from curl inside a container of the stack
  healthy:CONTAINER|*       Docker health status (``*``: every container with one)
  running:CONTAINER         the container is running
  log:CONTAINER:REGEX       a log line since the sample started matches

``{env:NAME}`` expands from the checkout's ``.devcontainer/.env`` and
``{daemon_ip}`` to the isolated daemon's address on the host network.
"""

from __future__ import annotations

import re
import subprocess
import sys
import threading
import time
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from enum import Enum
from typing import Any

from .isolation import Docker, IsolationError
from .process import http_status

PLACEHOLDER = re.compile(r"\{(env:[A-Za-z_][A-Za-z0-9_]*|daemon_ip)\}")
CURL_TIMEOUT_SECONDS = 5


class ProbeKind(Enum):
    HTTP = "http"
    EXEC = "exec"
    HEALTHY = "healthy"
    RUNNING = "running"
    LOG = "log"


class ProbeError(ValueError):
    pass


@dataclass(frozen=True)
class Probe:
    name: str
    kind: ProbeKind
    container: str | None = None
    argument: str | None = None
    required: bool = True

    def __str__(self) -> str:
        parts = [self.kind.value, self.container, self.argument]
        suffix = "" if self.required else "?"
        return f"{self.name}{suffix}=" + ":".join(
            part for part in parts if part is not None
        )


# The repository's default ("base" profile) stack: compose health, the GraphQL
# engine, the imported tenant realm and each Rust service's own readiness probe.
# Beat is optional: without AWS_S3_PUBLIC_BUCKET in its compose environment it
# panics while loading plugins, so it may never become ready.
PRESETS: dict[str, tuple[str, ...]] = {
    "devcontainer": (),
    "full-stack": (
        "compose-healthy=healthy:*",
        "hasura=exec:devcontainer:http://graphql-engine:8080/healthz",
        "keycloak-realm=exec:devcontainer:http://keycloak:8090/realms/"
        "tenant-{env:SUPER_ADMIN_TENANT_ID}/.well-known/openid-configuration",
        "harvest=exec:harvest:http://127.0.0.1:3030/ready",
        "windmill-running=log:windmill:Running `[^`]*/debug/main",
        "windmill=exec:windmill:http://127.0.0.1:3030/ready",
        "beat?=exec:beat:http://127.0.0.1:3030/ready",
    ),
}


def parse_probe(text: str) -> Probe:
    name, separator, specification = text.partition("=")
    required = not name.endswith("?")
    name = name.removesuffix("?")
    if not separator or not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", name):
        raise ProbeError(
            f"probe must be NAME=KIND:ARGUMENTS with a slug name: {text!r}"
        )
    kind_text, _, rest = specification.partition(":")
    try:
        kind = ProbeKind(kind_text)
    except ValueError:
        choices = ", ".join(kind.value for kind in ProbeKind)
        raise ProbeError(
            f"unknown probe kind {kind_text!r} (choose {choices})"
        ) from None
    if not rest:
        raise ProbeError(f"probe {name} needs arguments")
    if kind is ProbeKind.HTTP:
        return Probe(name, kind, argument=rest, required=required)
    if kind in (ProbeKind.HEALTHY, ProbeKind.RUNNING):
        return Probe(name, kind, container=rest, required=required)
    container, _, argument = rest.partition(":")
    if not container or not argument:
        raise ProbeError(f"probe {name} needs CONTAINER:{kind.value.upper()} arguments")
    if kind is ProbeKind.LOG:
        try:
            re.compile(argument)
        except re.error as error:
            raise ProbeError(
                f"probe {name}: invalid regular expression: {error}"
            ) from None
    return Probe(name, kind, container=container, argument=argument, required=required)


def resolve_probes(target: str, extra: Sequence[str]) -> list[Probe]:
    """The target's preset, with ``extra`` probes added or replacing by name."""
    if target not in PRESETS:
        raise ProbeError(f"unknown target {target!r} (choose {', '.join(PRESETS)})")
    probes = {probe.name: probe for probe in map(parse_probe, PRESETS[target])}
    for probe in map(parse_probe, extra):
        probes[probe.name] = probe
    return list(probes.values())


def expand(text: str, env: Mapping[str, str], daemon_ip: str | None) -> str:
    def replace(match: re.Match[str]) -> str:
        key = match.group(1)
        if key == "daemon_ip":
            if daemon_ip is None:
                raise ProbeError("{daemon_ip} needs a daemon created by the harness")
            return daemon_ip
        name = key.removeprefix("env:")
        if name not in env:
            raise ProbeError(
                f"{name} is not defined in the checkout's .devcontainer/.env"
            )
        return env[name]

    return PLACEHOLDER.sub(replace, text)


def all_healthy(containers: Sequence[Mapping[str, Any]]) -> bool:
    """Every container defining a health check runs and is healthy (at least one)."""
    checked = [
        container
        for container in containers
        if (container.get("Config", {}).get("Healthcheck") or {}).get("Test")
        not in (None, [], ["NONE"])
    ]
    return bool(checked) and all(
        container.get("State", {}).get("Running")
        and (container["State"].get("Health") or {}).get("Status") == "healthy"
        for container in checked
    )


class ProbeRunner:
    """Polls every probe in its own thread and records its first success."""

    def __init__(
        self,
        docker: Docker,
        probes: Sequence[Probe],
        env: Mapping[str, str],
        daemon_ip: str | None,
        interval: float,
    ) -> None:
        self.docker = docker
        self.probes = list(probes)
        self.interval = interval
        self.arguments = {
            probe.name: None
            if probe.argument is None
            else expand(probe.argument, env, daemon_ip)
            for probe in self.probes
        }
        self.passed: dict[str, float] = {}
        # Compose creates containers progressively; "every container" is only
        # meaningful once the devcontainer CLI has brought the whole stack up.
        self.stack_created = threading.Event()
        self._stop = threading.Event()
        self._threads: list[threading.Thread] = []
        self._start = 0.0
        self._since = ""

    def start(self, started_monotonic: float) -> None:
        self._start = started_monotonic
        self._since = datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")
        for probe in self.probes:
            thread = threading.Thread(target=self._poll, args=(probe,), daemon=True)
            thread.start()
            self._threads.append(thread)

    def ready(self) -> bool:
        """Every required probe has passed."""
        return all(probe.name in self.passed for probe in self.probes if probe.required)

    def stop(self) -> None:
        self._stop.set()
        for thread in self._threads:
            thread.join(CURL_TIMEOUT_SECONDS * 2)

    def pending(self) -> list[str]:
        return [
            probe.name
            for probe in self.probes
            if probe.required and probe.name not in self.passed
        ]

    def _poll(self, probe: Probe) -> None:
        while not self._stop.is_set():
            try:
                passed = self.check(probe)
            except (OSError, subprocess.SubprocessError, ValueError, IsolationError):
                passed = False
            if passed:
                seconds = time.monotonic() - self._start
                self.passed[probe.name] = seconds
                print(
                    f"bench: probe {probe.name} passed at {seconds:.1f}s",
                    file=sys.stderr,
                    flush=True,
                )
                return
            self._stop.wait(self.interval)

    def check(self, probe: Probe) -> bool:
        argument = self.arguments[probe.name]
        if probe.kind is ProbeKind.HTTP:
            status = http_status(str(argument), timeout=CURL_TIMEOUT_SECONDS)
            return status is not None and 200 <= status < 300
        if probe.kind is ProbeKind.EXEC:
            completed = self.docker.run(
                "exec",
                str(probe.container),
                "curl",
                "--silent",
                "--fail",
                "--output",
                "/dev/null",
                "--max-time",
                str(CURL_TIMEOUT_SECONDS),
                str(argument),
                check=False,
                timeout=CURL_TIMEOUT_SECONDS * 3,
            )
            return completed.returncode == 0
        if probe.kind is ProbeKind.LOG:
            completed = self.docker.run(
                "logs", "--since", self._since, str(probe.container), check=False
            )
            text = completed.stdout + completed.stderr
            return re.search(str(argument), strip_ansi(text)) is not None
        if probe.container == "*":
            if not self.stack_created.is_set():
                return False
            names = self.docker.names()
            return all_healthy([self.docker.inspect(name) or {} for name in names])
        info = self.docker.inspect(str(probe.container)) or {}
        if probe.kind is ProbeKind.RUNNING:
            return bool(info.get("State", {}).get("Running"))
        return all_healthy([info])


ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def strip_ansi(text: str) -> str:
    return ANSI_ESCAPE.sub("", text)
