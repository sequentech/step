# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Exercise remote configuration against synthetic realm and deployment files."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / ".devcontainer/remote-deployment/configure-environment.sh"


@unittest.skipUnless(shutil.which("jq"), "jq is required by configure-environment.sh")
class TenantBootstrapConfigurationTests(unittest.TestCase):
    def configure(self, invalid_realm=False):
        """Run the complete configuration script inside an isolated deployment tree."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / ".devcontainer"
            script = root / "remote-deployment/configure-environment.sh"
            script.parent.mkdir(parents=True)
            script.write_text(SCRIPT.read_text())
            (root / "nginx").mkdir()
            (root / "nginx/default.conf.template").write_text(
                "server_name admin-fixture.${DOMAIN};\n"
            )
            names = [
                "MASTER_SECRET", "KEYCLOAK_CLIENT_SECRET", "KEYCLOAK_CLI_CLIENT_SECRET",
                "HASURA_GRAPHQL_ADMIN_SECRET", "AWS_S3_ROOT_PASSWORD", "AWS_S3_ACCESS_SECRET",
                "KEYCLOAK_ADMIN_PASSWORD", "KEYCLOAK_ADMIN_CLIENT_SECRET",
            ]
            (root / ".env.remote-deployment.example").write_text(
                "\n".join(f"{name}=synthetic-before" for name in names) + "\n"
            )
            realm_path = root / "keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
            realm_path.parent.mkdir(parents=True)
            realm = {
                "users": [
                    {"username": "admin", "groups": ["/admin"], "requiredActions": ["VERIFY_EMAIL"],
                     "credentials": [{"type": "password", "value": "synthetic-password"}]},
                    {"username": "trustee1", "groups": ["/trustee"],
                     "credentials": [{"type": "otp", "secretData": "synthetic-otp"}]},
                    {"username": "custom-service-principal", "serviceAccountClientId": "service-account", "enabled": True},
                ],
                "clients": [
                    {"clientId": "service-account", "serviceAccountsEnabled": True, "secret": "synthetic-before"},
                    {"clientId": "cli-account-admin", "serviceAccountsEnabled": True, "secret": "synthetic-before"},
                ],
            }
            realm_path.write_text("{" if invalid_realm else json.dumps(realm))
            result = subprocess.run(
                ["bash", str(script), "example.invalid", "test"],
                env={"PATH": os.environ.get("PATH") or os.defpath},
                capture_output=True, text=True, timeout=30,
            )
            prepared = None if invalid_realm else json.loads(realm_path.read_text())
            return result, prepared

    def test_prepared_realm_has_no_user_credentials_and_keeps_service_clients(self):
        """Remove imported credentials while retaining user metadata and client authentication."""
        result, realm = self.configure()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(all("credentials" not in user for user in realm["users"]))
        admin = realm["users"][0]
        self.assertEqual(admin["groups"], ["/admin"])
        self.assertEqual(set(admin["requiredActions"]), {"UPDATE_PASSWORD", "VERIFY_EMAIL"})
        self.assertEqual(realm["users"][2]["serviceAccountClientId"], "service-account")
        self.assertTrue(all(client["serviceAccountsEnabled"] for client in realm["clients"]))
        self.assertTrue(all(client["secret"] != "synthetic-before" for client in realm["clients"]))

    def test_invalid_realm_aborts_configuration(self):
        """Never continue provisioning if the realm's credential removal failed."""
        result, _ = self.configure(invalid_realm=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json", result.stderr)


if __name__ == "__main__":
    unittest.main()
