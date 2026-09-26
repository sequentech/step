# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""``step-dev mode``: see, start, switch and stop devcontainer modes."""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections.abc import Sequence
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Any

from .checkout import REPOSITORY_ROOT, Checkout, CheckoutError, load_checkout
from .docker import (
    Compose,
    ContainerState,
    DockerError,
    docker,
    ensure_volumes,
    list_containers,
    probe,
    project_states,
)
from .manifest import (
    DEVCONTAINER_SERVICE,
    Manifest,
    ManifestError,
    Mode,
    Server,
    load_manifest,
)
from .plan import Conflict, PlanError, Readiness, closure, find_conflicts, readiness
from .servers import Devcontainer, ServerState, Started

POLL_SECONDS = 2.0
PROGRESS_SECONDS = 30.0
EXIT_OK = 0
EXIT_FAILED = 1
ACTIVE_STATUSES = frozenset({"running", "restarting"})
DEFAULT_SERVERS = "default"
NO_SERVERS = "none"
# Every dev server runs from the Yarn workspace.
NODE_MODULES = Path("packages/node_modules")


class OutputFormat(Enum):
    TEXT = "text"
    JSON = "json"


class ModeError(RuntimeError):
    """The command cannot proceed; the message says what to do instead."""


def _say(message: str = "") -> None:
    print(message, flush=True)


@dataclass
class Context:
    checkout: Checkout
    manifest: Manifest
    output: OutputFormat

    def say(self, message: str = "") -> None:
        # JSON output keeps stdout parseable; progress goes to stderr then.
        if self.output is OutputFormat.JSON:
            print(message, file=sys.stderr, flush=True)
        else:
            _say(message)


def _devcontainer_hint(checkout: Checkout, mode: Mode) -> str:
    config = f".devcontainer/{mode.config}"
    return (
        f"open the checkout with {config} first (VS Code: Dev Containers: Reopen in "
        f"Container; CLI: devcontainer up --workspace-folder {checkout.host_root} "
        f"--config {checkout.host_root / config})"
    )


def _selected_servers(manifest: Manifest, mode: Mode, spec: str) -> list[Server]:
    if spec == DEFAULT_SERVERS:
        names = list(mode.default_servers)
    elif spec == NO_SERVERS:
        names = []
    else:
        names = [name.strip() for name in spec.split(",") if name.strip()]
        unknown = [name for name in names if name not in mode.servers]
        if unknown:
            raise ModeError(
                f"{', '.join(unknown)} not in mode {mode.name}; its servers are "
                f"{', '.join(mode.servers) or 'none'}"
            )
    return [manifest.servers[name] for name in names]


@dataclass
class Plan:
    mode: Mode
    compose: Compose
    services: list[str]
    config: dict[str, Any]
    states: dict[str, ContainerState]
    conflicts: list[Conflict]


def _plan(context: Context, mode: Mode) -> Plan:
    compose = Compose(context.checkout, mode.compose_files)
    config = compose.config()
    services = closure(config, mode.services)
    states = project_states(context.checkout.project)
    conflicts = find_conflicts(
        context.checkout.project,
        str(context.checkout.host_root),
        config,
        services,
        list_containers(),
        states.get(DEVCONTAINER_SERVICE),
    )
    return Plan(mode, compose, services, config, states, conflicts)


def _report_conflicts(context: Context, plan: Plan) -> None:
    context.say(f"mode {plan.mode.name} cannot start in this checkout:")
    for conflict in plan.conflicts:
        context.say(f"  {conflict}")


def _probe_ok(context: Context, service: str, state: ContainerState) -> bool | None:
    command = context.manifest.settings(service).probe
    if command is None or state.status != "running":
        return None
    if state.health is not None and state.health != "healthy":
        return None
    return probe(state.id, command)


