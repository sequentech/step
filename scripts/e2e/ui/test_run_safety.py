#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise the UI cleanup handshake against the backend's fake Docker harness."""
from pathlib import Path
import shutil
import subprocess
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import test_run_safety as backend_safety  # noqa: E402


class UIRunSafety(backend_safety.RunSafety):
    def setUp(self):
        super().setUp()
        script = self.root / "scripts/e2e/ui/run.sh"
        script.parent.mkdir()
        shutil.copyfile(backend_safety.ROOT / "scripts/e2e/ui/run.sh", script)
        script.chmod(0o755)

    def run_ui(self, *args, **environment):
        return subprocess.run(
            [str(self.root / "scripts/e2e/ui/run.sh"), *args],
            env={**self.env, **environment}, capture_output=True, text=True, timeout=10,
        )

    def test_ui_down_requires_explicit_project(self):
        result = self.run_ui("--down")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("STEP_E2E_PROJECT", result.stderr)
        self.assertEqual(self.calls(), [])

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
                    "--skip-ui-build", "--skip-images", "--skip-build",
                    STEP_E2E_PROJECT="other-persons-stack", STEP_E2E_OUTPUT_DIR=str(output),
                    STEP_E2E_RUN_TOKEN="old-run", FAKE_DOCKER_MODE="collision-" + kind,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("already", result.stderr)
                self.assertEqual(self.compose_calls("up"), [])
                self.assertEqual(self.compose_calls("down"), [])

    def test_ui_partial_backend_start_is_owned_and_cleaned_up(self):
        result = self.run_ui(
            "--skip-ui-build", "--skip-images", "--skip-build",
            STEP_E2E_PROJECT="partial-ui", FAKE_DOCKER_MODE="partial-start",
        )
        self.assertEqual(result.returncode, 19, result.stderr)
        self.assertEqual(len(self.compose_calls("up")), 1)
        self.assertEqual(len(self.compose_calls("down")), 1)
        self.assertFalse(list((self.root / ".cache/backend-e2e/partial-ui").glob(".owned-*")))

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
            self.assertTrue((self.root / ".cache/backend-e2e" / project / "compose.env").is_file())


if __name__ == "__main__":
    unittest.main()
