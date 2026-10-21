# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Defaults in direct imports, generated bundles and remote preparation."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
TENANT = "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5"


class ApiKeyClientTemplates(unittest.TestCase):
    def test_direct_import_disables_api_client_and_requests_generated_secret(self):
        realm = json.loads((ROOT / f".devcontainer/keycloak/import/{TENANT}.json").read_text())
        (client,) = [c for c in realm["clients"] if c["clientId"] == "api-key-client"]
        self.assertIs(client["enabled"], False)
        self.assertNotIn("secret", client)
        for other in ["admin-portal", "service-account"]:
            self.assertTrue(next(c for c in realm["clients"] if c["clientId"] == other)["enabled"])

    def test_event_template_does_not_add_api_client(self):
        (path,) = (ROOT / ".devcontainer/keycloak/import").glob(f"{TENANT}-event-*.json")
        realm = json.loads(path.read_text())
        self.assertNotIn("api-key-client", [c["clientId"] for c in realm["clients"]])

    def test_janitor_bundle_uses_same_disabled_default(self):
        text = (ROOT / "packages/windmill/external-bin/janitor/templates/COMELEC/keycloakAdmin.hbs").read_text()
        client = text.split('"clientId": "api-key-client"', 1)[1].split('"clientId"', 1)[0]
        self.assertRegex(client, r'"enabled"\s*:\s*false')
        self.assertNotRegex(client, r'"secret"\s*:')

    def test_tenant_and_event_admin_actions_on_without_request_details(self):
        paths = list((ROOT / ".devcontainer/keycloak/import").glob(f"{TENANT}*.json"))
        self.assertEqual(len(paths), 2)
        for path in paths:
            with self.subTest(path=path.name):
                realm = json.loads(path.read_text())
                self.assertIs(realm["adminEventsEnabled"], True)
                self.assertIs(realm["adminEventsDetailsEnabled"], False)
        for name in ["keycloakAdmin.hbs", "keycloak.hbs"]:
            text = (ROOT / "packages/windmill/external-bin/janitor/templates/COMELEC" / name).read_text()
            self.assertRegex(text, r'"adminEventsEnabled"\s*:\s*true')
            self.assertRegex(text, r'"adminEventsDetailsEnabled"\s*:\s*false')

    @unittest.skipUnless(shutil.which("jq"), "remote preparation requires jq")
    def test_remote_preparation_sanitizes_legacy_enabled_import(self):
        script = ROOT / ".devcontainer/remote-deployment/configure-environment.sh"
        if not script.exists():
            self.skipTest("this branch has no remote preparation script")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / ".devcontainer"
            target = root / "remote-deployment/configure-environment.sh"
            target.parent.mkdir(parents=True)
            target.write_text(script.read_text())
            (root / "nginx").mkdir()
            (root / "nginx/default.conf.template").write_text("server_name admin-fixture.${DOMAIN};\n")
            (root / ".env.remote-deployment.example").write_text("MASTER_SECRET=synthetic\n")
            realm_path = root / f"keycloak/import/{TENANT}.json"
            realm_path.parent.mkdir(parents=True)
            realm_path.write_text(json.dumps({"users": [], "adminEventsEnabled": False, "adminEventsDetailsEnabled": True, "eventsEnabled": False, "clients": [
                {"clientId": "api-key-client", "enabled": True, "secret": "synthetic-imported"},
                {"clientId": "service-account", "enabled": True},
                {"clientId": "cli-account-admin", "enabled": True}
            ]}))
            result = subprocess.run(["bash", str(target), "example.invalid", "fixture"],
                env={"PATH": os.environ.get("PATH", os.defpath)}, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            prepared = json.loads(realm_path.read_text())
            self.assertIs(prepared["adminEventsEnabled"], True)
            self.assertIs(prepared["adminEventsDetailsEnabled"], False)
            self.assertIs(prepared["eventsEnabled"], False)
            clients = prepared["clients"]
            self.assertIs(clients[0]["enabled"], False)
            self.assertNotIn("secret", clients[0])
            self.assertTrue(clients[1]["enabled"])
            self.assertTrue(clients[1]["secret"])


if __name__ == "__main__":
    unittest.main()
