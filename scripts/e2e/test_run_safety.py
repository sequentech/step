# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise launcher ownership with a fake Docker executable; no daemon needed."""

import json
import os
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FAKE_DOCKER = r"""#!/usr/bin/env python3
import json, os, sys, time
from pathlib import Path
args = sys.argv[1:]
with open(os.environ["FAKE_DOCKER_LOG"], "a") as output:
    output.write(json.dumps(args) + "\n")
mode = os.environ.get("FAKE_DOCKER_MODE", "success")
if args[0] != "compose":
    if mode == "daemon-failure":
        sys.exit(125)
    kind = "container" if args[0] == "ps" else args[0]
    if mode == "collision-" + kind:
        print("owned-by-another-run")
    sys.exit(0)
if "up" in args and mode == "held-start":
    Path(os.environ["FAKE_START_MARKER"]).touch()
    while not Path(os.environ["FAKE_RELEASE_MARKER"]).exists():
        time.sleep(0.01)
if "up" in args and mode == "partial-start":
    sys.exit(19)
"""


class RunSafety(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="step-e2e-run-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        script = self.root / "scripts/e2e/run.sh"
        script.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "scripts/e2e/run.sh", script)
        script.chmod(0o755)
        shutil.copyfile(ROOT / "scripts/e2e/build.sh", script.parent / "build.sh")
        shutil.copyfile(
            ROOT / "scripts/e2e/project_lock.py", script.parent / "project_lock.py"
        )
        devcontainer = self.root / ".devcontainer"
        devcontainer.mkdir()
        (devcontainer / ".env.development").write_text("# synthetic test environment\n")
        self.docker = self.root / "docker"
        self.docker.write_text(FAKE_DOCKER)
        self.docker.chmod(0o755)
        self.log = self.root / "docker.jsonl"
        self.env = dict(os.environ)
        for name in (
            "STEP_E2E_PROJECT", "STEP_E2E_OUTPUT_DIR", "STEP_E2E_RUN_TOKEN",
            "STEP_E2E_BIN_DIR", "STEP_E2E_CARGO_TARGET", "STEP_E2E_CARGO_HOME",
        ):
            self.env.pop(name, None)
        self.env.update(DOCKER=str(self.docker), FAKE_DOCKER_LOG=str(self.log))

    def run_script(self, *args, **environment):
        return subprocess.run(
            [str(self.root / "scripts/e2e/run.sh"), *args],
            env={**self.env, **environment},
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )

    def calls(self):
        return (
            [json.loads(line) for line in self.log.read_text().splitlines()]
            if self.log.exists()
            else []
        )

    def compose_calls(self, verb):
        return [args for args in self.calls() if args[0] == "compose" and verb in args]

    def test_bash_invocation_from_the_script_directory_retains_arguments(self):
        script_directory = self.root / "scripts/e2e"
        result = subprocess.run(
            ["bash", "run.sh", "--keep", "--skip-images", "--skip-build", "-k", "cast vote"],
            cwd=script_directory,
            env=self.env,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(self.compose_calls("up")), 1)
        self.assertEqual(self.compose_calls("down"), [])
        driver = [call for call in self.compose_calls("run") if "test" in call]
        self.assertEqual(len(driver), 1)
        self.assertEqual(driver[0][-3:], ["test", "-k", "cast vote"])

    def test_relative_run_directories_are_resolved_against_the_callers_directory(self):
        caller = self.root / "caller"
        caller.mkdir()
        result = subprocess.run(
            [str(self.root / "scripts/e2e/run.sh"), "--skip-images", "--skip-build"],
            cwd=caller,
            env={
                **self.env,
                "STEP_E2E_BIN_DIR": "binaries here",
                "STEP_E2E_OUTPUT_DIR": "logs/run",
            },
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        output = caller / "logs/run"
        self.assertTrue((caller / "binaries here").is_dir())
        values = dict(
            line.split("=", 1) for line in (output / "compose.env").read_text().splitlines()
        )
        self.assertEqual(values["STEP_E2E_BIN_DIR"], str(caller / "binaries here"))
        self.assertEqual(values["STEP_E2E_OUTPUT_DIR"], str(output))
        for call in self.compose_calls("up") + self.compose_calls("down"):
            env_files = [
                call[index + 1] for index, value in enumerate(call) if value == "--env-file"
            ]
            self.assertEqual(env_files[-1], str(output / "compose.env"))
        self.assertFalse(list(output.glob(".owned-*")))

    def test_relative_build_output_is_a_host_bind_mount(self):
        caller = self.root / "caller"
        caller.mkdir()
        for configured in ("bin", ".cache/binaries here"):
            with self.subTest(configured=configured):
                result = subprocess.run(
                    ["bash", str(self.root / "scripts/e2e/build.sh")],
                    cwd=caller,
                    env={**self.env, "STEP_E2E_BIN_DIR": configured},
                    capture_output=True,
                    text=True,
                    timeout=10,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                build = self.calls()[-1]
                mounts = [
                    build[index + 1]
                    for index, value in enumerate(build)
                    if value == "--volume"
                ]
                self.assertIn(f"{caller / configured}:/out", mounts)
                self.assertTrue((caller / configured).is_dir())

    def test_down_requires_an_explicit_project(self):
        result = self.run_script("--down")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("STEP_E2E_PROJECT", result.stderr)
        self.assertEqual(self.calls(), [])

    def test_default_runs_have_distinct_projects_logs_and_cleanup_only_after_start(
        self,
    ):
        for _ in range(2):
            result = self.run_script("--skip-images", "--skip-build")
            self.assertEqual(result.returncode, 0, result.stderr)
        starts, stops = self.compose_calls("up"), self.compose_calls("down")
        self.assertEqual(len(starts), 2)
        self.assertEqual(len(stops), 2)
        projects = [args[args.index("--project-name") + 1] for args in starts]
        self.assertEqual(len(set(projects)), 2)
        self.assertEqual(
            projects, [args[args.index("--project-name") + 1] for args in stops]
        )
        all_calls = self.calls()
        for start, stop, project in zip(starts, stops, projects):
            self.assertLess(all_calls.index(start), all_calls.index(stop))
            self.assertTrue(
                (self.root / ".cache/backend-e2e" / project / "compose.env").is_file()
            )

    def test_existing_project_resources_are_never_started_or_removed(self):
        # First prove the fake daemon can complete an ordinary owned run.
        control = self.run_script("--skip-images", "--skip-build")
        self.assertEqual(control.returncode, 0, control.stderr)
        for kind in ("container", "network", "volume"):
            with self.subTest(kind=kind):
                self.log.unlink()
                result = self.run_script(
                    "--skip-images",
                    "--skip-build",
                    STEP_E2E_PROJECT="other-persons-stack",
                    FAKE_DOCKER_MODE="collision-" + kind,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("already", result.stderr)
                self.assertEqual(self.compose_calls("up"), [])
                self.assertEqual(self.compose_calls("down"), [])

    def test_daemon_inspection_failure_does_not_install_cleanup(self):
        result = self.run_script(
            "--skip-images", "--skip-build", FAKE_DOCKER_MODE="daemon-failure"
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.compose_calls("up"), [])
        self.assertEqual(self.compose_calls("down"), [])

    def test_partial_start_is_owned_and_cleaned_up(self):
        result = self.run_script(
            "--skip-images",
            "--skip-build",
            STEP_E2E_PROJECT="partial-stack",
            FAKE_DOCKER_MODE="partial-start",
        )
        self.assertEqual(result.returncode, 19, result.stderr)
        self.assertEqual(len(self.compose_calls("up")), 1)
        self.assertEqual(len(self.compose_calls("down")), 1)
        self.assertFalse(
            list((self.root / ".cache/backend-e2e/partial-stack").glob(".owned-*"))
        )

    def test_concurrent_project_claim_refuses_start_and_down_across_checkouts(self):
        project = "concurrent-" + self.root.name.lower()
        started = self.root / "started"
        release = self.root / "release"
        process = subprocess.Popen(
            [str(self.root / "scripts/e2e/run.sh"), "--skip-images", "--skip-build"],
            env={
                **self.env,
                "STEP_E2E_PROJECT": project,
                "FAKE_DOCKER_MODE": "held-start",
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
            self.assertTrue(started.exists(), "First launcher never reached compose up")
            # A different checkout/output directory must contend on the same lock.
            second_root = self.root / "second-checkout"
            shutil.copytree(self.root / "scripts", second_root / "scripts")
            shutil.copytree(self.root / ".devcontainer", second_root / ".devcontainer")
            for arguments in (("--skip-images", "--skip-build"), ("--down",)):
                with self.subTest(arguments=arguments):
                    prior = self.calls()
                    result = subprocess.run(
                        [str(second_root / "scripts/e2e/run.sh"), *arguments],
                        env={**self.env, "STEP_E2E_PROJECT": project},
                        capture_output=True,
                        text=True,
                        timeout=5,
                        check=False,
                    )
                    self.assertNotEqual(result.returncode, 0, result.stderr)
                    self.assertIn("already claimed", result.stderr)
                    self.assertEqual(self.calls(), prior)
        finally:
            release.touch()
            stdout, stderr = process.communicate(timeout=10)
        self.assertEqual(process.returncode, 0, stdout + stderr)
        self.assertEqual(len(self.compose_calls("up")), 1)
        self.assertEqual(len(self.compose_calls("down")), 1)
        # Completion releases the lock, including after ordinary owned cleanup.
        result = self.run_script(
            "--skip-images", "--skip-build", STEP_E2E_PROJECT=project
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_keep_can_be_removed_by_an_explicit_down(self):
        result = self.run_script(
            "--keep", "--skip-images", "--skip-build", STEP_E2E_PROJECT="kept-stack"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.compose_calls("down"), [])
        result = self.run_script("--down", STEP_E2E_PROJECT="kept-stack")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(self.compose_calls("down")), 1)


if __name__ == "__main__":
    unittest.main()
