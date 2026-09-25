# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Prepare the backend fixture without casting or closing any browser voter's ballot."""

import json
import unittest

from scripts.e2e.journeys import bootstrap, fixtures
from scripts.e2e.journeys.client import (
    ENV,
    OUTPUT,
    TENANT_ID,
    TENANT_REALM,
    event_realm,
)
from scripts.e2e.journeys.test_journeys import BackendJourneys


class BrowserFixtureJourneys(BackendJourneys):
    def import_event(self, document, name):
        # The exported template points at an external municipal logo. Use each
        # portal's bundled default logo; the browser must remain self-contained.
        document["election_event"]["presentation"].pop("logo_url", None)
        document["keycloak_event_realm"]["attributes"]["credential-input-policy"] = (
            "standard"
        )
        return super().import_event(document, name)

    def test_5_publish_ballot_styles(self):
        # Import normalizes portal clients to the backend's configured URL.
        # Register this run's browser origin before the publication login check.
        allow_portals(
            self.keycloak,
            event_realm(self.state.event_id),
            "voting-portal",
            [ENV["VOTING_PORTAL_URL"]],
        )
        super().test_5_publish_ballot_styles()


def allow_portals(keycloak, realm, client_id, origins):
    (client,) = keycloak.admin("GET", f"{realm}/clients?clientId={client_id}")
    client["redirectUris"] = [f"{origin}/*" for origin in origins]
    client["webOrigins"] = origins
    keycloak.admin("PUT", f"{realm}/clients/{client['id']}", client, expect=(204,))


def main():
    origins = json.loads((OUTPUT / "origins.json").read_text())
    ENV["VOTING_PORTAL_URL"] = origins["voting"]
    # These fixture journeys create and independently check real service state.
    # Stop before voting: all casts, audits and receipts belong to the browsers.
    methods = [
        "test_1_super_tenant_bootstrap",
        "test_2_import_election_event",
        "test_3_import_voters",
        "test_4_automatic_key_ceremony",
        "test_5_publish_ballot_styles",
    ]
    result = unittest.TextTestRunner(verbosity=2).run(
        unittest.TestSuite(BrowserFixtureJourneys(method) for method in methods)
    )
    if not result.wasSuccessful() or result.skipped:
        raise SystemExit("UI fixture preparation failed")
    fixture = BrowserFixtureJourneys(methods[-1])
    state = fixture.state
    keycloak = fixture.keycloak
    allow_portals(keycloak, TENANT_REALM, "admin-portal", [origins["admin"]])
    allow_portals(
        keycloak,
        event_realm(state.event_id),
        "voting-portal",
        [origins["voting"], origins["verifier"]],
    )
    # Exercise the actual two-factor form using Keycloak's isolated test mode.
    otp = bootstrap.email_otp_config(keycloak)
    otp["config"]["test-mode"] = "true"
    keycloak.admin(
        "PUT", f"{TENANT_REALM}/authentication/config/{otp['id']}", otp, expect=(204,)
    )
    fixture.step(
        "update-event-voting-status",
        "--election-event-id",
        state.event_id,
        "--voting-status",
        "OPEN",
        "--voting-channel",
        "ONLINE",
    )
    data = {
        "tenantId": TENANT_ID,
        "eventId": state.event_id,
        "eventName": fixtures.display_name(state.document["election_event"]),
        "elections": state.elections,
        "areas": state.areas,
        "contests": state.contests,
        "candidates": state.candidates,
        "voters": state.voters,
        "voterPassword": fixtures.VOTER_PASSWORD,
        "adminOtp": otp["config"].get("test-mode-code", "123456"),
        "origins": origins,
    }
    (OUTPUT / "fixture.json").write_text(json.dumps(data, indent=2))
    print(f"Browser fixture ready: {state.event_id}", flush=True)


if __name__ == "__main__":
    main()
