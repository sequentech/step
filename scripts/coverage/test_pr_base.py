# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from ratchet import compare

SCRIPT = Path(__file__).with_name("checkout-pr-base.sh").resolve()
ENV = {
    "PATH": os.defpath,
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_ALLOW_PROTOCOL": "file",
    "GIT_AUTHOR_NAME": "Fixture",
    "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
    "GIT_COMMITTER_NAME": "Fixture",
    "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
}


def git(root, *args):
    return subprocess.check_output(
        ["git", *args], cwd=root, env=ENV, text=True, stderr=subprocess.PIPE
    ).strip()


def commit(root, message):
    git(root, "add", ".")
    git(root, "commit", "-qm", message)
    return git(root, "rev-parse", "HEAD")


def measured_lines(checkout, output):
    # Count executed and missed statements in this tiny program with stdlib trace.
    subprocess.run(
        [
            sys.executable,
            "-m",
            "trace",
            "--count",
            "--missing",
            "--coverdir",
            str(output),
            str(checkout / "value.py"),
        ],
        env=ENV,
        check=True,
    )
    (report,) = output.glob("*value.cover")
    lines = report.read_text().splitlines()
    covered = sum(bool(re.match(r"^\s*\d+:", line)) for line in lines)
    missed = sum(line.startswith(">>>>>>") for line in lines)
    return {"lines": {"covered": covered, "count": covered + missed}}


class PullRequestBaseTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        origin = self.root / "repository"
        self.head = self.root / "head"
        self.base = self.root / "base"
        origin.mkdir()
        git(origin, "init", "-q", "--template=", "-b", "target")
        code = origin / "value.py"
        code.write_text("def value(flag):\n    return 1\n\nassert value(False) == 1\n")
        self.stale = commit(origin, "recorded base")
        git(origin, "checkout", "-qb", "topic")
        changed = "    if flag:\n        return 2\n    return 1"
        code.write_text(code.read_text().replace("    return 1", changed))
        self.topic = commit(origin, "add an uncovered path")
        git(origin, "checkout", "-q", "target")
        (origin / "upstream").write_text("already on the target\n")
        self.first = commit(origin, "actual target")
        git(origin, "merge", "-q", "--no-ff", "--no-commit", "topic")
        (origin / "integration").write_text("present only in the merge result\n")
        self.merge = commit(origin, "tested merge")
        git(self.root, "clone", "-q", "--depth=2", origin.as_uri(), "head")
        self.event = {
            "GITHUB_EVENT_NAME": "pull_request",
            "GITHUB_REF": "refs/pull/4361/merge",
            "GITHUB_SHA": self.merge,
            "PR_NUMBER": "4361",
            "PR_HEAD_SHA": self.topic,
            "PR_MERGE_SHA": self.merge,
        }

    def select(self, **event):
        return subprocess.run(
            ["bash", str(SCRIPT)],
            cwd=self.root,
            env=ENV | self.event | event,
            capture_output=True,
            text=True,
        )

    def reject(self, **event):
        self.assertNotEqual(self.select(**event).returncode, 0)
        self.assertFalse(self.base.exists())

    def test_pair_keeps_changed_code_and_its_decrease(self):
        result = self.select(PR_MERGE_SHA="")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(git(self.base, "rev-parse", "HEAD"), self.first)
        self.assertEqual(git(self.head, "rev-parse", "HEAD"), self.merge)
        for checkout in (self.base, self.head):
            self.assertEqual(git(checkout, "status", "--porcelain"), "")
        self.assertEqual(
            git(self.head, "diff", "--name-only", self.first, self.merge).splitlines(),
            ["integration", "value.py"],
        )
        stale_diff = git(
            self.root / "repository", "diff", "--name-only", self.stale, self.merge
        ).splitlines()
        self.assertIn("upstream", stale_diff)
        old = measured_lines(self.base, self.root / "base-report")
        new = measured_lines(self.head, self.root / "head-report")
        self.assertEqual(old["lines"], {"covered": 3, "count": 3})
        self.assertEqual(new["lines"], {"covered": 4, "count": 5})
        self.assertFalse(compare(old, new)["passes"])

    def test_rejects_wrong_event_ref_or_commit(self):
        for key, value in (
            ("GITHUB_EVENT_NAME", "pull_request_target"),
            ("GITHUB_REF", "refs/pull/4361/head"),
            ("GITHUB_REF", "refs/heads/target"),
            ("PR_NUMBER", "4362"),
            ("GITHUB_SHA", self.topic),
            ("PR_MERGE_SHA", self.stale),
            ("PR_HEAD_SHA", self.first),
            ("PR_HEAD_SHA", self.merge),
            ("PR_HEAD_SHA", self.topic[:7]),
            ("PR_HEAD_SHA", "HEAD^2"),
            ("PR_HEAD_SHA", ""),
        ):
            with self.subTest(key=key, value=value):
                self.reject(**{key: value})
        self.reject(GITHUB_SHA=self.topic, PR_MERGE_SHA=self.topic)

    def test_rejects_single_reversed_and_extra_parents(self):
        for parents in (
            (self.topic,),
            (self.topic, self.first),
            (self.first, self.topic, self.merge),
        ):
            with self.subTest(parents=parents):
                args = [arg for parent in parents for arg in ("-p", parent)]
                tree = f"{self.merge}^{{tree}}"
                other = git(self.head, "commit-tree", tree, *args, "-m", "other")
                git(self.head, "checkout", "-q", "--detach", other)
                self.reject(GITHUB_SHA=other, PR_MERGE_SHA=other)

    def test_shallow_boundary_cannot_supply_a_base(self):
        git(self.head, "fetch", "-q", "--depth=1", "origin", self.merge)
        self.reject()

    def test_existing_base_is_preserved_and_rejected(self):
        result = self.select()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotEqual(self.select().returncode, 0)
        self.assertEqual(git(self.base, "rev-parse", "HEAD"), self.first)