def _wait_services(
    context: Context, plan: Plan, timeout: float, started: float
) -> dict[str, dict[str, Any]]:
    """Waits until every service is ready; raises if one fails or time runs out."""
    pending = list(plan.services)
    results: dict[str, dict[str, Any]] = {}
    details: dict[str, str] = {}
    deadline = started + timeout
    next_progress = time.monotonic() + PROGRESS_SECONDS
    while pending:
        states = project_states(context.checkout.project)
        for service in list(pending):
            state = states.get(service)
            probe_ok = _probe_ok(context, service, state) if state else None
            ready_when = context.manifest.settings(service).ready_when
            status, detail = readiness(state, probe_ok, ready_when)
            details[service] = detail
            if status.done:
                seconds = time.monotonic() - started
                results[service] = {
                    "readiness": status.value,
                    "seconds": round(seconds, 1),
                }
                context.say(f"  {service:<22} {status.value} after {seconds:.1f}s")
                pending.remove(service)
            elif status is Readiness.FAILED:
                raise ModeError(
                    f"{service} {detail}; see docker logs "
                    f"{state.name if state else service}"
                )
        if not pending:
            break
        now = time.monotonic()
        if now > deadline:
            waiting = ", ".join(
                f"{service} ({details[service]})" for service in pending
            )
            logs = " ".join(states[s].name for s in pending if s in states)
            hint = f"; see docker logs {logs}" if logs else ""
            raise ModeError(f"not ready after {timeout:.0f}s: {waiting}{hint}")
        if now >= next_progress:
            waiting = ", ".join(
                f"{service} ({details[service]})" for service in pending
            )
            context.say(f"  waiting for {waiting}")
            next_progress = now + PROGRESS_SECONDS
        time.sleep(POLL_SECONDS)
    return results


def _start_servers(
    context: Context, devcontainer: Devcontainer, servers: list[Server], timeout: float
) -> dict[str, dict[str, Any]]:
    results: dict[str, dict[str, Any]] = {}
    for state in devcontainer.states(servers):
        server = state.server
        if state.listening:
            owner = f" ({state.owner})" if state.owner else ""
            context.say(
                f"  {server.name:<26} already listening on {server.port}{owner}"
            )
            results[server.name] = {"action": "reused", "url": state.url}
            continue
        if not (context.checkout.root / NODE_MODULES).is_dir():
            raise ModeError(
                f"{server.name} needs the JavaScript dependencies: "
                f"yarn --cwd packages install --frozen-lockfile"
            )
        began = time.monotonic()
        if state.pid is not None:
            started = Started(state.pid, None)
            context.say(f"  {server.name:<26} already starting (pid {started.pid})")
        else:
            started = devcontainer.start(server)
            context.say(
                f"  {server.name:<26} started (pid {started.pid}), "
                f"waiting for {state.url}"
            )
        failure = devcontainer.wait_ready(server, started, timeout)
        if failure is not None:
            raise ModeError(f"{server.name} {failure}")
        seconds = time.monotonic() - began
        context.say(f"  {server.name:<26} ready after {seconds:.1f}s")
        results[server.name] = {
            "action": "started",
            "url": state.url,
            "seconds": round(seconds, 1),
        }
    return results


def _stop_services(context: Context, services: Sequence[str]) -> list[str]:
    states = project_states(context.checkout.project)
    targets = [
        states[service]
        for service in services
        if service in states and states[service].status in ACTIVE_STATUSES
    ]
    if targets:
        context.say(f"stopping {', '.join(state.service for state in targets)}")
        docker(["stop", *(state.id for state in targets)])
    return [state.service for state in targets]


def _stop_servers(
    context: Context, devcontainer: Devcontainer, keep: set[str]
) -> list[str]:
    stopped = []
    for state in devcontainer.states(context.manifest.servers.values()):
        if state.pid is None or state.server.name in keep:
            continue
        context.say(f"stopping {state.server.name} (pid {state.pid})")
        devcontainer.stop(state.server, state.pid)
        stopped.append(state.server.name)
    return stopped


def _print_urls(
    context: Context, plan: Plan, servers: dict[str, dict[str, Any]]
) -> None:
    lines = []
    for service in plan.services:
        url = context.manifest.settings(service).url
        if url:
            lines.append(f"  {service:<26} {url}")
    for name, result in servers.items():
        lines.append(f"  {name:<26} {result['url']}")
    if lines:
        context.say("open:")
        for line in lines:
            context.say(line)


