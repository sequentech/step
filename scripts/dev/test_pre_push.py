# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for scripts.dev.pre_push: pushed ranges and the lint checks they run."""

import unittest

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

    def test_a_ref_without_base_uses_the_default_selection(self):
        self.assertEqual(pre_push.selection([(None, "1" * 40)]), [])


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

    def test_checks_whose_tools_are_missing_are_skipped(self):
        document = {"checks": [check("reuse", requires=["reuse"]), check("lint:rust")]}
        self.assertEqual(
            pre_push.lint_checks(document, True, lambda tool: tool != "reuse"),
            (["lint:rust"], [("reuse", "needs reuse")]),
        )


if __name__ == "__main__":
    unittest.main()
