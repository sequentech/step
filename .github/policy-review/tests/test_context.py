# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Pull request context gathering: issue references and diff budgeting."""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from policy_review.context import (
    changed_paths,
    collect_diff,
    find_issue_reference,
    merge_base,
)


class IssueReferenceTests(unittest.TestCase):
    def test_finds_a_related_link(self):
        body = "Does the thing.\n\nRelated: https://github.com/example-org/tracker/issues/12676"
        self.assertEqual(find_issue_reference(body), ("example-org", "tracker", 12676))

    def test_finds_a_parent_issue_link(self):
        body = "Parent issue: https://github.com/example-org/tracker/issues/42"
        self.assertEqual(find_issue_reference(body), ("example-org", "tracker", 42))

    def test_finds_a_bare_url_anywhere(self):
        body = "see https://github.com/example-org/tracker/issues/7 for background"
        self.assertEqual(find_issue_reference(body), ("example-org", "tracker", 7))

    def test_takes_the_first_of_several(self):
        body = (
            "https://github.com/example-org/tracker/issues/1 and "
            "https://github.com/example-org/tracker/issues/2"
        )
        self.assertEqual(find_issue_reference(body)[2], 1)

    def test_ignores_a_pull_request_link(self):
        body = "Follows https://github.com/example-org/tracker/pull/9"
        self.assertIsNone(find_issue_reference(body))

    def test_handles_an_empty_body(self):
        self.assertIsNone(find_issue_reference(""))
        self.assertIsNone(find_issue_reference(None))


class DiffTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

        self.git("init", "--quiet", "--initial-branch=main")
        self.git("config", "user.email", "test@example.com")
        self.git("config", "user.name", "Test")
        self.git("config", "commit.gpgsign", "false")

        (self.repo / "base.txt").write_text("base\n", encoding="utf-8")
        self.git("add", "-A")
        self.git("commit", "--quiet", "-m", "base")
        self.base = self.rev("HEAD")

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", *args],
            cwd=str(self.repo),
            check=True,
            capture_output=True,
            text=True,
        ).stdout

    def rev(self, ref: str) -> str:
        return self.git("rev-parse", ref).strip()

    def add_commit(self, name: str, content: str) -> str:
        (self.repo / name).write_text(content, encoding="utf-8")
        self.git("add", "-A")
        self.git("commit", "--quiet", "-m", f"add {name}")
        return self.rev("HEAD")

    def test_lists_changed_paths(self):
        head = self.add_commit("added.txt", "new file\n")
        self.assertEqual(changed_paths(self.base, head, self.repo), ["added.txt"])

    def test_resolves_the_merge_base(self):
        head = self.add_commit("added.txt", "new\n")
        self.assertEqual(merge_base("main", head, self.repo), head)

    def test_returns_the_full_diff_when_it_fits(self):
        head = self.add_commit("added.txt", "new file\n")
        diff, truncated = collect_diff(self.base, head, self.repo, 100_000)
        self.assertFalse(truncated)
        self.assertIn("added.txt", diff)
        self.assertIn("+new file", diff)

    def test_truncates_an_oversized_diff_and_says_so(self):
        for index in range(6):
            self.add_commit(f"file{index}.txt", ("x" * 400 + "\n") * 12)
        head = self.rev("HEAD")

        diff, truncated = collect_diff(self.base, head, self.repo, 2_000)

        self.assertTrue(truncated)
        self.assertIn("diff truncated", diff)
        # The budget bounds the retained hunks; the notice itself may exceed it
        # slightly, which is what makes the truncation visible to the reviewer.
        self.assertLess(len(diff.encode("utf-8")), 4_000)

    def test_keeps_a_prefix_when_one_file_exceeds_the_whole_budget(self):
        head = self.add_commit("huge.txt", ("y" * 200 + "\n") * 60)
        diff, truncated = collect_diff(self.base, head, self.repo, 500)
        self.assertTrue(truncated)
        self.assertTrue(diff.strip())


if __name__ == "__main__":
    unittest.main()
