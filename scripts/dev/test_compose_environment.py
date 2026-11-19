# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Compose configuration comes from files and defaults, without shell exports."""

import ipaddress
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MIRU_IDENTITY = (
    "MIRU_DEVICE_ID",
    "MIRU_SERIAL_NUMBER",
    "MIRU_IP_ADDRESS",
    "MIRU_MAC_ADDRESS",
)


def development_identity():
    """The demo transmission identity .env.development documents."""
    values = dict(
        line.split("=", 1)
        for line in (DEVCONTAINER / ".env.development").read_text().splitlines()
        if line.partition("=")[0] in MIRU_IDENTITY
    )
    return {name: values[name] for name in MIRU_IDENTITY}
DEVCONTAINER = ROOT / ".devcontainer"


@unittest.skipUnless(shutil.which("docker"), "Docker Compose is required")
class ComposeEnvironmentTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.project = self.root / ".devcontainer"
        self.project.mkdir()
        for source in DEVCONTAINER.glob("docker-compose*.yml"):
            shutil.copy2(source, self.project / source.name)
        shutil.copy2(
            DEVCONTAINER / ".env.development", self.project / ".env.development"
        )
        # Keep only process/tool lookup settings, never application variables.
        self.environment = {
            key: os.environ[key] for key in ("PATH", "HOME") if key in os.environ
        }

    def render(self, files, env_source, extra_env=None):
        configuration = env_source.read_text()
        # Remote setup generates this secret into .env before Compose runs.
        configuration = configuration.replace(
            "MESSAGING_RELAY_SECRET=\n",
            "MESSAGING_RELAY_SECRET=synthetic-compose-test-secret\n",
        )
        env_file = self.project / ".env"
        env_file.write_text(configuration)
        command = [
            "docker",
            "compose",
            "--project-directory",
            str(self.project),
            "--env-file",
            str(env_file),
        ]
        if extra_env is not None:
            override = self.project / "run.env"
            override.write_text(extra_env)
            command.extend(["--env-file", str(override)])
        for filename in files:
            command.extend(["--file", str(self.project / filename)])
        result = subprocess.run(
            [*command, "--profile", "*", "config", "--format", "json"],
            env=self.environment,
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def render_with_older_local_env(self, overrides="", also_missing=()):
        missing = {
            "MESSAGING_WEBHOOKS_PUBLIC_URL",
            "MESSAGING_RELAY_SECRET",
            "MESSAGING_WEBHOOKS_PORT",
            "ADMIN_PORTAL_URL",
            "TRUSTEE_KEYCLOAK_CLIENT_ID",
            "TRUSTEE_KEYCLOAK_CLIENT_SECRET",
            *also_missing,
        }
        configuration = (
            "\n".join(
                line
                for line in (DEVCONTAINER / ".env.development").read_text().splitlines()
                if line.partition("=")[0] not in missing
            )
            + "\n"
            + overrides
        )
        local = self.project / ".env"
        local.write_text(configuration)
        result = subprocess.run(
            ["docker", "compose", "--profile", "*", "config", "--format", "json"],
            cwd=self.project,
            env=self.environment,
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("variable is not set", result.stderr)
        self.assertEqual(local.read_text(), configuration)
        return json.loads(result.stdout)["services"]

    def test_plain_compose_inherits_settings_missing_from_older_local_env(self):
        services = self.render_with_older_local_env()
        messaging = services["messaging-webhooks"]
        self.assertEqual(messaging["ports"][0]["target"], 8410)
        self.assertEqual(messaging["ports"][0]["published"], "8410")
        harvest = services["harvest"]["environment"]
        self.assertEqual(
            services["windmill"]["environment"]["ADMIN_PORTAL_URL"],
            "http://127.0.0.1:3002",
        )
        self.assertEqual(
            harvest["MESSAGING_WEBHOOKS_PUBLIC_URL"], "http://messaging-webhooks:8410"
        )
        self.assertEqual(
            harvest["MESSAGING_RELAY_SECRET"],
            "dev-messaging-relay-secret-not-for-production",
        )
        for trustee in ("trustee1", "trustee2"):
            environment = services[trustee]["environment"]
            self.assertEqual(
                environment["TRUSTEE_PLATFORM_URL"],
                "http://graphql-engine:8080/v1/graphql",
            )
            self.assertEqual(
                environment["TRUSTEE_KEYCLOAK_CLIENT_ID"], "api-key-client"
            )
            self.assertTrue(environment["TRUSTEE_KEYCLOAK_CLIENT_SECRET"])

    def test_plain_compose_preserves_local_values_and_explicit_empty_settings(self):
        services = self.render_with_older_local_env(
            "MESSAGING_WEBHOOKS_PORT=18410\n"
            "MESSAGING_WEBHOOKS_PUBLIC_URL=\n"
            "MESSAGING_RELAY_SECRET=synthetic-local-relay-secret\n"
            "ADMIN_PORTAL_URL=http://localhost:13002\n"
            "TRUSTEE_PLATFORM_URL=https://trustees.example.invalid/v1/graphql\n"
            "TRUSTEE_KEYCLOAK_CLIENT_ID=local-trustee-client\n"
            "TRUSTEE_KEYCLOAK_CLIENT_SECRET=synthetic-local-trustee-secret\n"
        )
        messaging = services["messaging-webhooks"]
        self.assertEqual(messaging["ports"][0]["target"], 18410)
        self.assertEqual(messaging["ports"][0]["published"], "18410")
        harvest = services["harvest"]["environment"]
        self.assertEqual(harvest["MESSAGING_WEBHOOKS_PUBLIC_URL"], "")
        self.assertEqual(
            harvest["MESSAGING_RELAY_SECRET"], "synthetic-local-relay-secret"
        )
        self.assertEqual(
            services["windmill"]["environment"]["ADMIN_PORTAL_URL"],
            "http://localhost:13002",
        )
        for trustee in ("trustee1", "trustee2"):
            environment = services[trustee]["environment"]
            self.assertEqual(
                environment["TRUSTEE_PLATFORM_URL"],
                "https://trustees.example.invalid/v1/graphql",
            )
            self.assertEqual(
                environment["TRUSTEE_KEYCLOAK_CLIENT_ID"], "local-trustee-client"
            )
            self.assertEqual(
                environment["TRUSTEE_KEYCLOAK_CLIENT_SECRET"],
                "synthetic-local-trustee-secret",
            )

    def assert_transmission_identity(self, services, expected=None):
        # Windmill and Harvest refuse to seal an ER transmission package
        # without all four (windmill services/consolidation/acm_json.rs).
        for name in ("harvest", "windmill"):
            environment = services[name]["environment"]
            for variable in MIRU_IDENTITY:
                with self.subTest(service=name, variable=variable):
                    value = environment.get(variable)
                    self.assertTrue(value, f"{name} has no {variable}")
                    if expected:
                        self.assertEqual(value, expected[variable])
            ipaddress.ip_address(environment["MIRU_IP_ADDRESS"])
            self.assertRegex(
                environment["MIRU_MAC_ADDRESS"],
                r"^[0-9A-Fa-f]{2}([:-][0-9A-Fa-f]{2}){5}$",
            )

    def test_default_compose_gives_the_apps_the_transmission_identity(self):
        model = self.render(["docker-compose.yml"], DEVCONTAINER / ".env.development")
        self.assert_transmission_identity(model["services"])

    def test_an_older_local_env_inherits_the_transmission_identity(self):
        services = self.render_with_older_local_env(also_missing=MIRU_IDENTITY)
        self.assert_transmission_identity(services)

    def test_a_local_transmission_identity_is_kept(self):
        local = {
            "MIRU_DEVICE_ID": "LOCAL-OVCS-0001",
            "MIRU_SERIAL_NUMBER": "LOCAL-SN-0001",
            "MIRU_IP_ADDRESS": "fd00::7",
            "MIRU_MAC_ADDRESS": "02-00-00-00-00-01",
        }
        services = self.render_with_older_local_env(
            "".join(f"{name}={value}\n" for name, value in local.items())
        )
        self.assert_transmission_identity(services, local)

    def test_a_blank_local_transmission_identity_falls_back_to_the_demo_one(self):
        # An explicitly empty value is kept by Compose for other settings, but
        # these four have a fallback: without them no package can be sealed.
        # Only the blank lines: no .env.development value is left to win.
        services = self.render_with_older_local_env(
            "".join(f"{name}=\n" for name in MIRU_IDENTITY),
            also_missing=MIRU_IDENTITY,
        )
        self.assert_transmission_identity(services, development_identity())

    def test_the_compose_fallback_is_the_development_identity(self):
        base = (DEVCONTAINER / "docker-compose-base.yml").read_text()
        for name, value in development_identity().items():
            with self.subTest(variable=name):
                self.assertEqual(
                    base.count(f"{name}: ${{{name}:-{value}}}"), 2, name
                )

    def test_deployment_configurations_render_without_shell_exports(self):
        for filename, env_source in (
            ("docker-compose.yml", DEVCONTAINER / ".env.development"),
            ("docker-compose-remote.yml", DEVCONTAINER / ".env.development"),
            (
                "docker-compose-airgap-preparation.yml",
                DEVCONTAINER / ".env.development",
            ),
            (
                "docker-compose-remote.yml",
                DEVCONTAINER / ".env.remote-deployment.example",
            ),
            (
                "docker-compose-airgap-preparation.yml",
                ROOT / "scripts/airgap-files/.env",
            ),
        ):
            with self.subTest(compose=filename):
                self.render([filename], env_source)

    def test_e2e_defaults_work_without_shell_exports(self):
        model = self.render(
            [
                "docker-compose.yml",
                "docker-compose-ci.yml",
                "docker-compose-ci-coverage.yml",
                "docker-compose-ui-e2e.yml",
            ],
            DEVCONTAINER / ".env.development",
        )
        for service in ("driver", "ui-browser"):
            volumes = {
                volume["target"]: Path(volume["source"])
                for volume in model["services"][service]["volumes"]
            }
            self.assertEqual(
                volumes["/opt/step-e2e/bin"], self.root / ".cache/backend-e2e/bin"
            )
        driver = model["services"]["driver"]["volumes"]
        output = next(
            volume["source"] for volume in driver if volume["target"] == "/out"
        )
        self.assertEqual(Path(output), self.root / ".cache/backend-e2e/default")

    def test_backend_rust_services_build_inside_the_checkout(self):
        model = self.render(["docker-compose.yml"], DEVCONTAINER / ".env.development")
        target = "/workspaces/step/packages/rust-local-target/docker-dev"
        for name in ("harvest", "windmill", "beat", "task-queues-setup"):
            with self.subTest(service=name):
                self.assertEqual(
                    model["services"][name]["environment"].get("CARGO_TARGET_DIR"),
                    target,
                )
        for name, binary in (("windmill", "main"), ("beat", "beat")):
            command = " ".join(model["services"][name]["command"])
            self.assertIn(f"../rust-local-target/docker-dev/debug/{binary}", command)
            self.assertNotIn("../target/", command)

    def test_e2e_run_paths_can_be_overridden_in_a_configuration_file(self):
        binaries = self.root / "owned-run/bin"
        output = self.root / "owned-run/output"
        model = self.render(
            [
                "docker-compose.yml",
                "docker-compose-ci.yml",
                "docker-compose-ci-coverage.yml",
            ],
            DEVCONTAINER / ".env.development",
            f"STEP_E2E_BIN_DIR={binaries}\nSTEP_E2E_OUTPUT_DIR={output}\n",
        )
        mounts = {
            volume["target"]: Path(volume["source"])
            for volume in model["services"]["driver"]["volumes"]
        }
        self.assertEqual(mounts["/opt/step-e2e/bin"], binaries)
        self.assertEqual(mounts["/out"], output)
        self.assertEqual(mounts["/profiles"], output / "coverage/profiles")


if __name__ == "__main__":
    unittest.main()
