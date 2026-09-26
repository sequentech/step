# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The scenario stages on the checkout's running stack.

Built on the backend journeys' clients: Hasura with the admin secret for
reads, step-cli as the tenant administrator for changes, Keycloak for realms
and voter logins. Those clients read their endpoints from the environment when
first imported, so import this module after setting it up.
"""

from __future__ import annotations

import json
import shutil
import socket
import time
import urllib.parse
from collections.abc import Callable
from pathlib import Path
from typing import Any

from scripts.dev.mode.checkout import Checkout
from scripts.dev.mode.docker import Compose, ContainerState, probe, project_states
from scripts.dev.mode.manifest import Manifest
from scripts.dev.mode.plan import Readiness, readiness
from scripts.e2e.journeys import bootstrap, fixtures
from scripts.e2e.journeys.client import (
    HASURA_URL,
    KEYCLOAK_URL,
    TENANT_ID,
    TENANT_REALM,
    Hasura,
    Http,
    Keycloak,
    LoginError,
    StepCli,
    event_realm,
    wait_until,
)
from scripts.e2e.journeys.voting import Portal
from scripts.e2e.ui.seed import allow_portals

from .catalog import Scenario, Stage, census, census_usernames, voter_username
from .runner import Say, ScenarioError
from .settings import PORTAL_CLIENTS, Portals, accepts
from .state import State

BACKEND_MODE = "backend"
# Long-running services the stages use; all belong to the backend mode.
REQUIRED_SERVICES = (
    "postgres",
    "postgres-keycloak",
    "postgres-b4",
    "minio",
    "rabbitmq",
    "immudb",
    "keycloak",
    "graphql-engine",
    "harvest",
    "windmill",
    "beat",
    "b4",
)
# The braid services signing as the trustees bootstrap registers; no mode
# starts them.
TRUSTEE_SERVICES = bootstrap.TRUSTEES
TRUSTEE_PROFILE = "full"
RUNNING = "running"
EVENT_NAME = "Scenario {name}"
THRESHOLD = str(len(bootstrap.TRUSTEES))
SUCCESS = "SUCCESS"
FAILED_EXECUTIONS = frozenset({"FAILED", "CANCELLED"})
OPEN = "OPEN"
CLOSED = "CLOSED"
ONLINE = "ONLINE"
KIOSK = "KIOSK"
STATUS_KEYS = {ONLINE: "voting_status", KIOSK: "kiosk_voting_status"}
PUBLISHING = "Publishing"
PUBLISHED = "Published"
FAILED_PUBLICATION = "Failed"
TALLY_TYPE = "ELECTORAL_RESULTS"
RESULTS_ACCESS = ("--status", "enabled", "--access", "public")
RESULTS_SCOPE = ("--visibility-scope", "full_event")
NOT_SET_UP = "Account is not fully set up"
OTP_MARKER = "otp-test-mode.json"
MAX_REALM_USERS = 1000
POLL_SECONDS = 3.0
PROGRESS_SECONDS = 30.0
# Seconds each wait allows unless --timeout overrides it.
TIMEOUTS = {
    "services": 3600.0,
    "tenant": 600.0,
    "hasura": 90.0,
    "keys": 600.0,
    "status": 60.0,
    "tally": 900.0,
    "results": 300.0,
    "delete": 300.0,
}

EVENT = """query ($tenant: uuid!, $event: uuid!) {
  sequent_backend_election_event(
    where: {tenant_id: {_eq: $tenant}, id: {_eq: $event}}
  ) { id annotations status }
}"""
ANNOTATED_EVENTS = """query ($tenant: uuid!, $annotations: jsonb!) {
  sequent_backend_election_event(
    where: {tenant_id: {_eq: $tenant}, annotations: {_contains: $annotations}}
  ) { id }
}"""
EVENT_ENTITIES = """query ($event: uuid!) {
  sequent_backend_election(where: {election_event_id: {_eq: $event}}) { id external_id }
  sequent_backend_area(where: {election_event_id: {_eq: $event}}) { id name }
  sequent_backend_contest(where: {election_event_id: {_eq: $event}}) {
    id election_id presentation
  }
  sequent_backend_candidate(where: {election_event_id: {_eq: $event}}) {
    id contest_id presentation
  }
}"""
KEYS_CEREMONIES = """query ($event: uuid!) {
  sequent_backend_keys_ceremony(
    where: {election_event_id: {_eq: $event}}, order_by: {created_at: desc}
  ) { id execution_status status }
}"""
BALLOT_PUBLICATIONS = """query ($event: uuid!) {
  sequent_backend_ballot_publication(where: {election_event_id: {_eq: $event},
    published_at: {_is_null: false}, is_generated: {_eq: true}}) { id }
}"""
BALLOT_STYLE = """query ($id: uuid!) {
  sequent_backend_ballot_style(where: {id: {_eq: $id}}) { ballot_eml }
}"""
CAST_VOTERS = """query ($event: uuid!) {
  sequent_backend_cast_vote(where: {election_event_id: {_eq: $event}}) {
    voter_id_string
  }
}"""
TALLY_SESSIONS = """query ($event: uuid!) {
  sequent_backend_tally_session(
    where: {election_event_id: {_eq: $event}}, order_by: {created_at: desc}
  ) { id execution_status }
}"""
TALLY_EXECUTION = """query ($session: uuid!) {
  sequent_backend_tally_session_execution(
    where: {tally_session_id: {_eq: $session}, results_event_id: {_is_null: false}},
    order_by: {current_message_id: desc}, limit: 1
  ) { id results_event_id }
}"""
RESULTS_PUBLICATION = """query ($event: uuid!) {
  sequent_backend_tally_results_publication(
    where: {election_event_id: {_eq: $event}}, order_by: {created_at: desc}, limit: 1
  ) { id publication_status error_message }
}"""


class StackBackend:
    """The stages against the services of one checkout's Compose project."""

    def __init__(
        self,
        checkout: Checkout,
        manifest: Manifest,
        portals: Portals,
        state_directory: Path,
        say: Say,
        timeout: float | None = None,
    ) -> None:
        self.tenant_id = TENANT_ID
        self.project = checkout.project
        self.checkout = checkout
        self.manifest = manifest
        self.portals = portals
        self.state_directory = state_directory
        self.say = say
        self.timeout = timeout
        self.admin = Hasura.admin()
        self.keycloak = Keycloak()
        self.cli: StepCli | None = None

    # Readiness

    def _wait(
        self,
        description: str,
        observe: Callable[[], tuple[bool, str]],
        budget: str,
        hint: str = "",
    ) -> None:
        """Polls ``observe`` until it reports done; fails with what it saw last."""
        timeout = self.timeout if self.timeout is not None else TIMEOUTS[budget]
        last = ["nothing observed yet"]

        def done() -> bool:
            finished, observed = observe()
            last[0] = observed
            return finished

        try:
            wait_until(description, done, timeout=timeout, interval=POLL_SECONDS)
        except TimeoutError as error:
            suffix = f"; {hint}" if hint else ""
            raise ScenarioError(
                f"{description}: not done after {timeout:.0f}s, last observed: "
                f"{last[0]}{suffix}"
            ) from error

    def _states(self) -> dict[str, ContainerState]:
        return project_states(self.project)

    def _service_readiness(
        self, service: str, state: ContainerState | None
    ) -> tuple[Readiness, str]:
        probe_command = self.manifest.settings(service).probe
        probe_ok = None
        if (
            probe_command is not None
            and state is not None
            and state.status == RUNNING
            and state.health in (None, "healthy")
        ):
            probe_ok = probe(state.id, probe_command)
        return readiness(state, probe_ok)

    def check_services(self) -> None:
        """Waits for the backend mode's services; fails at once if one is stopped."""
        began = time.monotonic()
        next_progress = began + PROGRESS_SECONDS
        timeout = self.timeout if self.timeout is not None else TIMEOUTS["services"]
        while True:
            states = self._states()
            pending = []
            for service in REQUIRED_SERVICES:
                state = states.get(service)
                if state is None or state.status not in (RUNNING, "restarting"):
                    status = state.status if state else "not created"
                    raise ScenarioError(
                        f"the backend is not running ({service}: {status}); start it "
                        f"with scripts/dev/step-dev mode up {BACKEND_MODE}"
                    )
                status, detail = self._service_readiness(service, state)
                if status is not Readiness.READY:
                    pending.append(f"{service} ({detail})")
            if not pending:
                break
            now = time.monotonic()
            if now - began > timeout:
                raise ScenarioError(
                    f"backend services not ready after {timeout:.0f}s: "
                    f"{', '.join(pending)}"
                )
            if now >= next_progress:
                self.say(f"waiting for {', '.join(pending)}")
                next_progress = now + PROGRESS_SECONDS
            time.sleep(POLL_SECONDS)

    def start_trustees(self) -> None:
        states = self._states()
        stopped = [
            service
            for service in TRUSTEE_SERVICES
            if service not in states or states[service].status != RUNNING
        ]
        if not stopped:
            return
        self.say(
            f"starting {', '.join(stopped)}: keys ceremonies and tallies need the "
            "trustees (their first start builds the braid image)"
        )
        compose = Compose(self.checkout, self.manifest.mode(BACKEND_MODE).compose_files)
        code = compose.run(
            "--profile",
            TRUSTEE_PROFILE,
            "up",
            "--detach",
            "--no-deps",
            "--no-recreate",
            *stopped,
        )
        if code != 0:
            raise ScenarioError(
                f"docker compose up {' '.join(stopped)} failed ({code})"
            )

        def running() -> tuple[bool, str]:
            states = self._states()
            observed = {
                service: states[service].status if service in states else "absent"
                for service in TRUSTEE_SERVICES
            }
            return (
                all(status == RUNNING for status in observed.values()),
                ", ".join(
                    f"{service} {status}" for service, status in observed.items()
                ),
            )

        self._wait("the trustee services", running, "status")

    def authenticate(self) -> None:
        """Waits for the tenant's bootstrap and signs its administrator in."""
        for url in (HASURA_URL, KEYCLOAK_URL):
            host = urllib.parse.urlsplit(url).hostname or url
            try:
                socket.getaddrinfo(host, None)
            except socket.gaierror as error:
                raise ScenarioError(
                    f"{host} does not resolve here ({error}); run step-dev scenario "
                    "in the devcontainer, which is on the Compose network"
                ) from error
        self._wait(
            "the super tenant",
            lambda: (bootstrap.tenant_row() is not None, "no tenant row"),
            "tenant",
        )

        def keys_published() -> tuple[bool, str]:
            kids = bootstrap.realm_signing_kids()
            published = bootstrap.published_kids()
            return (
                bool(kids) and kids <= published,
                f"realm keys {sorted(kids)}, published {sorted(published)}",
            )

        self._wait(
            "the tenant realm keys in the published JWKS", keys_published, "tenant"
        )
        self._restore_otp_test_mode()
        try:
            bootstrap.admin_token(self.keycloak)
        except LoginError as error:
            if NOT_SET_UP not in str(error):
                raise ScenarioError(
                    f"the tenant administrator cannot sign in ({error}); set "
                    "ADMIN_PORTAL_TEST_USERNAME and ADMIN_PORTAL_TEST_PASSWORD to an "
                    f"administrator of tenant {TENANT_ID}"
                ) from error
            self._enroll_administrator()
        self._wait(
            "Hasura to accept administrator tokens",
            lambda: (
                bool(bootstrap.hasura_accepts_admin_token(self.keycloak)),
                "tokens rejected",
            ),
            "hasura",
            hint=f"restart Hasura: docker restart {self.checkout.name_prefix}hasura",
        )

    def _otp_marker(self) -> Path:
        return self.state_directory / OTP_MARKER

    def _enroll_administrator(self) -> None:
        """Enrolls the administrator's email code as bootstrap does for the journeys.

        bootstrap switches the tenant's code test mode on for one login; the
        marker lets a later run switch it back if this one is killed meanwhile.
        """
        config = bootstrap.email_otp_config(self.keycloak)
        self.state_directory.mkdir(parents=True, exist_ok=True)
        self._otp_marker().write_text(
            json.dumps(
                {
                    "configId": config["id"],
                    "testMode": config["config"].get("test-mode", "false"),
                }
            ),
            encoding="utf-8",
        )
        try:
            bootstrap.enroll_admin_mfa(self.keycloak)
        finally:
            self._restore_otp_test_mode()
        self.say(
            "enrolled the tenant administrator's email code; the admin portal asks "
            f"for it, and docker logs {self.checkout.name_prefix}keycloak shows it"
        )

    def _restore_otp_test_mode(self) -> None:
        marker = self._otp_marker()
        if not marker.is_file():
            return
        recorded = json.loads(marker.read_text(encoding="utf-8"))
        path = f"{TENANT_REALM}/authentication/config/{recorded['configId']}"
        config = self.keycloak.admin("GET", path)
        if config["config"].get("test-mode", "false") != recorded["testMode"]:
            config["config"]["test-mode"] = recorded["testMode"]
            self.keycloak.admin("PUT", path, config, expect=(204,))
            self.say("restored the tenant realm's email code test mode")
        marker.unlink()

    def open_cli(self, binary: Path) -> None:
        self.cli = StepCli(binary)

    def close(self) -> None:
        if self.cli is not None:
            shutil.rmtree(self.cli.directory, ignore_errors=True)
            self.cli = None

    def _step(self, *args: str, timeout: float = 900) -> str:
        """An administrator command with a fresh login, as the journeys run them."""
        if self.cli is None:
            raise ScenarioError("step-cli is not open")
        bootstrap.configure_step_cli(self.cli)
        return self.cli.step(*args, timeout=timeout)

    # Queries

    def _event(self, event_id: str) -> dict[str, Any] | None:
        rows = self.admin.query(EVENT, {"tenant": self.tenant_id, "event": event_id})[
            "sequent_backend_election_event"
        ]
        return rows[0] if rows else None

    def event_annotations(self, event_id: str) -> dict[str, Any] | None:
        event = self._event(event_id)
        return None if event is None else (event.get("annotations") or {})

    def annotated_events(self, annotations: dict[str, str]) -> list[str]:
        rows = self.admin.query(
            ANNOTATED_EVENTS, {"tenant": self.tenant_id, "annotations": annotations}
        )["sequent_backend_election_event"]
        return [row["id"] for row in rows]

    def _voting_status(self, event_id: str, channel: str) -> str:
        event = self._event(event_id) or {}
        return (event.get("status") or {}).get(STATUS_KEYS[channel]) or ""

    def _realm_users(self, event_id: str) -> dict[str, str]:
        """User IDs by username in the event's realm."""
        users = self.keycloak.admin(
            "GET",
            f"{event_realm(event_id)}/users?briefRepresentation=true&max={MAX_REALM_USERS}",
        )
        return {user["username"]: user["id"] for user in users}

    def _voters_with_votes(self, event_id: str) -> set[str]:
        rows = self.admin.query(CAST_VOTERS, {"event": event_id})[
            "sequent_backend_cast_vote"
        ]
        return {row["voter_id_string"] for row in rows}

    def _keys_ceremonies(self, event_id: str) -> list[dict[str, Any]]:
        return self.admin.query(KEYS_CEREMONIES, {"event": event_id})[
            "sequent_backend_keys_ceremony"
        ]

    def _tally_sessions(self, event_id: str) -> list[dict[str, Any]]:
        return self.admin.query(TALLY_SESSIONS, {"event": event_id})[
            "sequent_backend_tally_session"
        ]

    def _tally_execution(self, session_id: str) -> dict[str, Any] | None:
        rows = self.admin.query(TALLY_EXECUTION, {"session": session_id})[
            "sequent_backend_tally_session_execution"
        ]
        return rows[0] if rows else None

    def _results_publication(self, event_id: str) -> dict[str, Any] | None:
        rows = self.admin.query(RESULTS_PUBLICATION, {"event": event_id})[
            "sequent_backend_tally_results_publication"
        ]
        return rows[0] if rows else None

    def _ballots_published(self, event_id: str) -> bool:
        return bool(
            self.admin.query(BALLOT_PUBLICATIONS, {"event": event_id})[
                "sequent_backend_ballot_publication"
            ]
        )

    def _realm_exists(self, event_id: str) -> bool:
        response = Http().get(
            f"{KEYCLOAK_URL}/admin/realms/{event_realm(event_id)}",
            headers={"Authorization": f"Bearer {self.keycloak.admin_token()}"},
        )
        return response.status == 200

    # Stages

    def holds(self, stage: Stage, scenario: Scenario, state: State) -> bool:
        event_id = state.event_id
        if event_id is None:
            return False
        if stage is Stage.EVENT:
            return self._event(event_id) is not None
        if stage is Stage.VOTERS:
            expected = [
                name for names in census_usernames(scenario).values() for name in names
            ]
            return set(expected) <= set(self._realm_users(event_id))
        if stage is Stage.KEYS:
            return any(
                ceremony["execution_status"] == SUCCESS
                for ceremony in self._keys_ceremonies(event_id)
            )
        if stage is Stage.BALLOTS:
            return self._ballots_published(event_id)
        if stage is Stage.ONLINE_OPEN:
            return self._voting_status(event_id, ONLINE) == OPEN
        if stage is Stage.KIOSK_OPEN:
            return self._voting_status(event_id, KIOSK) == OPEN
        if stage is Stage.VOTES:
            users = self._realm_users(event_id)
            voted = self._voters_with_votes(event_id)
            return all(
                users.get(voter_username(scenario, vote)) in voted
                for vote in scenario.votes
            )
        if stage is Stage.ONLINE_CLOSED:
            return self._voting_status(event_id, ONLINE) == CLOSED
        if stage is Stage.TALLY:
            return any(
                session["execution_status"] == SUCCESS
                and self._tally_execution(session["id"]) is not None
                for session in self._tally_sessions(event_id)
            )
        publication = self._results_publication(event_id)
        return (
            publication is not None and publication["publication_status"] == PUBLISHED
        )

    def run(self, stage: Stage, scenario: Scenario, state: State) -> None:
        if stage is Stage.EVENT:
            self._import_event(scenario, state)
            return
        event_id = state.event_id
        if event_id is None:
            raise ScenarioError(f"{stage.value} needs the scenario's event")
        if stage is Stage.VOTERS:
            self._import_voters(scenario, state, event_id)
        elif stage is Stage.KEYS:
            self._keys_ceremony(scenario, event_id)
        elif stage is Stage.BALLOTS:
            self._publish_ballots(event_id)
        elif stage is Stage.ONLINE_OPEN:
            self._set_voting(event_id, ONLINE, OPEN)
        elif stage is Stage.KIOSK_OPEN:
            self._set_voting(event_id, KIOSK, OPEN)
        elif stage is Stage.VOTES:
            self._cast_votes(scenario, event_id)
        elif stage is Stage.ONLINE_CLOSED:
            self._set_voting(event_id, ONLINE, CLOSED)
        elif stage is Stage.TALLY:
            self._tally(scenario, event_id)
        else:
            self._publish_results(scenario, state, event_id)

    def point_portals(self, state: State) -> None:
        """Lets the event's portal clients redirect to the configured portal URLs.

        The import points them at the backend's own settings, which a developer
        running a portal elsewhere may not share.
        """
        if state.event_id is None:
            return
        realm = event_realm(state.event_id)
        for client_id in PORTAL_CLIENTS:
            (client,) = self.keycloak.admin(
                "GET", f"{realm}/clients?clientId={client_id}"
            )
            origins = self.portals.origins(client_id)
            if not accepts(client, origins):
                allow_portals(self.keycloak, realm, client_id, origins)
                self.say(f"{client_id} now accepts {', '.join(origins)}")

    def _import_event(self, scenario: Scenario, state: State) -> None:
        if self.cli is None:
            raise ScenarioError("step-cli is not open")
        document = fixtures.election_event(self.portals.voting, scenario.name)
        event = document["election_event"]
        fixtures._name(event, EVENT_NAME.format(name=scenario.name))
        event["annotations"] = state.annotations()
        # Portals bundle a default logo; the fixture's is on an external host.
        event["presentation"].pop("logo_url", None)
        # A plain password field, which browser automation fills like a person.
        document["keycloak_event_realm"]["attributes"]["credential-input-policy"] = (
            "standard"
        )
        path = self.cli.directory / "event.json"
        path.write_text(json.dumps(document), encoding="utf-8")
        state.event_id = StepCli.last_id(
            self._step("import-election", "--file-path", str(path), "--is-local")
        )
        self._record_ids(state, state.event_id)
        state.voters = census_usernames(scenario)
        self.point_portals(state)

    def _record_ids(self, state: State, event_id: str) -> None:
        data = self.admin.query(EVENT_ENTITIES, {"event": event_id})
        areas = {area["name"]: area["id"] for area in data["sequent_backend_area"]}
        contests = {
            fixtures.display_name(contest): contest
            for contest in data["sequent_backend_contest"]
        }
        ids: dict[str, dict[str, Any]] = {
            "elections": {},
            "areas": {},
            "contests": {},
            "candidates": {},
            "authorization": {},
        }
        external = {
            election["id"]: election["external_id"]
            for election in data["sequent_backend_election"]
        }
        for spec in fixtures.AREAS:
            contest = contests[spec.contest]
            election_id = contest["election_id"]
            ids["elections"][spec.election] = election_id
            ids["areas"][spec.key] = areas[spec.name]
            ids["contests"][spec.key] = contest["id"]
            ids["candidates"][spec.key] = {
                fixtures.display_name(candidate): candidate["id"]
                for candidate in data["sequent_backend_candidate"]
                if candidate["contest_id"] == contest["id"]
            }
            # Voters name their election by external ID where it has one.
            ids["authorization"][spec.key] = external.get(election_id) or election_id
        state.ids = ids

    def _import_voters(self, scenario: Scenario, state: State, event_id: str) -> None:
        if self.cli is None:
            raise ScenarioError("step-cli is not open")
        if not state.ids:
            self._record_ids(state, event_id)
        state.voters = census_usernames(scenario)
        path = self.cli.directory / "voters.csv"
        fixtures.write_census(path, census(scenario), state.ids["authorization"])
        self._step(
            "import-voters",
            "--election-event-id",
            event_id,
            "--file-path",
            str(path),
            "--is-local",
        )

    def _keys_ceremony(self, scenario: Scenario, event_id: str) -> None:
        if self.cli is None:
            raise ScenarioError("step-cli is not open")
        bootstrap.configure_step_cli(self.cli)
        bootstrap.seed_trustees(self.cli)
        ceremonies = self._keys_ceremonies(event_id)
        active = [
            ceremony
            for ceremony in ceremonies
            if ceremony["execution_status"] not in FAILED_EXECUTIONS
        ]
        if not active and ceremonies:
            latest = ceremonies[0]
            raise ScenarioError(
                f"keys ceremony {latest['id']} is {latest['execution_status']}; see "
                f"docker logs {self.checkout.name_prefix}windmill, then "
                f"scripts/dev/step-dev scenario reset {scenario.name}"
            )
        if not active:
            StepCli.last_id(
                self._step(
                    "start-key-ceremony",
                    "--election-event-id",
                    event_id,
                    "--threshold",
                    THRESHOLD,
                    "--automatic",
                )
            )

        def finished() -> tuple[bool, str]:
            ceremonies = self._keys_ceremonies(event_id)
            if not ceremonies:
                return False, "no ceremony"
            ceremony = ceremonies[0]
            execution = ceremony["execution_status"]
            if execution in FAILED_EXECUTIONS:
                raise ScenarioError(f"keys ceremony {ceremony['id']} {execution}")
            trustees = ((ceremony.get("status") or {}).get("trustees")) or []
            detail = ", ".join(f"{t.get('name')} {t.get('status')}" for t in trustees)
            return execution == SUCCESS, f"{execution} ({detail or 'no trustee yet'})"

        self._wait(
            "the automatic keys ceremony",
            finished,
            "keys",
            hint=f"check docker logs {self.checkout.name_prefix}trustee1",
        )

    def _publish_ballots(self, event_id: str) -> None:
        self._step("publish", "--election-event-id", event_id)
        self._wait(
            "the ballot publication",
            lambda: (self._ballots_published(event_id), "not generated"),
            "status",
        )

    def _set_voting(self, event_id: str, channel: str, status: str) -> None:
        self._step(
            "update-event-voting-status",
            "--election-event-id",
            event_id,
            "--voting-status",
            status,
            "--voting-channel",
            channel,
        )

        def changed() -> tuple[bool, str]:
            current = self._voting_status(event_id, channel)
            return current == status, f"{STATUS_KEYS[channel]} {current}"

        self._wait(f"{channel.lower()} voting {status.lower()}", changed, "status")

    def _cast_votes(self, scenario: Scenario, event_id: str) -> None:
        if self.cli is None:
            raise ScenarioError("step-cli is not open")
        users = self._realm_users(event_id)
        voted = self._voters_with_votes(event_id)
        portal = Portal(event_id, self.cli)
        for vote in scenario.votes:
            username = voter_username(scenario, vote)
            if users.get(username) in voted:
                continue
            token = portal.login(username)
            files = portal.status(token)["get_ballot_files_urls"]["files"]
            if len(files) != 1:
                raise ScenarioError(f"{username} has {len(files)} ballots, expected 1")
            # The published style, read here rather than from the signed URL,
            # which names the host the browser uses.
            (row,) = self.admin.query(BALLOT_STYLE, {"id": files[0]["id"]})[
                "sequent_backend_ballot_style"
            ]
            ballot = portal.encrypt(json.loads(row["ballot_eml"]), vote.candidate)
            receipt, errors = portal.cast(token, ballot)
            if errors or not receipt:
                raise ScenarioError(f"the vote of {username} was refused: {errors}")
            self.say(f"    {username} voted for {vote.candidate}")

    def _tally(self, scenario: Scenario, event_id: str) -> None:
        sessions = self._tally_sessions(event_id)
        active = [
            session
            for session in sessions
            if session["execution_status"] not in FAILED_EXECUTIONS
        ]
        if not active and sessions:
            latest = sessions[0]
            raise ScenarioError(
                f"tally {latest['id']} is {latest['execution_status']}; see docker "
                f"logs {self.checkout.name_prefix}windmill, then "
                f"scripts/dev/step-dev scenario reset {scenario.name}"
            )
        if active:
            session_id = active[0]["id"]
        else:
            session_id = StepCli.last_id(
                self._step(
                    "start-tally",
                    "--election-event-id",
                    event_id,
                    "--tally-type",
                    TALLY_TYPE,
                )
            )

        def finished() -> tuple[bool, str]:
            execution = next(
                (
                    session["execution_status"]
                    for session in self._tally_sessions(event_id)
                    if session["id"] == session_id
                ),
                "absent",
            )
            if execution in FAILED_EXECUTIONS:
                raise ScenarioError(f"tally {session_id} {execution}")
            stored = self._tally_execution(session_id) is not None
            return execution == SUCCESS and stored, execution

        self._wait(
            "the electoral results tally",
            finished,
            "tally",
            hint=f"check docker logs {self.checkout.name_prefix}trustee1",
        )

    def _publish_results(self, scenario: Scenario, state: State, event_id: str) -> None:
        tallied = [
            session
            for session in self._tally_sessions(event_id)
            if session["execution_status"] == SUCCESS
        ]
        execution = self._tally_execution(tallied[0]["id"]) if tallied else None
        if execution is None:
            raise ScenarioError(
                f"event {event_id} has no stored tally results; "
                f"scripts/dev/step-dev scenario reset {scenario.name}"
            )
        if not state.ids:
            self._record_ids(state, event_id)
        latest = self._results_publication(event_id)
        if latest is None or latest["publication_status"] != PUBLISHING:
            self._publish_tally(event_id, state, tallied[0]["id"], execution)

        def published() -> tuple[bool, str]:
            publication = self._results_publication(event_id)
            if publication is None:
                return False, "no publication"
            status = publication["publication_status"]
            if status == FAILED_PUBLICATION:
                raise ScenarioError(
                    f"results publication {publication['id']} failed: "
                    f"{publication['error_message']}"
                )
            return status == PUBLISHED, status

        self._wait("the results publication", published, "results")

    def _publish_tally(
        self, event_id: str, state: State, session_id: str, execution: dict[str, Any]
    ) -> None:
        self._step(
            "configure-results-website",
            "--election-event-id",
            event_id,
            *RESULTS_ACCESS,
            *RESULTS_SCOPE,
        )
        elections = sorted(set(state.ids["elections"].values()))
        contests = sorted(set(state.ids["contests"].values()))
        self._step(
            "publish-results",
            "--election-event-id",
            event_id,
            "--tally-session-id",
            session_id,
            "--tally-session-execution-id",
            execution["id"],
            "--results-event-id",
            execution["results_event_id"],
            *(
                argument
                for election in elections
                for argument in ("--election-id", election)
            ),
            *(
                argument
                for contest in contests
                for argument in ("--contest-id", contest)
            ),
        )

    def delete_event(self, event_id: str) -> None:
        self._step("delete-election-event", "--election-event-id", event_id)

        def gone() -> tuple[bool, str]:
            row = self._event(event_id) is not None
            realm = self._realm_exists(event_id)
            return not row and not realm, f"event row {row}, realm {realm}"

        self._wait(f"the deletion of event {event_id}", gone, "delete")
