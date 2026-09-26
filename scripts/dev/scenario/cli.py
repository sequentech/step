# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""``step-dev scenario``: bring named synthetic scenarios up on the real backend."""

from __future__ import annotations

import argparse
import json
import os
import signal
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from enum import Enum
from types import FrameType
from typing import TYPE_CHECKING, Any

from scripts.dev.mode.checkout import (
    REPOSITORY_ROOT,
    Checkout,
    CheckoutError,
    load_checkout,
)
from scripts.dev.mode.docker import DockerError
from scripts.dev.mode.manifest import ManifestError, load_manifest
from scripts.e2e.journeys import fixtures

from . import runner
from .catalog import (
    SCENARIOS,
    STAGE_DESCRIPTIONS,
    CatalogError,
    Scenario,
    find_scenario,
    scenario_names,
    voter_username,
)
from .runner import Progress, ScenarioError, StatusReport
from .settings import Portals, SettingsError, find_step_cli, journey_environment
from .state import State, StateError, StateStore

if TYPE_CHECKING:
    from .backend import StackBackend

EXIT_OK = 0
EXIT_FAILED = 1
EXIT_INTERRUPTED = 130
LOG_DIRECTORY = "logs"


class OutputFormat(Enum):
    TEXT = "text"
    JSON = "json"


@dataclass
class Context:
    checkout: Checkout
    store: StateStore
    environment: dict[str, str]
    portals: Portals
    output: OutputFormat

    def say(self, message: str = "") -> None:
        # JSON output keeps stdout parseable; progress goes to stderr then.
        stream = sys.stderr if self.output is OutputFormat.JSON else sys.stdout
        print(message, file=stream, flush=True)


def _context(output: OutputFormat) -> Context:
    checkout = load_checkout()
    store = StateStore.for_project(checkout.root, checkout.project)
    environment = journey_environment(
        checkout.env, os.environ, store.directory / LOG_DIRECTORY
    )
    return Context(
        checkout, store, environment, Portals.from_environment(environment), output
    )


def _backend(context: Context, timeout: float | None) -> StackBackend:
    (context.store.directory / LOG_DIRECTORY).mkdir(parents=True, exist_ok=True)
    os.environ.update(context.environment)
    # The journeys' clients read their endpoints from the environment on import.
    from .backend import StackBackend

    return StackBackend(
        context.checkout,
        load_manifest(REPOSITORY_ROOT),
        context.portals,
        context.store.directory,
        context.say,
        timeout,
    )


def _links(context: Context, scenario: Scenario, state: State) -> dict[str, str]:
    if state.event_id is None:
        return {}
    return {
        link.value: context.portals.link(link, state.tenant_id, state.event_id)
        for link in scenario.links
    }


def _credentials(context: Context, scenario: Scenario, state: State) -> dict[str, Any]:
    return {
        "synthetic": True,
        "voterPassword": fixtures.VOTER_PASSWORD,
        "voters": state.voters,
        "votes": [
            {"voter": voter_username(scenario, vote), "candidate": vote.candidate}
            for vote in scenario.votes
        ],
        "admin": {
            "username": context.environment["ADMIN_PORTAL_TEST_USERNAME"],
            "password": context.environment["ADMIN_PORTAL_TEST_PASSWORD"],
        },
    }


def _describe(context: Context, scenario: Scenario, state: State) -> dict[str, Any]:
    return {
        "scenario": scenario.name,
        "project": state.project,
        "tenantId": state.tenant_id,
        "eventId": state.event_id,
        "links": _links(context, scenario, state),
        "credentials": _credentials(context, scenario, state),
        "stateFile": str(context.store.path(scenario.name)),
    }


def _print_links(context: Context, scenario: Scenario, state: State) -> None:
    say = context.say
    say(f"event     {state.event_id} (Scenario {scenario.name})")
    say("open:")
    for name, url in _links(context, scenario, state).items():
        say(f"  {name:<10}{url}")
    credentials = _credentials(context, scenario, state)
    say(
        "synthetic fixture credentials, not real people; every voter's password is "
        f"{credentials['voterPassword']}"
    )
    for area, usernames in state.voters.items():
        say(f"  area {area}    {'  '.join(usernames)}")
    if credentials["votes"]:
        cast = ", ".join(
            f"{vote['voter']} {vote['candidate']}" for vote in credentials["votes"]
        )
        say(f"  votes     {cast}")
    admin = credentials["admin"]
    say(
        f"  admin     {admin['username']} / {admin['password']} (development default; "
        f"the emailed code is in docker logs {context.checkout.name_prefix}keycloak)"
    )


