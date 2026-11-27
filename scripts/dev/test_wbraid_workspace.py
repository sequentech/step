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
                # The board is reached through Harvest's authenticated forward.
                self.assertEqual(environment["B4_URL"], "http://harvest:8400/b4")
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
                    service["depends_on"]["harvest"]["condition"], "service_started"
                )
        self.assertEqual(
            self.services["b4v6"]["environment"]["AWS_ENDPOINT_URL"],
            "http://localhost.localstack.cloud:4566",
        )


WBRAID = ROOT / "packages" / "wbraid"


def storage_credentials(service):
    environment = service["environment"]
    return environment["AWS_ACCESS_KEY_ID"], environment["AWS_SECRET_ACCESS_KEY"]


@unittest.skipUnless(compose_available(), "Docker Compose is required")
class WbraidStorageTest(unittest.TestCase):
    """b4v6 message bodies outlive a restart of the S3 service (meta#15323)."""

    @classmethod
    def setUpClass(cls):
        cls.services = compose_services(["docker-compose.yml"])
        cls.storage = cls.services["localstack"]

    def test_bodies_are_kept_in_a_named_volume(self):
        volumes = [
            volume for volume in self.storage["volumes"] if volume["target"] == "/data"
        ]
        self.assertEqual(len(volumes), 1)
        self.assertEqual(volumes[0]["type"], "volume")
        self.assertEqual(volumes[0]["source"], "wbraid_storage")

    def test_storage_is_the_pinned_minio_server_on_the_localstack_port(self):
        self.assertEqual(self.storage["image"], self.services["minio"]["image"])
        command = self.storage["command"]
        self.assertEqual(command[:3], ["server", "--address", ":4566"])
        self.assertEqual(command[-1], "/data")
        self.assertEqual(self.storage["ports"][0]["target"], 4566)
        self.assertIn("/minio/health/", " ".join(self.storage["healthcheck"]["test"]))

    def test_board_and_bucket_step_use_the_storage_root_credentials(self):
        environment = self.storage["environment"]
        root = environment["MINIO_ROOT_USER"], environment["MINIO_ROOT_PASSWORD"]
        # MinIO refuses to start with a root password shorter than 8 characters.
        self.assertGreaterEqual(len(root[1]), 8)
        for name in ("b4v6", "configure-localstack"):
            with self.subTest(service=name):
                self.assertEqual(storage_credentials(self.services[name]), root)

    def test_bucket_step_creates_the_bucket_only_when_missing(self):
        step = self.services["configure-localstack"]
        script = " ".join(step["command"])
        self.assertIn("head-bucket --bucket wbraid-messages", script)
        self.assertIn("create-bucket --bucket wbraid-messages", script)
        # MinIO answers put-bucket-cors with NotImplemented, which would fail the
        # step and keep b4v6 from starting; it allows every origin already.
        self.assertNotIn("cors", script)
        self.assertNotIn("volumes", step)
        self.assertEqual(
            self.services["b4v6"]["depends_on"]["configure-localstack"]["condition"],
            "service_completed_successfully",
        )

    def test_e2e_overlay_keeps_the_storage_and_its_credentials(self):
        services = compose_services(["docker-compose.yml", "docker-compose-ci.yml"])
        self.assertEqual(
            services["b4v6"]["environment"]["AWS_ENDPOINT_URL"],
            "http://localstack:4566",
        )
        self.assertEqual(
            storage_credentials(services["b4v6"]),
            storage_credentials(self.services["b4v6"]),
        )
        self.assertEqual(services["localstack"]["volumes"], self.storage["volumes"])

    def test_scripts_outside_compose_start_the_same_storage(self):
        image = self.storage["image"]
        user, password = storage_credentials(self.services["b4v6"])
        for name in ("localstack.sh", "localstack.ps1", "b4.sh", "b4.ps1"):
            with self.subTest(script=name):
                text = (WBRAID / name).read_text()
                expected = [f'"{user}"', f'"{password}"']
                if name.startswith("localstack"):
                    expected += [image, "wbraid_storage:/data"]
                for fragment in expected:
                    self.assertTrue(fragment in text, f"{name} lacks {fragment}")
                for fragment in ('"test"', "put-bucket-cors"):
                    self.assertFalse(fragment in text, f"{name} has {fragment}")


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
