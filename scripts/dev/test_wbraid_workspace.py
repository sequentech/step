# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The workspace launches native wbraid trustees without rebuilding its editor."""

import json
import shlex
import tempfile
import unittest
from pathlib import Path

from scripts.dev.mode.checkout import load_checkout
from scripts.dev.mode.docker import Compose
from scripts.dev.mode.manifest import load_manifest
from scripts.dev.test_mode import ROOT, compose_available, compose_services, load_jsonc


@unittest.skipUnless(compose_available(), "Docker Compose is required")
class WorkspaceTrusteesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.services = compose_services(["docker-compose.yml"])
        cls.manifest = load_manifest(ROOT)

    def test_backend_modes_start_both_native_trustees(self):
        for mode in ("backend", "full"):
            for trustee in ("trustee1", "trustee2"):
                with self.subTest(mode=mode, trustee=trustee):
                    self.assertIn(trustee, self.manifest.mode(mode).services)
                    self.assertIn("base", self.services[trustee]["profiles"])

    def test_watchers_use_locked_optimized_builds_in_the_checkout(self):
        for name in ("trustee1", "trustee2", "b4v6"):
            with self.subTest(service=name):
                service = self.services[name]
                command = service.get("command") or service["entrypoint"]
                arguments = shlex.split(command[command.index("-x") + 1])
                self.assertIn("--release", arguments)
                self.assertIn("--locked", arguments)
                self.assertEqual(
                    service["environment"].get("CARGO_TARGET_DIR"),
                    "/workspaces/step/packages/rust-local-target/docker-dev",
                )

    def test_signed_storage_urls_work_without_browser_hosts_file_changes(self):
        self.assertEqual(
            self.services["b4v6"]["environment"]["AWS_ENDPOINT_URL"],
            "http://localhost.localstack.cloud:4566",
        )
        self.assertIn(
            "localhost.localstack.cloud",
            self.services["localstack"]["networks"]["default"]["aliases"],
        )

    def test_trustee_identity_store_and_service_addresses_match_the_runner(self):
        for name in ("trustee1", "trustee2"):
            with self.subTest(trustee=name):
                service = self.services[name]
                environment = service["environment"]
                self.assertEqual(environment["TRUSTEE_NAME"], name)
                self.assertEqual(environment["B4_URL"], "http://b4v6:3005")
                self.assertEqual(
                    environment["TRUSTEE_PLATFORM_URL"],
                    "http://graphql-engine:8080/v1/graphql",
                )
                self.assertNotIn("TRUSTEE_HARVEST_URL", environment)
                self.assertEqual(
                    environment["TRUSTEE_KEYCLOAK_URL"], "http://keycloak:8090"
                )
                self.assertEqual(
                    environment["TRUSTEE_KEYCLOAK_CLIENT_ID"], "api-key-client"
                )
                self.assertEqual(
                    environment["TRUSTEE_CONFIG_PATH"],
                    f"{service['working_dir']}/dev/{name}.toml",
                )
                stores = [
                    v
                    for v in service["volumes"]
                    if v["target"].endswith("/message_store")
                ]
                self.assertEqual(len(stores), 1)
                self.assertEqual(stores[0]["source"], f"{name}_data")
                self.assertEqual(
                    stores[0]["target"], f"{service['working_dir']}/message_store"
                )
                self.assertEqual(
                    service["depends_on"]["b4v6"]["condition"], "service_healthy"
                )
        self.assertEqual(
            self.services["b4v6"]["environment"]["AWS_ENDPOINT_URL"],
            "http://localhost.localstack.cloud:4566",
        )


class WorkspaceEnvironmentTest(unittest.TestCase):
    def test_browser_trustee_accounts_have_distinct_matching_identities(self):
        realm = json.loads(
            (
                ROOT
                / ".devcontainer/keycloak/import"
                / "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
            ).read_text()
        )
        users = {user["username"]: user for user in realm["users"]}
        for name in ("trustee1-wbraid", "trustee2-wbraid"):
            with self.subTest(name=name):
                self.assertIn(name, users)
                user = users[name]
                self.assertEqual(user["attributes"]["trustee"], [name])
                self.assertEqual(
                    user["attributes"]["tenant-id"],
                    ["90505c8a-23a9-4cdf-a26b-4e19f6a097d5"],
                )
                self.assertIn("/trustee", user["groups"])
                self.assertTrue(
                    user["firstName"] and user["lastName"] and user["email"]
                )
        for name in ("trustee1", "trustee2"):
            self.assertEqual(users[name]["attributes"]["trustee"], [name])

    def test_browser_service_ports_cannot_be_silently_forwarded_to_other_ports(self):
        for path in ("devcontainer.json", "backend/devcontainer.json"):
            config = load_jsonc(ROOT / ".devcontainer" / path)
            with self.subTest(config=path):
                self.assertIn("hasura:8080", config["forwardPorts"])
                self.assertIn("b4v6:3005", config["forwardPorts"])
                self.assertIn("localstack:4566", config["forwardPorts"])
                for port in (3005, 4566, 8080, 8090):
                    self.assertTrue(
                        config["portsAttributes"][str(port)]["requireLocalPort"]
                    )
                if path == "devcontainer.json":
                    self.assertIn(3002, config["forwardPorts"])

    def test_existing_local_env_inherits_new_defaults_without_being_rewritten(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            compose = root / ".devcontainer"
            compose.mkdir()
            defaults = compose / ".env.development"
            defaults.write_text(
                "COMPOSE_PROJECT_NAME=default\n"
                "TRUSTEE_KEYCLOAK_CLIENT_ID=api-key-client\n"
                "MESSAGING_WEBHOOKS_PORT=8086\n"
            )
            local = compose / ".env"
            original = (
                "COMPOSE_PROJECT_NAME=owned-workspace\n"
                "TRUSTEE_KEYCLOAK_CLIENT_SECRET=local-secret\n"
            )
            local.write_text(original)
            checkout = load_checkout(root)
            self.assertEqual(checkout.project, "owned-workspace")
            self.assertEqual(
                checkout.env["TRUSTEE_KEYCLOAK_CLIENT_ID"], "api-key-client"
            )
            self.assertEqual(
                checkout.env["TRUSTEE_KEYCLOAK_CLIENT_SECRET"], "local-secret"
            )
            self.assertEqual(local.read_text(), original)
            command = Compose(checkout, ("docker-compose.yml",)).argv("config")
            env_files = [
                command[index + 1]
                for index, arg in enumerate(command)
                if arg == "--env-file"
            ]
            self.assertEqual(env_files, [str(defaults), str(local)])

    def test_empty_local_override_and_workspaces_without_defaults_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            compose = root / ".devcontainer"
            compose.mkdir()
            local = compose / ".env"
            local.write_text("COMPOSE_PROJECT_NAME=local\nOPTIONAL_SECRET=\n")
            checkout = load_checkout(root)
            self.assertEqual(checkout.env["OPTIONAL_SECRET"], "")
            self.assertNotIn(
                "--env-file", Compose(checkout, ("docker-compose.yml",)).argv("config")
            )
            (compose / ".env.development").write_text(
                "OPTIONAL_SECRET=default-secret\n"
            )
            self.assertEqual(load_checkout(root).env["OPTIONAL_SECRET"], "")


if __name__ == "__main__":
    unittest.main()
