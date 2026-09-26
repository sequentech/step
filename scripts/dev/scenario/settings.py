# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Endpoints, credentials and portal URLs from the checkout's devcontainer settings."""

from __future__ import annotations

import shutil
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from .catalog import Link

# What the backend journeys read, as the checkout's .devcontainer/.env names it.
REQUIRED = (
    "SUPER_ADMIN_TENANT_ID",
    "ENV_SLUG",
    "HASURA_ENDPOINT",
    "KEYCLOAK_URL",
    "KEYCLOAK_ADMIN",
    "KEYCLOAK_ADMIN_PASSWORD",
    "KEYCLOAK_ADMIN_CLIENT_SECRET",
    "KEYCLOAK_VOTER_GROUP_NAME",
    "ADMIN_PORTAL_TEST_USERNAME",
    "ADMIN_PORTAL_TEST_PASSWORD",
    "API_KEY_CLIENT_SECRET",
    "AWS_S3_PRIVATE_URI",
    "AWS_S3_PUBLIC_BUCKET",
    "AWS_S3_JWKS_CERTS_PATH",
    "B4_URL",
    "VOTING_PORTAL_URL",
    "BALLOT_VERIFIER_URL",
    "RESULTS_PORTAL_URL",
)
# The journeys use the names docker-compose-ci.yml gives its driver service.
JOURNEY_ALIASES = {
    "HASURA_ADMIN_SECRET": "KEYCLOAK_ADMIN_CLIENT_SECRET",
    "ADMIN_USERNAME": "ADMIN_PORTAL_TEST_USERNAME",
    "ADMIN_PASSWORD": "ADMIN_PORTAL_TEST_PASSWORD",
}
# Where the step-cli output of the journeys goes (scripts/e2e/journeys/client.py).
JOURNEY_OUTPUT = "STEP_E2E_OUTPUT_DIR"
# The admin portal's dev server (.devcontainer/modes.json) and the root URL of
# its Keycloak client.
ADMIN_PORTAL_URL = "http://127.0.0.1:3002"
ADMIN_EVENT_PATH = "sequent_backend_election_event"
STEP_CLI = "step-cli"
# Where .devcontainer/scripts/init-cli.sh builds it; devenv puts it on PATH.
STEP_CLI_BUILD = Path("packages/step-cli/rust-local-target/release/step-cli")
STEP_CLI_BUILD_COMMAND = "(cd packages/step-cli && cargo build --release)"
# The event realm clients the voting portal signs voters in with.
VOTING_CLIENT = "voting-portal"
KIOSK_CLIENT = "voting-portal-kiosk"
PORTAL_CLIENTS = (VOTING_CLIENT, KIOSK_CLIENT)


class SettingsError(RuntimeError):
    """The devcontainer settings lack what the scenarios need."""


def journey_environment(
    checkout: Mapping[str, str], process: Mapping[str, str], output: Path
) -> dict[str, str]:
    """The variables the journeys' clients read, process values first."""
    merged = {**checkout, **process}
    missing = [key for key in REQUIRED if not merged.get(key)]
    if missing:
        raise SettingsError(
            f"{', '.join(missing)} not set; run .devcontainer/scripts/"
            "initialize-command.sh on the host to write .devcontainer/.env"
        )
    values = {key: merged[key] for key in REQUIRED}
    for alias, source in JOURNEY_ALIASES.items():
        values[alias] = merged.get(alias) or merged[source]
    values[JOURNEY_OUTPUT] = str(output)
    return values


@dataclass(frozen=True)
class Portals:
    voting: str
    verifier: str
    results: str
    admin: str = ADMIN_PORTAL_URL

    @classmethod
    def from_environment(cls, environment: Mapping[str, str]) -> Portals:
        return cls(
            voting=environment["VOTING_PORTAL_URL"].rstrip("/"),
            verifier=environment["BALLOT_VERIFIER_URL"].rstrip("/"),
            results=environment["RESULTS_PORTAL_URL"].rstrip("/"),
        )

    def origins(self, client: str) -> list[str]:
        """Origins an event realm client must accept for these portals."""
        if client == KIOSK_CLIENT:
            return [self.voting]
        return [self.voting, self.verifier]

    def link(self, link: Link, tenant_id: str, event_id: str) -> str:
        event = f"/tenant/{tenant_id}/event/{event_id}"
        if link is Link.VOTING:
            return f"{self.voting}{event}/login"
        if link is Link.KIOSK:
            return f"{self.voting}{event}/login?kiosk"
        if link is Link.VERIFIER:
            return f"{self.verifier}{event}/start"
        if link is Link.RESULTS:
            return f"{self.results}/{event_id}"
        return f"{self.admin}/{ADMIN_EVENT_PATH}/{event_id}"


def accepts(client: Mapping[str, object], origins: list[str]) -> bool:
    """Whether a Keycloak client redirects to and serves every origin.

    Relative redirect URIs resolve against the client's root URL.
    """
    root = str(client.get("rootUrl") or "").rstrip("/")
    redirects = {
        f"{root}{uri}" if str(uri).startswith("/") else str(uri)
        for uri in client.get("redirectUris") or []
    }
    web_origins = {str(origin).rstrip("/") for origin in client.get("webOrigins") or []}
    return all(
        (f"{origin}/*" in redirects or "*" in redirects)
        and (origin in web_origins or "*" in web_origins or "+" in web_origins)
        for origin in origins
    )


def find_step_cli(explicit: str | None, root: Path, path: str | None) -> Path:
    if explicit:
        candidate = Path(explicit)
        if not candidate.is_file():
            raise SettingsError(f"--step-cli {candidate} does not exist")
        return candidate
    found = shutil.which(STEP_CLI, path=path)
    if found:
        return Path(found)
    built = root / STEP_CLI_BUILD
    if built.is_file():
        return built
    raise SettingsError(
        f"step-cli is not built: run {STEP_CLI_BUILD_COMMAND} in the devcontainer, "
        "as its post-create command does, or pass --step-cli"
    )
