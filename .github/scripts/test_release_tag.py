# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Regression checks for the explicit-version workflow's Git operations."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import yaml

WORKFLOW = Path(__file__).resolve().parents[1] / "workflows/release.yml"
STEPS = {step["name"]: step for step in yaml.safe_load(WORKFLOW.read_text())["jobs"]["release"]["steps"]}


class ReleaseTagTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1",
                        REQUESTED_VERSION="v11.0.0-rc.3", GITHUB_REF_TYPE="branch")
        self.git("init", "--bare", "remote.git")
        self.git("init", "checkout")
        self.root /= "checkout"
        self.git("config", "user.name", "Release test")
        self.git("config", "user.email", "release@example.invalid")
        self.git("commit", "--allow-empty", "-m", "fixture")
        self.git("remote", "add", "origin", "../remote.git")
        self.env["GITHUB_SHA"] = self.git("rev-parse", "HEAD").stdout.strip()

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, env=self.env,
                              text=True, capture_output=True, check=True)

    def shell(self, script):
        return subprocess.run(["bash", "-e", "-c", script], cwd=self.root,
                              env=self.env, text=True, capture_output=True)

    def test_first_explicit_version_needs_no_previous_tag(self):
        result = self.shell(STEPS["Check the requested version"]["run"])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(STEPS["Determine Release Increment"]["if"], "github.event.inputs.version == ''")

    def test_dispatch_commit_must_match_checkout(self):
        self.env["GITHUB_SHA"] = "0" * 40
        self.assertNotEqual(self.shell(STEPS["Check the requested version"]["run"]).returncode, 0)

    def test_explicit_release_requires_correct_published_tag(self):
        script = STEPS["Release"]["run"].split('if [ -n "$requestedVersion" ]; then')[1].split('# If the new release branch')[0]
        tag = "v11.0.0-rc.3"
        # Stub only release-it; exercise the workflow's actual Git verification.
        stub = self.root / "release-it"
        stub.write_text("#!/bin/sh\nexit 0\n")
        stub.chmod(0o755)
        self.env["PATH"] = str(self.root) + os.pathsep + self.env["PATH"]
        script = 'requestedVersion=11.0.0-rc.3\n' + script
        self.assertNotEqual(self.shell(script).returncode, 0)  # No tag.
        self.git("tag", "-a", tag, "-m", "release")
        self.assertNotEqual(self.shell(script).returncode, 0)  # Local only.
        self.git("push", "origin", "refs/tags/" + tag)
        self.assertEqual(self.shell(script).returncode, 0)
        self.git("commit", "--allow-empty", "-m", "different source")
        self.assertNotEqual(self.shell(script).returncode, 0)
        self.git("tag", "-d", tag)
        self.git("tag", "-a", tag, "-m", "local replacement")
        self.assertNotEqual(self.shell(script).returncode, 0)  # Remote still differs.
        self.assertEqual(self.git("ls-remote", "origin", "refs/tags/" + tag + "^{}").stdout.split()[0], self.env["GITHUB_SHA"])


if __name__ == "__main__":
    unittest.main()
