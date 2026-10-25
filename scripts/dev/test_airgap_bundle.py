# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The air-gap bundle renders and starts with its own `.env`.

`scripts/build-airgapped.sh` ships `scripts/airgap-files/{.env,
docker-compose.yml,up}` as they are, so a variable the compose file needs and
the bundle's `.env` lacks breaks every service of the next release, not only
the one that uses it.
"""

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BUNDLE = ROOT / "scripts" / "airgap-files"
PLACEHOLDER = "generate-on-first-up"
REFERENCE = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)([^}]*)\}")


def defined(env_text):
    return set(re.findall(r"^\s*([A-Za-z_][A-Za-z0-9_]*)=", env_text, re.M))


def service_block(compose_text, name):
    match = re.search(
        rf"^  {re.escape(name)}:\n(.*?)(?=^  [A-Za-z0-9_-]+:\n|^[A-Za-z]|\Z)",
        compose_text,
        re.M | re.S,
    )
    assert match, name
    return match.group(1)


def secret_preamble():
    """The part of `up` that runs before anything is removed or started."""
    text = (BUNDLE / "up").read_text()
    return text[: text.index("cd janitor")]


class AirgapBundleTest(unittest.TestCase):
    def setUp(self):
        self.env = (BUNDLE / ".env").read_text()
        self.compose = (BUNDLE / "docker-compose.yml").read_text()

    def test_published_ports_and_messaging_variables_are_in_the_bundle_env(self):
        names = defined(self.env)
        needed = set()
        for line in self.compose.splitlines():
            for name, modifier in REFERENCE.findall(line):
                if re.match(r":?[-?]", modifier):
                    continue
                if name.startswith("MESSAGING_") or line.strip().startswith("- ${"):
                    needed.add(name)
        self.assertEqual(sorted(needed - names), [])

    def test_airgap_trustees_keep_native_identity_and_session_state(self):
        for relative in (
            ".devcontainer/docker-compose-airgap-preparation.yml",
            "scripts/airgap-files/docker-compose.yml",
        ):
            compose = (ROOT / relative).read_text()
            for name in ("trustee1", "trustee2"):
                with self.subTest(deployment=relative, trustee=name):
                    trustee = service_block(compose, name)
                    self.assertIn("TRUSTEE_CONFIG_PATH:", trustee)
                    self.assertRegex(
                        trustee, r"(?m)^\s+- .+:/opt/trustee(?:/message_store)?$"
                    )

    def test_callback_limits_are_configurable_in_every_deployment(self):
        for relative in (
            ".devcontainer/docker-compose-base.yml",
            ".devcontainer/docker-compose-remote.yml",
            ".devcontainer/docker-compose-airgap-preparation.yml",
            "scripts/airgap-files/docker-compose.yml",
        ):
            compose = (ROOT / relative).read_text()
            for service, variable, default in (
                ("harvest", "MESSAGING_CALLBACK_CONNECTIONS", 4),
                ("messaging-webhooks", "MESSAGING_WEBHOOKS_MAX_IN_FLIGHT", 16),
            ):
                with self.subTest(deployment=relative, service=service):
                    self.assertIn(
                        f"{variable}: ${{{variable}:-{default}}}",
                        service_block(compose, service),
                    )

    def test_board_forwards_target_each_deployments_harvest_port(self):
        # nginx files take no environment, so their Harvest port is hardcoded.
        airgap = ".devcontainer/admin-portal-nginx/b4.conf"
        for forward, env in (
            (airgap, "scripts/airgap-files/.env"),
            (airgap, ".devcontainer/.env.development"),
            (
                ".devcontainer/nginx/default.conf.template",
                ".devcontainer/.env.remote-deployment.example",
            ),
        ):
            with self.subTest(forward=forward, env=env):
                port = re.search(
                    r'^HARVEST_PORT="?(\d+)"?$', (ROOT / env).read_text(), re.M
                )
                upstream = re.search(
                    r"set \$harvest_upstream harvest:(\d+);",
                    (ROOT / forward).read_text(),
                )
                self.assertTrue(port and upstream)
                self.assertEqual(upstream.group(1), port.group(1))

    def test_harvest_is_not_published_on_the_host(self):
        harvest = service_block(self.compose, "harvest")
        self.assertNotIn("ports:", harvest)
        self.assertIn(
            "${MESSAGING_RELAY_SECRET:?",
            service_block(self.compose, "messaging-webhooks"),
        )

    @unittest.skipUnless(shutil.which("docker"), "docker is not installed")
    def test_the_compose_file_renders_with_the_bundle_env(self):
        result = subprocess.run(
            [
                "docker",
                "compose",
                "--project-directory",
                str(BUNDLE),
                "--env-file",
                str(BUNDLE / ".env"),
                "-f",
                str(BUNDLE / "docker-compose.yml"),
                "--profile",
                "full",
                "config",
                "--quiet",
            ],
            capture_output=True,
            text=True,
            # Only the bundle's own values.
            env={"PATH": os.environ.get("PATH", "")},
        )
        if "Cannot connect" in result.stderr or "permission denied" in result.stderr:
            self.skipTest(result.stderr.strip())
        self.assertEqual(result.returncode, 0, result.stderr)

    def run_preamble(self, env_text):
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, ".env").write_text(env_text)
            result = subprocess.run(
                ["bash", "-c", secret_preamble()],
                cwd=directory,
                capture_output=True,
                text=True,
            )
            return result, Path(directory, ".env").read_text()

    def test_up_generates_the_relay_secret_once_and_refuses_a_blank_one(self):
        self.assertIn(f'MESSAGING_RELAY_SECRET="{PLACEHOLDER}"', self.env)
        # Biometric services are decided in the same preamble; not starting
        # them keeps this check about the generated secrets alone.
        env = self.env.replace(
            'BIOMETRIC_SERVICES="start"', 'BIOMETRIC_SERVICES="stopped"'
        )
        result, generated = self.run_preamble(env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        secret = re.search(
            r'^MESSAGING_RELAY_SECRET="([0-9a-f]{64})"$', generated, re.M
        )
        self.assertIsNotNone(secret, generated)
        # Other lines are left as they were, apart from the other generated
        # secrets (test_airgap_offline_services.py covers those).
        generated_keys = (
            "MONITORING_RENDERER_TOKEN",
            "SCANOVATE_LIVENESS_JWT_SECRET_KEY",
            "SCANOVATE_OCR_JWT_SECRET_KEY",
            "SCANOVATE_LIVENESS_CALLBACK_SECRET",
        )
        restored = generated.replace(
            secret.group(0), f'MESSAGING_RELAY_SECRET="{PLACEHOLDER}"'
        )
        for key in generated_keys:
            restored = re.sub(
                rf'^{key}="[0-9a-f]{{64}}"$', f'{key}="{PLACEHOLDER}"', restored, flags=re.M
            )
        self.assertEqual(restored, env)
        result, again = self.run_preamble(generated)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(again, generated)

        blank = env.replace(
            f'MESSAGING_RELAY_SECRET="{PLACEHOLDER}"', 'MESSAGING_RELAY_SECRET=""'
        )
        result, _ = self.run_preamble(blank)
        self.assertEqual(result.returncode, 1)
        self.assertIn("MESSAGING_RELAY_SECRET", result.stdout)


if __name__ == "__main__":
    unittest.main()
