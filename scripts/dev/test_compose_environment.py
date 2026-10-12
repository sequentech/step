# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Compose configuration comes from files and defaults, without shell exports."""

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
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
        for service in ("immudb-init", "driver", "ui-browser"):
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
