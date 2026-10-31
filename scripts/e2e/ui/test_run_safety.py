# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise the UI cleanup handshake against the backend's fake Docker harness."""

import json
import shutil
import subprocess
import sys
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import test_run_safety as backend_safety


class UIRunSafety(backend_safety.RunSafety):
    def setUp(self):
        super().setUp()
        script = self.root / "scripts/e2e/ui/run.sh"
        script.parent.mkdir()
        shutil.copyfile(backend_safety.ROOT / "scripts/e2e/ui/run.sh", script)
        script.chmod(0o755)
        self.docker.write_text(
            self.docker.read_text().replace(
                'if "up" in args and mode == "held-start":',
                'if ("up" in args and mode == "held-start") or ("exec" in args and mode == "held-ui"):',
            )
        )

    def run_ui(self, *args, **environment):
        return subprocess.run(
            [str(self.root / "scripts/e2e/ui/run.sh"), *args],
            env={**self.env, **environment},
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )

    def test_ui_retains_project_claim_through_browser_execution(self):
        project = "concurrent-ui-" + self.root.name.lower()
        started, release = self.root / "started", self.root / "release"
        process = subprocess.Popen(
            [
                str(self.root / "scripts/e2e/ui/run.sh"),
                "--skip-ui-build",
                "--skip-images",
                "--skip-build",
            ],
            env={
                **self.env,
                "STEP_E2E_PROJECT": project,
                "FAKE_DOCKER_MODE": "held-ui",
                "FAKE_START_MARKER": str(started),
                "FAKE_RELEASE_MARKER": str(release),
            },
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        try:
            deadline = time.monotonic() + 5
            while not started.exists() and time.monotonic() < deadline:
                time.sleep(0.01)
            self.assertTrue(
                started.exists(), "First UI launcher never reached browser execution"
            )
            for run, arguments in (
                (self.run_ui, ("--skip-ui-build", "--skip-images", "--skip-build")),
                (self.run_ui, ("--down",)),
                (self.run_script, ("--skip-images", "--skip-build")),
                (self.run_script, ("--down",)),
            ):
                with self.subTest(launcher=run.__name__, arguments=arguments):
                    prior = self.calls()
                    result = run(*arguments, STEP_E2E_PROJECT=project)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("already claimed", result.stderr)
                    self.assertEqual(self.calls(), prior)
        finally:
            release.touch()
            stdout, stderr = process.communicate(timeout=10)
        self.assertEqual(process.returncode, 0, stdout + stderr)
        self.assertEqual(len(self.compose_calls("up")), 2)
        self.assertEqual(len(self.compose_calls("down")), 1)
        result = self.run_ui(
            "--skip-ui-build", "--skip-images", "--skip-build", STEP_E2E_PROJECT=project
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_ui_bash_invocation_preserves_relative_artifact_paths(self):
        working = self.root / "scripts/e2e/ui"
        result = subprocess.run(
            ["bash", "run.sh", "--skip-ui-build", "--skip-images", "--skip-build"],
            cwd=working,
            env={
                **self.env,
                "STEP_E2E_PROJECT": "relative-" + self.root.name.lower(),
                "STEP_E2E_BIN_DIR": "relative binaries",
                "STEP_E2E_OUTPUT_DIR": "relative logs",
            },
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        environment = (working / "relative logs/compose.env").read_text()
        self.assertIn(f"STEP_E2E_BIN_DIR={working}/relative binaries\n", environment)
        self.assertIn(f"STEP_E2E_OUTPUT_DIR={working}/relative logs\n", environment)
        for call in self.compose_calls("up"):
            env_files = [
                call[i + 1] for i, value in enumerate(call) if value == "--env-file"
            ]
            self.assertEqual(env_files[-1], str(working / "relative logs/compose.env"))
        self.assertEqual(len(self.compose_calls("down")), 1)
        self.assertFalse(list((working / "relative logs").glob(".owned-*")))

    def test_ui_down_requires_explicit_project(self):
        result = self.run_ui("--down")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("STEP_E2E_PROJECT", result.stderr)
        self.assertEqual(self.calls(), [])

    def test_ui_down_normalizes_retained_relative_output(self):
        caller = self.root / "caller"
        output = caller / "retained logs"
        output.mkdir(parents=True)
        environment = output / "compose.env"
        environment.write_text(f"STEP_E2E_OUTPUT_DIR={output}\n")
        marker = output / ".owned-retained"
        marker.write_text("retained-ui\n")
        result = subprocess.run(
            [str(self.root / "scripts/e2e/ui/run.sh"), "--down"],
            cwd=caller,
            env={
                **self.env,
                "STEP_E2E_PROJECT": "retained-ui",
                "STEP_E2E_OUTPUT_DIR": "retained logs",
                "STEP_E2E_BIN_DIR": "uncreated binaries",
            },
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        (down,) = self.calls()
        self.assertEqual(
            down[-5:], ["down", "--volumes", "--remove-orphans", "--timeout", "20"]
        )
        env_files = [
            down[i + 1] for i, value in enumerate(down) if value == "--env-file"
        ]
        self.assertEqual(env_files[-1], str(environment))
        exported = json.loads(self.environment_log.read_text())
        self.assertEqual(exported["STEP_E2E_OUTPUT_DIR"], str(output))
        self.assertEqual(environment.read_text(), f"STEP_E2E_OUTPUT_DIR={output}\n")
        self.assertEqual(marker.read_text(), "retained-ui\n")
        self.assertFalse((output / "logs").exists())
        self.assertFalse((caller / "uncreated binaries").exists())

    def test_ui_collision_and_stale_marker_never_remove_existing_project(self):
        control = self.run_ui("--skip-ui-build", "--skip-images", "--skip-build")
        self.assertEqual(control.returncode, 0, control.stderr)
        output = self.root / "prior-output"
        output.mkdir()
        (output / ".owned-old-run").write_text("other-persons-stack\n")
        for kind in ("container", "network", "volume"):
            with self.subTest(kind=kind):
                self.log.unlink()
                result = self.run_ui(
                    "--skip-ui-build",
                    "--skip-images",
                    "--skip-build",
                    STEP_E2E_PROJECT="other-persons-stack",
                    STEP_E2E_OUTPUT_DIR=str(output),
                    STEP_E2E_RUN_TOKEN="old-run",
                    FAKE_DOCKER_MODE="collision-" + kind,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("already", result.stderr)
                self.assertEqual(self.compose_calls("up"), [])
                self.assertEqual(self.compose_calls("down"), [])

    def test_ui_partial_backend_start_is_owned_and_cleaned_up(self):
        result = self.run_ui(
            "--skip-ui-build",
            "--skip-images",
            "--skip-build",
            STEP_E2E_PROJECT="partial-ui",
            FAKE_DOCKER_MODE="partial-start",
        )
        self.assertEqual(result.returncode, 19, result.stderr)
        self.assertEqual(len(self.compose_calls("up")), 1)
        self.assertEqual(len(self.compose_calls("down")), 1)
        self.assertFalse(
            list((self.root / ".cache/backend-e2e/partial-ui").glob(".owned-*"))
        )

    def test_ui_default_runs_have_distinct_projects_and_logs(self):
        for _ in range(2):
            result = self.run_ui("--skip-ui-build", "--skip-images", "--skip-build")
            self.assertEqual(result.returncode, 0, result.stderr)
        starts, stops = self.compose_calls("up"), self.compose_calls("down")
        self.assertEqual(len(starts), 4)
        self.assertEqual(len(stops), 2)
        projects = [args[args.index("--project-name") + 1] for args in stops]
        self.assertEqual(len(set(projects)), 2)
        for project in projects:
            self.assertTrue(project.startswith("step-e2e-ui-"))
            self.assertTrue(
                (self.root / ".cache/backend-e2e" / project / "compose.env").is_file()
            )


if __name__ == "__main__":
    unittest.main()
