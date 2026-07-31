# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Rendering of the pull request comment."""

from __future__ import annotations

import unittest

from policy_review.github_api import COMMENT_MARKER
from policy_review.report import (
    render_comment,
    render_error_comment,
    render_review_body,
)
from policy_review.verdict import Verdict, Violation


def violation(**overrides) -> Violation:
    defaults = dict(
        policy_id="10-scope",
        policy_title="Repository scope",
        severity="blocker",
        path="charts/app/Chart.yaml",
        explanation="A Helm chart is shared code and does not belong here.",
        remediation="Move it to the repository that owns shared code.",
        line=None,
    )
    defaults.update(overrides)
    return Violation(**defaults)


class PassCommentTests(unittest.TestCase):
    def setUp(self):
        self.comment = render_comment(
            Verdict(summary="Nothing to flag."),
            policies_source="origin/main",
            policy_count=3,
            blocking_count=0,
        )

    def test_carries_the_marker_so_the_comment_can_be_updated(self):
        self.assertTrue(self.comment.startswith(COMMENT_MARKER))

    def test_states_that_all_policies_passed(self):
        self.assertIn("all policies passed", self.comment)
        self.assertIn("Nothing to flag.", self.comment)

    def test_reports_how_many_policies_ran_and_from_where(self):
        self.assertIn("3 policies", self.comment)
        self.assertIn("origin/main", self.comment)

    def test_uses_the_singular_for_one_policy(self):
        comment = render_comment(
            Verdict(summary="ok"),
            policies_source="origin/main",
            policy_count=1,
            blocking_count=0,
        )
        self.assertIn("1 policy", comment)


class ViolationCommentTests(unittest.TestCase):
    def setUp(self):
        self.verdict = Verdict(
            summary="One boundary crossed.",
            violations=[violation(line=4), violation(policy_id="20-secrets", severity="warning")],
        )
        self.comment = render_comment(
            self.verdict,
            policies_source="origin/main",
            policy_count=2,
            blocking_count=1,
        )

    def test_counts_the_violations_in_the_heading(self):
        self.assertIn("2 violations found", self.comment)

    def test_says_how_many_block_the_merge(self):
        self.assertIn("1 of 2 must be resolved", self.comment)

    def test_includes_the_location_of_each_finding(self):
        self.assertIn("charts/app/Chart.yaml:4", self.comment)

    def test_includes_the_policy_and_the_fix(self):
        self.assertIn("Repository scope", self.comment)
        self.assertIn("`10-scope`", self.comment)
        self.assertIn("How to fix", self.comment)

    def test_marks_severities_distinctly(self):
        self.assertIn("blocker", self.comment)
        self.assertIn("warning", self.comment)

    def test_uses_the_singular_for_one_violation(self):
        comment = render_comment(
            Verdict(summary="s", violations=[violation()]),
            policies_source="origin/main",
            policy_count=1,
            blocking_count=1,
        )
        self.assertIn("1 violation found", comment)

    def test_says_when_nothing_blocks(self):
        comment = render_comment(
            Verdict(summary="s", violations=[violation(severity="info")]),
            policies_source="origin/main",
            policy_count=1,
            blocking_count=0,
        )
        self.assertIn("None of these block the merge", comment)

    def test_surfaces_a_truncated_diff_to_the_reader(self):
        comment = render_comment(
            self.verdict,
            policies_source="origin/main",
            policy_count=2,
            blocking_count=1,
            diff_truncated=True,
        )
        self.assertIn("too large to review in full", comment)

    def test_explains_that_policies_come_from_the_target_branch(self):
        self.assertIn("read from the target branch", self.comment)


class ReviewBodyTests(unittest.TestCase):
    def test_lists_only_the_blocking_findings(self):
        body = render_review_body(
            Verdict(summary="s", violations=[violation(), violation(severity="info")]),
            [violation()],
        )
        self.assertIn("1 blocking violation", body)
        self.assertIn("charts/app/Chart.yaml", body)

    def test_keeps_each_finding_on_one_line(self):
        body = render_review_body(
            Verdict(summary="s", violations=[violation()]),
            [violation(explanation="line one\nline two")],
        )
        self.assertNotIn("line one\nline two", body)
        self.assertIn("line one line two", body)


class ErrorCommentTests(unittest.TestCase):
    def setUp(self):
        self.comment = render_error_comment("the model was unreachable")

    def test_is_explicitly_not_a_pass(self):
        # A review that did not run must never read like a clean one.
        self.assertIn("not** a pass", self.comment)

    def test_states_the_reason(self):
        self.assertIn("the model was unreachable", self.comment)

    def test_carries_the_marker(self):
        self.assertTrue(self.comment.startswith(COMMENT_MARKER))


if __name__ == "__main__":
    unittest.main()
