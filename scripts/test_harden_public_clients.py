# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Check that the realm migration script hardens clients as upsert_realm does."""

import importlib.util
import contextlib
import io
import json
import os
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "harden_public_clients", ROOT / "scripts/keycloak/harden_public_clients.py"
)
harden = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(harden)

TEMPLATES = [
    ROOT / ".devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json",
    ROOT
    / ".devcontainer/keycloak/import/"
    "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json",
]


def client(client_id, **fields):
    return {
        "clientId": client_id,
        "publicClient": True,
        "directAccessGrantsEnabled": True,
        "implicitFlowEnabled": True,
        "standardFlowEnabled": True,
        "webOrigins": ["*"],
        "redirectUris": ["https://portal.example.test/*"],
        "attributes": {"post.logout.redirect.uris": "*", "request.uris": "*"},
        **fields,
    }


class HardenClientsTests(unittest.TestCase):
    def test_public_browser_clients_lose_password_and_implicit_grants(self):
        clients = [client(client_id) for client_id in harden.PUBLIC_BROWSER_CLIENT_IDS]
        harden.harden_clients(clients, "tenant-t")
        for hardened in clients:
            self.assertFalse(hardened["directAccessGrantsEnabled"], hardened["clientId"])
            self.assertFalse(hardened["implicitFlowEnabled"])
            self.assertTrue(hardened["standardFlowEnabled"])
            self.assertEqual(hardened["webOrigins"], ["+"])
            self.assertEqual(hardened["attributes"]["pkce.code.challenge.method"], "S256")
            self.assertEqual(hardened["attributes"]["post.logout.redirect.uris"], "+")
            self.assertNotIn("request.uris", hardened["attributes"])

    def test_account_console_redirects_to_its_realm(self):
        clients = [client("account-console", redirectUris=["*"])]
        harden.harden_clients(clients, "tenant-t")
        self.assertEqual(clients[0]["redirectUris"], ["/realms/tenant-t/account/*"])
        self.assertNotIn("request.uris", clients[0]["attributes"])

    def test_admin_portal_accepting_any_redirect_uri_needs_the_admin_portal_url(self):
        clients = [client("admin-portal", redirectUris=["*"]), client("voting-portal")]
        before = json.loads(json.dumps(clients))
        with self.assertRaisesRegex(harden.UnpinnedRedirectError, "--admin-portal-url"):
            harden.harden_clients(clients, "tenant-t")
        self.assertEqual(clients, before)

        harden.harden_clients(clients, "tenant-t", "https://admin.example.test/")
        self.assertEqual(clients[0]["redirectUris"], ["https://admin.example.test/*"])
        self.assertEqual(clients[0]["webOrigins"], ["+"])

    def test_voter_client_accepting_any_redirect_uri_leaves_the_realm_unchanged(self):
        clients = [client("voting-portal", redirectUris=["*"]), client("account-console")]
        before = json.loads(json.dumps(clients))
        with self.assertRaises(harden.UnpinnedRedirectError):
            harden.harden_clients(clients, "tenant-t", "https://admin.example.test")
        self.assertEqual(clients, before)

    def test_cli_account_admin_is_confidential_with_admin_portal_claims(self):
        clients = [
            client("admin-portal", protocolMappers=[{"id": "1", "name": "labels"}]),
            client("cli-account-admin", secret="kept", protocolMappers=[]),
        ]
        harden.harden_clients(clients, "tenant-t")
        cli = clients[1]
        self.assertFalse(cli["publicClient"])
        self.assertTrue(cli["directAccessGrantsEnabled"])
        self.assertFalse(cli["standardFlowEnabled"])
        self.assertEqual(cli["secret"], "kept")
        self.assertEqual(cli["protocolMappers"], [{"name": "labels"}])
        self.assertNotIn("request.uris", cli["attributes"])

    def test_cli_account_admin_gets_the_admin_portal_default_role(self):
        def default_role(role, **fields):
            return {
                "name": "x-hasura-default-role",
                "config": {"claim.value": role},
                **fields,
            }

        clients = [
            client("admin-portal", protocolMappers=[default_role("admin-user", id="1")]),
            client(
                "cli-account-admin",
                protocolMappers=[default_role("cli-account-admin", id="2")],
            ),
        ]
        before = json.loads(json.dumps(clients[1]))
        harden.harden_clients(clients, "tenant-t")
        cli = clients[1]
        self.assertEqual(cli["protocolMappers"], [default_role("admin-user", id="2")])
        self.assertEqual(harden.mapper_changes(before, cli), ([], cli["protocolMappers"]))

    def test_confidential_service_clients_are_unchanged(self):
        service = client("ivr-voting", publicClient=False)
        self.assertEqual(harden.harden_clients([service], "tenant-t"), [])

    def test_hardening_is_idempotent(self):
        clients = [client("admin-portal"), client("cli-account-admin")]
        harden.harden_clients(clients, "tenant-t", "https://admin.example.test")
        self.assertEqual(harden.harden_clients(clients, "tenant-t", "https://admin.example.test"), [])

    def test_realm_run_skips_only_the_realm_with_an_unpinned_client(self):
        realms = {
            "tenant-bad": [client("admin-portal", id="a", redirectUris=["*"])],
            "tenant-good": [client("voting-portal", id="v")],
        }
        calls = []

        class FakeAdmin:
            def __init__(self, *args):
                pass

            def call(self, method, path, body=None):
                calls.append((method, path))
                if method == "GET":
                    return json.loads(json.dumps(realms[path.split("/")[1]]))
                return None

        env = {"KEYCLOAK_URL": "u", "KEYCLOAK_ADMIN": "a", "KEYCLOAK_ADMIN_PASSWORD": "p"}
        argv = ["realm", "--apply", "--realm", "tenant-bad", "--realm", "tenant-good"]
        with mock.patch.dict(os.environ, env), mock.patch.object(
            harden, "KeycloakAdmin", FakeAdmin
        ), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(
            io.StringIO()
        ) as stderr:
            self.assertEqual(harden.main(argv), 1)
        self.assertIn("tenant-bad: admin-portal accepts any redirect URI", stderr.getvalue())
        self.assertEqual(
            [call for call in calls if call[0] != "GET"],
            [("PUT", "/tenant-good/clients/v")],
        )

    def test_development_templates_are_already_hardened(self):
        for path in TEMPLATES:
            realm = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(
                harden.harden_clients(realm["clients"], realm["realm"]), [], path.name
            )


if __name__ == "__main__":
    unittest.main()
