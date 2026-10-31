# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise explicit releases against disposable local Git remotes."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("release_explicit_version.py").resolve()
CONFIG = Path(__file__).resolve().parents[2] / ".release-it.json"


class ExplicitReleaseTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "checkout"
        self.remote = self.root / "remote.git"
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull,
                        GIT_CONFIG_NOSYSTEM="1", NO_UPDATE_NOTIFIER="1")
        self.call("git", "init", "--bare", str(self.remote), cwd=self.root)
        self.call("git", "init", "-b", "ovcs", str(self.repo), cwd=self.root)
        self.git("config", "user.name", "Release test")
        self.git("config", "user.email", "release@example.invalid")
        config = json.loads(CONFIG.read_text())
        config["github"]["release"] = False  # Only the external GitHub boundary.
        (self.repo / ".release-it.json").write_text(json.dumps(config))
        self.git("add", ".release-it.json")
        self.git("commit", "-m", "Initial fixture")
        self.git("tag", "v10.0.0")
        self.git("remote", "add", "origin", str(self.remote))
        self.git("push", "-u", "origin", "ovcs", "--tags")
        self.sha = self.git("rev-parse", "HEAD").stdout.strip()
        self.env.update(REQUESTED_VERSION="v11.0.0-rc.3",
                        RELEASE_REF="refs/heads/ovcs", RELEASE_SHA=self.sha)

    def call(self, *args, cwd=None, check=True, env=None):
        return subprocess.run(args, cwd=cwd or self.repo,
                              env=env or self.env, text=True,
                              capture_output=True, check=check, timeout=30)

    def git(self, *args):
        return self.call("git", *args)

    def release(self, *args, **changes):
        return self.call(sys.executable, str(SCRIPT), *args, check=False,
                         env=dict(self.env, **changes))

    def remote_target(self):
        rows = self.git("ls-remote", "origin", "refs/tags/v11.0.0-rc.3*").stdout
        refs = dict(line.split()[::-1] for line in rows.splitlines())
        return refs.get("refs/tags/v11.0.0-rc.3^{}", refs.get("refs/tags/v11.0.0-rc.3"))

    def test_creates_exact_annotated_remote_tag(self):
        result = self.release()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.remote_target(), self.sha)
        self.assertEqual(self.git("cat-file", "-t", "v11.0.0-rc.3").stdout.strip(), "tag")

    def test_accepts_version_without_v(self):
        result = self.release(REQUESTED_VERSION="11.0.0-rc.3")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.remote_target(), self.sha)

    def test_same_commit_retry_keeps_tag_object(self):
        self.git("tag", "-a", "v11.0.0-rc.3", "-m", "Existing release")
        self.git("push", "origin", "refs/tags/v11.0.0-rc.3")
        before = self.git("rev-parse", "v11.0.0-rc.3").stdout
        result = self.release()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.remote_target(), self.sha)
        self.assertEqual(self.git("rev-parse", "v11.0.0-rc.3").stdout, before)

    def test_conflicting_remote_tag_is_not_moved(self):
        self.git("tag", "v11.0.0-rc.3")
        self.git("push", "origin", "refs/tags/v11.0.0-rc.3")
        self.git("commit", "--allow-empty", "-m", "Another commit")
        result = self.release(RELEASE_SHA=self.git("rev-parse", "HEAD").stdout.strip())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("different commit", result.stderr)
        self.assertEqual(self.remote_target(), self.sha)

    def test_lightweight_same_commit_tag_can_be_reused(self):
        self.git("tag", "v11.0.0-rc.3")
        self.git("push", "origin", "refs/tags/v11.0.0-rc.3")
        result = self.release()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.remote_target(), self.sha)

    def test_missing_remote_is_an_error_not_a_missing_tag(self):
        self.git("remote", "set-url", "origin", str(self.root / "absent.git"))
        result = self.release()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("ls-remote", result.stderr)
        self.assertEqual(self.git("tag", "--list", "v11*").stdout, "")

    def test_matching_release_branch_accepts_its_version(self):
        self.git("checkout", "-b", "release/10.0")
        self.git("push", "-u", "origin", "release/10.0")
        result = self.release(REQUESTED_VERSION="v10.0.1-rc.3",
                              RELEASE_REF="refs/heads/release/10.0")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.git("rev-parse", "v10.0.1-rc.3^{commit}").stdout.strip(), self.sha)

    def test_wrong_checkout_fails_before_tagging(self):
        result = self.release(RELEASE_SHA="0" * 40)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("dispatch commit", result.stderr)
        self.assertIsNone(self.remote_target())

    def test_wrong_branch_fails_before_tagging(self):
        result = self.release(RELEASE_REF="refs/heads/main")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("dispatch branch", result.stderr)
        self.assertIsNone(self.remote_target())

    def test_version_branch_and_ref_validation(self):
        for changes in [{"REQUESTED_VERSION": "11.0.0;false"},
                        {"REQUESTED_VERSION": "v11.00.0-rc.3"},
                        {"RELEASE_REF": "refs/tags/v11.0.0-rc.3"}]:
            with self.subTest(changes=changes):
                result = self.release(**changes)
                self.assertNotEqual(result.returncode, 0)
                self.assertIsNone(self.remote_target())
        self.git("checkout", "-b", "release/10.0")
        result = self.release(RELEASE_REF="refs/heads/release/10.0")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not belong", result.stderr)

    def test_older_version_is_rejected(self):
        result = self.release(REQUESTED_VERSION="v9.0.0")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not newer", result.stderr)

    def test_check_only_does_not_publish(self):
        result = self.release("--check")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(self.remote_target())

    def test_successful_noop_cannot_claim_a_published_tag(self):
        bindir = self.root / "bin"
        bindir.mkdir()
        fake = bindir / "release-it"
        fake.write_text("#!/bin/sh\nexit 0\n")
        fake.chmod(0o755)
        result = self.release(PATH=str(bindir) + os.pathsep + self.env["PATH"])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not published", result.stderr)
        self.assertIsNone(self.remote_target())


if __name__ == "__main__":
    unittest.main()