def command_up(
    context: Context, mode: Mode, servers_spec: str, timeout: float | None
) -> int:
    began = time.monotonic()
    servers = _selected_servers(context.manifest, mode, servers_spec)
    plan = _plan(context, mode)
    if plan.conflicts:
        _report_conflicts(context, plan)
        return EXIT_FAILED
    devcontainer_state = plan.states.get(DEVCONTAINER_SERVICE)
    if DEVCONTAINER_SERVICE in plan.services and devcontainer_state is None:
        raise ModeError(
            "this checkout has no devcontainer yet: "
            + _devcontainer_hint(context.checkout, mode)
        )
    missing = [service for service in plan.services if service not in plan.states]
    if missing and not context.checkout.binds_resolve_on_host:
        raise ModeError(
            f"cannot create {', '.join(missing)} from here: "
            f"{context.checkout.host_root} is not visible, so bind mounts would not "
            "resolve on the Docker host; run this command on the host"
        )
    extra = sorted(
        service
        for service, state in plan.states.items()
        if service not in plan.services and state.status in ACTIVE_STATUSES
    )
    ensure_volumes(context.checkout.cache_volumes())
    to_start = [service for service in plan.services if service != DEVCONTAINER_SERVICE]
    context.say(f"mode {mode.name}: {', '.join(plan.services)}")
    if to_start:
        # --no-recreate: containers of this checkout, the devcontainer included,
        # are reused as they are rather than replaced.
        code = plan.compose.run("up", "--detach", "--no-recreate", *to_start)
        if code != EXIT_OK:
            raise ModeError(f"docker compose up failed with exit code {code}")
    wait = timeout if timeout is not None else mode.ready_timeout
    context.say("waiting for readiness")
    service_results = _wait_services(context, plan, wait, began)
    server_results: dict[str, dict[str, Any]] = {}
    if servers:
        devcontainer = Devcontainer(
            context.checkout,
            project_states(context.checkout.project).get(DEVCONTAINER_SERVICE),
        )
        if not devcontainer.running:
            raise ModeError(
                "the devcontainer is not running: "
                + _devcontainer_hint(context.checkout, mode)
            )
        context.say("dev servers")
        server_results = _start_servers(context, devcontainer, servers, wait)
    seconds = time.monotonic() - began
    context.say(f"mode {mode.name} ready in {seconds:.1f}s")
    if extra:
        context.say(
            f"also running: {', '.join(extra)} "
            f"(step-dev mode switch {mode.name} stops them)"
        )
    _print_urls(context, plan, server_results)
    if context.output is OutputFormat.JSON:
        print(
            json.dumps(
                {
                    "mode": mode.name,
                    "project": context.checkout.project,
                    "seconds": round(seconds, 1),
                    "services": service_results,
                    "servers": server_results,
                    "alsoRunning": extra,
                },
                indent=2,
            )
        )
    return EXIT_OK


def command_switch(
    context: Context, mode: Mode, servers_spec: str, timeout: float | None
) -> int:
    _selected_servers(context.manifest, mode, servers_spec)
    plan = _plan(context, mode)
    if plan.conflicts:
        _report_conflicts(context, plan)
        return EXIT_FAILED
    others = [
        service
        for service in plan.states
        if service not in plan.services and service != DEVCONTAINER_SERVICE
    ]
    _stop_services(context, others)
    devcontainer = Devcontainer(context.checkout, plan.states.get(DEVCONTAINER_SERVICE))
    if devcontainer.running:
        _stop_servers(context, devcontainer, set(mode.servers))
    return command_up(context, mode, servers_spec, timeout)


def command_stop(context: Context) -> int:
    states = project_states(context.checkout.project)
    devcontainer = Devcontainer(context.checkout, states.get(DEVCONTAINER_SERVICE))
    stopped_servers = (
        _stop_servers(context, devcontainer, set()) if devcontainer.running else []
    )
    stopped = _stop_services(
        context, [service for service in states if service != DEVCONTAINER_SERVICE]
    )
    if not stopped and not stopped_servers:
        context.say("nothing to stop")
    context.say("volumes and caches are kept; the devcontainer keeps running")
    if context.output is OutputFormat.JSON:
        print(json.dumps({"services": stopped, "servers": stopped_servers}, indent=2))
    return EXIT_OK