def _prepare(
    context: Context,
    backend: StackBackend,
    scenario: Scenario | None,
    step_cli: str | None,
) -> None:
    binary = find_step_cli(step_cli, context.checkout.root, os.environ.get("PATH"))
    context.say("waiting for the backend services")
    backend.check_services()
    if scenario is not None and scenario.needs_trustees:
        backend.start_trustees()
    backend.authenticate()
    backend.open_cli(binary)


def command_up(
    context: Context, scenario: Scenario, timeout: float | None, step_cli: str | None
) -> int:
    backend = _backend(context, timeout)
    try:
        _prepare(context, backend, scenario, step_cli)
        context.say(f"scenario {scenario.name}: {scenario.summary}")
        report = runner.up(scenario, backend, context.store, context.say)
    finally:
        backend.close()
    verb = {
        runner.Outcome.CREATED: "created",
        runner.Outcome.RESUMED: "completed",
        runner.Outcome.REUSED: "reused",
    }[report.outcome]
    if context.output is OutputFormat.JSON:
        document = _describe(context, scenario, report.state)
        document.update(
            outcome=report.outcome.value, seconds=report.seconds, stages=report.ran
        )
        print(json.dumps(document, indent=2))
        return EXIT_OK
    context.say(f"{scenario.name} ready in {report.seconds:.1f}s ({verb})")
    _print_links(context, scenario, report.state)
    return EXIT_OK


def command_urls(context: Context, scenario: Scenario) -> int:
    state = context.store.load(scenario.name)
    if state is None or state.event_id is None:
        raise ScenarioError(
            f"{scenario.name} is not up here; run scripts/dev/step-dev scenario up "
            f"{scenario.name}"
        )
    if context.output is OutputFormat.JSON:
        print(json.dumps(_describe(context, scenario, state), indent=2))
        return EXIT_OK
    _print_links(context, scenario, state)
    return EXIT_OK


def command_reset(
    context: Context, scenario: Scenario, timeout: float | None, step_cli: str | None
) -> int:
    if context.store.load(scenario.name) is None:
        context.say(f"{scenario.name}: {runner.ResetOutcome.NOTHING.value}")
        return EXIT_OK
    backend = _backend(context, timeout)
    try:
        _prepare(context, backend, None, step_cli)
        outcome = runner.reset(scenario, backend, context.store, context.say)
    finally:
        backend.close()
    if context.output is OutputFormat.JSON:
        print(json.dumps({"scenario": scenario.name, "outcome": outcome.value}))
    else:
        context.say(f"{scenario.name}: {outcome.value}")
    return EXIT_OK


def _status_entry(
    context: Context, scenario: Scenario, report: StatusReport, error: str | None
) -> dict[str, Any]:
    state = report.state
    return {
        "state": report.progress.value,
        "eventId": state.event_id if state else None,
        "next": STAGE_DESCRIPTIONS[report.next_stage] if report.next_stage else None,
        "unverified": error,
        "links": _links(context, scenario, state) if state else {},
    }


def command_status(context: Context, scenarios: Sequence[Scenario]) -> int:
    backend = None
    entries: dict[str, dict[str, Any]] = {}
    for scenario in scenarios:
        state = context.store.load(scenario.name)
        error = None
        if state is None:
            report = StatusReport(Progress.ABSENT)
        else:
            try:
                backend = backend or _backend(context, None)
                report = runner.status(scenario, backend, context.store)
            except (OSError, AssertionError) as failure:
                report = StatusReport(Progress.INCOMPLETE, state)
                error = f"backend unreachable: {failure}"
        entries[scenario.name] = _status_entry(context, scenario, report, error)
    if context.output is OutputFormat.JSON:
        print(
            json.dumps(
                {"project": context.checkout.project, "scenarios": entries}, indent=2
            )
        )
        return EXIT_OK
    context.say(f"project {context.checkout.project}")
    for name, entry in entries.items():
        detail = entry["eventId"] or "-"
        if entry["next"]:
            detail += f" (next: {entry['next']})"
        if entry["unverified"]:
            detail += f" ({entry['unverified']})"
        context.say(f"  {name:<20}{entry['state']:<12}{detail}")
    return EXIT_OK


