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
import urllib.error

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


class FakeKeycloak:
    """The admin API calls the script makes. Like Keycloak, the clients list
    leaves secrets out, and an update keeps the attributes it doesn't name and
    removes the ones it sends empty."""

    def __init__(self, realms, secrets=None):
        self.realms = realms
        self.secrets = dict(secrets or {})
        self.secret_error = None
        self.calls = []

    def __call__(self, *args):
        return self

    def call(self, method, path, body=None):
        self.calls.append((method, path))
        parts = path.strip("/").split("/")
        clients = self.realms.get(parts[0], [])
        if method == "GET" and parts[1:] == ["clients"]:
            return [
                {k: v for k, v in json.loads(json.dumps(c)).items() if k != "secret"}
                for c in clients
            ]
        if method == "GET" and parts[3:] == ["client-secret"]:
            if self.secret_error:
                raise urllib.error.HTTPError(path, self.secret_error, "error", {}, None)
            return {"type": "secret", "value": self.secrets.get(parts[2], "")}
        if method == "PUT" and len(parts) == 3:
            (stored,) = [c for c in clients if c["id"] == parts[2]]
            attributes = {**stored.get("attributes", {}), **body.get("attributes", {})}
            stored.update(body)
            stored["attributes"] = {k: v for k, v in attributes.items() if v != ""}
            if "secret" in body:
                self.secrets[parts[2]] = body["secret"]
            return None
        if method in ("POST", "PUT"):
            return None
        raise AssertionError(f"unexpected call {method} {path}")


def run_realm_mode(keycloak, *args, **extra_env):
    env = {
        "KEYCLOAK_URL": "u",
        "KEYCLOAK_ADMIN": "a",
        "KEYCLOAK_ADMIN_PASSWORD": "p",
        "KEYCLOAK_CLI_CLIENT_SECRET": "",
        **extra_env,
    }
    stderr = io.StringIO()
    with mock.patch.dict(os.environ, env), mock.patch.object(
        harden, "KeycloakAdmin", keycloak
    ), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(stderr):
        status = harden.main(["realm", *args])
    return status, stderr.getvalue()


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
            post_logout = "*" if hardened["clientId"] in harden.FINISH_URL_CLIENT_IDS else "+"
            self.assertEqual(hardened["attributes"]["post.logout.redirect.uris"], post_logout)
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
        keycloak = FakeKeycloak(
            {
                "tenant-bad": [client("admin-portal", id="a", redirectUris=["*"])],
                "tenant-good": [client("voting-portal", id="v")],
            }
        )
        status, stderr = run_realm_mode(
            keycloak, "--apply", "--realm", "tenant-bad", "--realm", "tenant-good"
        )
        self.assertEqual(status, 1)
        self.assertIn("tenant-bad: admin-portal accepts any redirect URI", stderr)
        self.assertEqual(
            [call for call in keycloak.calls if call[0] != "GET"],
            [("PUT", "/tenant-good/clients/v")],
        )

    def test_realm_run_keeps_the_stored_cli_secret_and_settles(self):
        keycloak = FakeKeycloak(
            {"tenant-t": [client("cli-account-admin", id="c", protocolMappers=[])]},
            secrets={"c": "kept"},
        )
        self.assertEqual(run_realm_mode(keycloak, "--apply", "--realm", "tenant-t")[0], 0)
        self.assertEqual(keycloak.secrets["c"], "kept")
        self.assertFalse(keycloak.realms["tenant-t"][0]["publicClient"])
        keycloak.calls.clear()
        self.assertEqual(run_realm_mode(keycloak, "--apply", "--realm", "tenant-t")[0], 0)
        self.assertEqual([call for call in keycloak.calls if call[0] != "GET"], [])

    def test_realm_run_generates_a_cli_secret_only_when_none_is_stored(self):
        keycloak = FakeKeycloak(
            {"tenant-t": [client("cli-account-admin", id="c", protocolMappers=[])]}
        )
        run_realm_mode(keycloak, "--apply", "--realm", "tenant-t")
        generated = keycloak.secrets["c"]
        self.assertEqual(len(generated), 32)
        run_realm_mode(keycloak, "--apply", "--realm", "tenant-t")
        self.assertEqual(keycloak.secrets["c"], generated)

    def test_realm_run_uses_the_configured_cli_secret_when_none_is_stored(self):
        keycloak = FakeKeycloak(
            {"tenant-t": [client("cli-account-admin", id="c", protocolMappers=[])]}
        )
        run_realm_mode(
            keycloak, "--apply", "--realm", "tenant-t", KEYCLOAK_CLI_CLIENT_SECRET="configured"
        )
        self.assertEqual(keycloak.secrets["c"], "configured")

    def test_realm_run_stops_a_realm_whose_secret_cannot_be_read(self):
        keycloak = FakeKeycloak(
            {
                "tenant-a": [client("cli-account-admin", id="c", protocolMappers=[])],
                "tenant-b": [client("voting-portal", id="v")],
            },
            secrets={"c": "kept"},
        )
        keycloak.secret_error = 500
        status, stderr = run_realm_mode(
            keycloak, "--apply", "--realm", "tenant-a", "--realm", "tenant-b",
            KEYCLOAK_CLI_CLIENT_SECRET="configured",
        )
        self.assertEqual(status, 1)
        self.assertIn("tenant-a: stopped", stderr)
        self.assertEqual(keycloak.secrets["c"], "kept")
        self.assertEqual(
            [call for call in keycloak.calls if call[0] != "GET"],
            [("PUT", "/tenant-b/clients/v")],
        )

    def test_realm_run_clears_removed_attributes(self):
        keycloak = FakeKeycloak({"tenant-t": [client("admin-portal", id="a")]})
        run_realm_mode(keycloak, "--apply", "--realm", "tenant-t")
        self.assertNotIn("request.uris", keycloak.realms["tenant-t"][0]["attributes"])

    def test_keycloak_requests_send_their_own_user_agent(self):
        sent = []

        def urlopen(request):
            sent.append(request)
            if request.full_url.endswith("/protocol/openid-connect/token"):
                return io.BytesIO(b'{"access_token": "token"}')
            return io.BytesIO(b"[]")

        admin = harden.KeycloakAdmin("https://keycloak.example.test/auth", "admin", "password")
        with mock.patch.object(harden.urllib.request, "urlopen", urlopen):
            self.assertEqual(admin.call("GET", ""), [])

        self.assertEqual(
            [request.full_url for request in sent],
            [
                "https://keycloak.example.test/auth/realms/master/protocol/openid-connect/token",
                "https://keycloak.example.test/auth/admin/realms",
            ],
        )
        for request in sent:
            self.assertEqual(request.get_header("User-agent"), harden.USER_AGENT)

    def test_development_templates_keep_portal_clients_public(self):
        for path in TEMPLATES:
            realm = json.loads(path.read_text(encoding="utf-8"))
            for portal in realm["clients"]:
                if portal["clientId"] in harden.PUBLIC_BROWSER_CLIENT_IDS:
                    self.assertTrue(portal["publicClient"], (path.name, portal["clientId"]))
                    self.assertTrue(portal["standardFlowEnabled"], (path.name, portal["clientId"]))

    def test_development_templates_are_already_hardened(self):
        for path in TEMPLATES:
            realm = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(
                harden.harden_clients(realm["clients"], realm["realm"]), [], path.name
            )


if __name__ == "__main__":
    unittest.main()