def command_preflight(context: Context, mode: Mode) -> int:
    plan = _plan(context, mode)
    if context.output is OutputFormat.JSON:
        conflicts = [str(conflict) for conflict in plan.conflicts]
        print(json.dumps({"mode": mode.name, "conflicts": conflicts}, indent=2))
    if plan.conflicts:
        _report_conflicts(context, plan)
        return EXIT_FAILED
    context.say(
        f"mode {mode.name} can start in Compose project {context.checkout.project}"
    )
    return EXIT_OK


def _mode_configs(context: Context) -> dict[str, dict[str, Any]]:
    """Each mode's Compose services, one Compose call per set of files."""
    by_files: dict[tuple[str, ...], dict[str, Any]] = {}
    for mode in context.manifest.modes:
        if mode.compose_files not in by_files:
            by_files[mode.compose_files] = Compose(
                context.checkout, mode.compose_files
            ).config()
    return {mode.name: by_files[mode.compose_files] for mode in context.manifest.modes}


def command_status(context: Context) -> int:
    checkout = context.checkout
    states = project_states(checkout.project)
    configs = _mode_configs(context)
    mode_services = {
        mode.name: closure(configs[mode.name], mode.services)
        for mode in context.manifest.modes
    }
    active = {
        service for service, state in states.items() if state.status in ACTIVE_STATUSES
    }
    known = []
    for services in mode_services.values():
        known += [service for service in services if service not in known]
    known += sorted(service for service in states if service not in known)
    devcontainer = Devcontainer(checkout, states.get(DEVCONTAINER_SERVICE))
    servers = devcontainer.states(context.manifest.servers.values())
    in_modes = set().union(*mode_services.values())
    listening = {state.server.name for state in servers if state.listening}
    # The smallest mode that accounts for what runs; modes sharing services,
    # such as backend and full, differ in their dev servers.
    covering = [
        mode
        for mode in context.manifest.modes
        if active & in_modes <= set(mode_services[mode.name])
    ]
    serving = [mode for mode in covering if listening <= set(mode.servers)]
    candidates = serving or covering
    current = candidates[0].name if active & in_modes and candidates else None
    up = {
        service
        for service, state in states.items()
        if readiness(state, None, context.manifest.settings(service).ready_when)[0].done
    }
    containers = list_containers()
    conflicts: dict[str, list[str]] = {}
    for mode in context.manifest.modes:
        found = find_conflicts(
            checkout.project,
            str(checkout.host_root),
            configs[mode.name],
            mode_services[mode.name],
            containers,
            states.get(DEVCONTAINER_SERVICE),
        )
        conflicts[mode.name] = [str(conflict) for conflict in found]
    document = {
        "checkout": str(checkout.host_root),
        "project": checkout.project,
        "containerPrefix": checkout.name_prefix,
        "mode": current,
        # Running services no mode starts, such as the opt-in wbraid profile.
        "outsideModes": sorted(active - in_modes),
        "services": {
            service: {
                "container": states[service].name if service in states else None,
                "status": states[service].status if service in states else "absent",
                "health": states[service].health if service in states else None,
            }
            for service in known
        },
        "servers": {
            state.server.name: {
                "port": state.server.port,
                "listening": state.listening,
                "startedByStepDev": state.pid is not None,
                "owner": state.owner,
            }
            for state in servers
        },
        "modes": {
            mode.name: {
                "up": len(up & set(mode_services[mode.name])),
                "services": len(mode_services[mode.name]),
                "conflicts": conflicts[mode.name],
            }
            for mode in context.manifest.modes
        },
    }
    if context.output is OutputFormat.JSON:
        print(json.dumps(document, indent=2))
        return EXIT_OK
    _print_status(document, servers)
    return EXIT_OK