def command_list() -> int:
    for scenario in SCENARIOS:
        print(f"{scenario.name:<20}{scenario.summary}")
        stages = ", ".join(STAGE_DESCRIPTIONS[stage] for stage in scenario.stages)
        print(f"{'':<20}stages  {stages}")
        print(f"{'':<20}links   {', '.join(link.value for link in scenario.links)}")
        print()
    return EXIT_OK


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="step-dev scenario",
        description="Synthetic scenarios on the checkout's running backend. Each has "
        "its own election event, recorded in .cache/scenarios/<Compose project>/; "
        "reset deletes only that event.",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="the scenarios and the stages that build them")
    up = commands.add_parser(
        "up", help="create or reuse the scenario's event and bring it to its state"
    )
    urls = commands.add_parser("urls", help="the scenario's links and credentials")
    reset = commands.add_parser("reset", help="delete the scenario's own event")
    status = commands.add_parser("status", help="which scenarios are up and ready")
    for command in (up, urls, reset):
        command.add_argument("scenario", choices=scenario_names())
    status.add_argument("scenario", nargs="?", choices=scenario_names())
    for command in (up, reset):
        command.add_argument(
            "--timeout",
            type=float,
            help="seconds to allow each wait (default: its own, up to an hour for "
            "services that are still compiling)",
        )
        command.add_argument(
            "--step-cli",
            help="the step-cli binary (default: step-cli on PATH, then "
            "packages/step-cli/rust-local-target/release/step-cli)",
        )
    for command in (up, urls, reset, status):
        command.add_argument(
            "--format",
            choices=[output.value for output in OutputFormat],
            default=OutputFormat.TEXT.value,
            help="output format",
        )
    return parser


def _interrupt(signum: int, frame: FrameType | None) -> None:
    # Unwinds like Ctrl-C, so that switched Keycloak settings are restored.
    raise KeyboardInterrupt


def main(argv: Sequence[str] | None = None) -> int:
    signal.signal(signal.SIGTERM, _interrupt)
    context = None
    try:
        arguments = _parser().parse_args(argv)
        if arguments.command == "list":
            return command_list()
        context = _context(OutputFormat(arguments.format))
        if arguments.command == "status":
            scenarios = (
                [find_scenario(arguments.scenario)] if arguments.scenario else SCENARIOS
            )
            return command_status(context, scenarios)
        scenario = find_scenario(arguments.scenario)
        if arguments.command == "urls":
            return command_urls(context, scenario)
        if arguments.command == "reset":
            return command_reset(
                context, scenario, arguments.timeout, arguments.step_cli
            )
        return command_up(context, scenario, arguments.timeout, arguments.step_cli)
    except KeyboardInterrupt:
        print("step-dev scenario: interrupted", file=sys.stderr)
        return EXIT_INTERRUPTED
    except (
        CatalogError,
        CheckoutError,
        DockerError,
        ManifestError,
        ScenarioError,
        SettingsError,
        StateError,
    ) as error:
        print(f"step-dev scenario: {error}", file=sys.stderr)
        return EXIT_FAILED
    except OSError as error:
        print(
            f"step-dev scenario: cannot reach the backend ({error}); run this in the "
            "devcontainer, whose network reaches the Compose services",
            file=sys.stderr,
        )
        return EXIT_FAILED
    except AssertionError as error:
        # The journeys' clients report failed requests and step-cli commands so.
        print(f"step-dev scenario: {error}", file=sys.stderr)
        if context is not None:
            log = context.store.directory / LOG_DIRECTORY / "step-cli.log"
            print(f"step-cli output: {log}", file=sys.stderr)
        return EXIT_FAILED
