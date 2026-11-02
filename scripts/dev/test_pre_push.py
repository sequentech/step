# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for scripts.dev.pre_push: pushed ranges and the lint checks they run."""

import subprocess
import unittest
from unittest import mock

from scripts.dev import pre_push
from scripts.dev.pre_push import ZERO


def check(
    identifier,
    kind="lint",
    selected=True,
    cwd="packages",
    command="cargo fmt",
    requires=(),
):
    return {
        "id": identifier,
        "kind": kind,
        "selected": selected,
        "cwd": cwd,
        "command": command,
        "requires": list(requires),
    }


class PushedRanges(unittest.TestCase):
    def test_updates_new_refs_and_deletions(self):
        lines = [
            f"refs/heads/a {'1' * 40} refs/heads/a {'2' * 40}\n",
            f"refs/heads/b {'3' * 40} refs/heads/b {ZERO}\n",
            f"(delete) {ZERO} refs/heads/c {'4' * 40}\n",
        ]
        self.assertEqual(
            pre_push.pushed_ranges(lines, lambda sha: True),
            [("2" * 40, "1" * 40), (None, "3" * 40)],
        )

    def test_a_remote_tip_this_clone_lacks_has_no_base(self):
        lines = [f"refs/heads/a {'1' * 40} refs/heads/a {'2' * 40}"]
        self.assertEqual(
            pre_push.pushed_ranges(lines, lambda sha: False), [(None, "1" * 40)]
        )


class ChangedFiles(unittest.TestCase):
    def git(self, *arguments):
        if arguments[0] == "merge-base":
            return "base\n"
        self.assertEqual(arguments[:3], ("diff", "--name-only", "--no-renames"))
        # Both endpoints: a forced push also counts the commits it drops.
        return {("old", "a"): "a.rs\nshared.rs\n", ("base", "b"): "b.ts\nshared.rs\n"}[
            arguments[3:]
        ]

    def test_a_new_ref_counts_from_its_own_tip(self):
        with (
            mock.patch.object(pre_push, "git", side_effect=self.git),
            mock.patch.object(pre_push, "known", return_value=True),
        ):
            self.assertEqual(
                pre_push.changed_files([("old", "a"), (None, "b")], "origin/ovcs"),
                ["a.rs", "b.ts", "shared.rs"],
            )

    def test_without_a_merge_base_the_selection_falls_back(self):
        failure = subprocess.CalledProcessError(1, ["git", "merge-base"])
        with (
            mock.patch.object(pre_push, "git", side_effect=failure),
            mock.patch.object(pre_push, "known", return_value=True),
        ):
            self.assertIsNone(pre_push.changed_files([(None, "b")], "origin/ovcs"))

    def test_without_the_default_base_the_selection_falls_back(self):
        with mock.patch.object(pre_push, "known", return_value=False):
            self.assertIsNone(pre_push.changed_files([(None, "b")], "origin/ovcs"))


class LintChecks(unittest.TestCase):
    def test_runs_selected_lint_checks_only(self):
        document = {
            "checks": [
                check("lint:rust"),
                check(
                    "lint:hasura", cwd="hasura", command="yarn install && yarn prettify"
                ),
                check("lint:wbraid", selected=False),
                check("jest:ui-core", kind="test"),
            ]
        }
        self.assertEqual(
            pre_push.lint_checks(document, True, lambda tool: True),
            (["lint:rust", "lint:hasura"], []),
        )

    def test_frontend_lint_needs_installed_dependencies(self):
        document = {
            "checks": [
                check("lint:ui-core", cwd="packages/ui-core", command="yarn lint"),
                check("lint:rust"),
            ]
        }
        self.assertEqual(
            pre_push.lint_checks(document, False, lambda tool: True)[1][0][0],
            "lint:ui-core",
        )

    def test_checks_whose_command_is_missing_are_skipped(self):
        document = {"checks": [check("lint:rust", command="cargo fmt --check")]}
        self.assertEqual(
            pre_push.lint_checks(document, True, lambda tool: tool != "cargo"),
            ([], [("lint:rust", "needs cargo")]),
        )

    def test_checks_whose_tools_are_missing_are_skipped(self):
        document = {
            "checks": [
                check("reuse", command="reuse lint", requires=["reuse"]),
                check("lint:rust"),
            ]
        }
        self.assertEqual(
            pre_push.lint_checks(document, True, lambda tool: tool != "reuse"),
            (["lint:rust"], [("reuse", "needs reuse")]),
        )


if __name__ == "__main__":
    unittest.main()