def _print_status(document: dict[str, Any], servers: list[ServerState]) -> None:
    prefix = document["containerPrefix"]
    _say(f"checkout  {document['checkout']}")
    names = f"container names {prefix}*" if prefix else "unprefixed container names"
    _say(f"project   {document['project']} ({names})")
    _say(f"mode      {document['mode'] or 'none running'}")
    if document["outsideModes"]:
        _say(f"also      {', '.join(document['outsideModes'])} (in no mode)")
    _say()
    _say(f"{'service':<24}{'status':<14}{'health':<11}container")
    for service, info in document["services"].items():
        _say(
            f"{service:<24}{info['status']:<14}{info['health'] or '-':<11}"
            f"{info['container'] or '-'}"
        )
    _say()
    _say(f"{'server':<28}{'port':<7}state")
    for state in servers:
        if state.listening:
            description = "listening"
            if state.pid is not None:
                description += f" (started by step-dev, pid {state.pid})"
            elif state.owner:
                description += f" ({state.owner})"
        elif state.pid is not None:
            description = f"starting (pid {state.pid})"
        else:
            description = "-"
        _say(f"{state.server.name:<28}{state.server.port:<7}{description}")
    _say()
    for name, info in document["modes"].items():
        _say(f"{name:<14}{info['up']}/{info['services']} services up or done")
        for conflict in info["conflicts"]:
            _say(f"  blocked: {conflict}")


def command_list(manifest: Manifest) -> int:
    for mode in manifest.modes:
        _say(f"{mode.name:<14}{mode.summary}")
        _say(f"{'':<14}config    .devcontainer/{mode.config}")
        _say(f"{'':<14}services  {', '.join(mode.services)}")
        if mode.servers:
            defaults = ", ".join(mode.default_servers) or "none"
            _say(f"{'':<14}servers   {', '.join(mode.servers)} (default: {defaults})")
        _say()
    return EXIT_OK


def _parser(manifest: Manifest) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="step-dev mode",
        description="Devcontainer modes: the Compose services and dev servers each "
        "kind of work needs (.devcontainer/modes.json). Stopping and switching never "
        "remove containers, volumes or caches.",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="the modes and what each starts")
    commands.add_parser("status", help="running services, dev servers and blockers")
    for name, help_text in (
        (
            "up",
            "start a mode's services, wait until they are ready and start its servers",
        ),
        ("switch", "stop what the mode does not use, then bring it up"),
    ):
        command = commands.add_parser(name, help=help_text)
        command.add_argument("mode", choices=manifest.mode_names())
        command.add_argument(
            "--servers",
            default=DEFAULT_SERVERS,
            help=f"dev servers to start: {DEFAULT_SERVERS} (the mode's), {NO_SERVERS} "
            "or comma-separated names; listening ones are reused",
        )
        command.add_argument(
            "--timeout",
            type=float,
            help="seconds to wait for readiness (default: the mode's)",
        )
    commands.add_parser(
        "stop",
        help="stop this checkout's services and step-dev servers, not the devcontainer",
    )
    preflight = commands.add_parser(
        "preflight",
        help="check that a mode can start here without touching any container",
    )
    preflight.add_argument("mode", choices=manifest.mode_names())
    for command in commands.choices.values():
        command.add_argument(
            "--format",
            choices=[output.value for output in OutputFormat],
            default=OutputFormat.TEXT.value,
            help="output format",
        )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    try:
        manifest = load_manifest(REPOSITORY_ROOT)
        arguments = _parser(manifest).parse_args(argv)
        if arguments.command == "list":
            return command_list(manifest)
        context = Context(load_checkout(), manifest, OutputFormat(arguments.format))
        if arguments.command == "status":
            return command_status(context)
        if arguments.command == "stop":
            return command_stop(context)
        mode = manifest.mode(arguments.mode)
        if arguments.command == "preflight":
            return command_preflight(context, mode)
        if arguments.command == "up":
            return command_up(context, mode, arguments.servers, arguments.timeout)
        return command_switch(context, mode, arguments.servers, arguments.timeout)
    except (CheckoutError, DockerError, ManifestError, ModeError, PlanError) as error:
        print(f"step-dev mode: {error}", file=sys.stderr)
        return EXIT_FAILED
