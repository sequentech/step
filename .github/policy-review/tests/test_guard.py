# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Self-protection: noticing when the policy review system itself is changed."""

from __future__ import annotations

import unittest

from policy_review.guard import (
    DEFAULT_GUARDED_PATHS,
    GuardHit,
    find_hits,
    touched_policies,
)

POLICIES = ".github/policies"


def paths(hits: list[GuardHit]) -> list[str]:
    return [hit.path for hit in hits]


def reason_for(hits: list[GuardHit], path: str) -> str:
    return next(hit.reason for hit in hits if hit.path == path)


class DetectionTests(unittest.TestCase):
    def test_ordinary_changes_are_not_flagged(self):
        changed = ["src/main.py", "docs/guide.md", "charts/app/values.yaml"]
        self.assertEqual(find_hits(changed, POLICIES), [])

    def test_flags_an_edited_policy_file(self):
        hits = find_hits([f"{POLICIES}/10-scope.md"], POLICIES)
        self.assertEqual(paths(hits), [f"{POLICIES}/10-scope.md"])
        self.assertEqual(hits[0].reason, "policy file")

    def test_flags_a_policy_file_in_a_subdirectory(self):
        hits = find_hits([f"{POLICIES}/nested/20-more.md"], POLICIES)
        self.assertEqual(len(hits), 1)

    def test_flags_a_deleted_policy_file(self):
        # Deleting a rule is the most direct way to weaken enforcement, and it
        # appears in the changed-path list exactly like an edit.
        hits = find_hits([f"{POLICIES}/10-scope.md"], POLICIES)
        self.assertEqual(len(hits), 1)

    def test_flags_the_caller_workflow(self):
        hits = find_hits([".github/workflows/policy-check.yml"], POLICIES)
        self.assertEqual(len(hits), 1)
        self.assertIn("policy check workflow", hits[0].reason)

    def test_flags_the_reusable_workflow(self):
        hits = find_hits([".github/workflows/policy-review.yml"], POLICIES)
        self.assertEqual(len(hits), 1)
        self.assertIn("reusable policy workflow", hits[0].reason)

    def test_flags_engine_code(self):
        hits = find_hits([".github/policy-review/policy_review/verdict.py"], POLICIES)
        self.assertEqual(len(hits), 1)
        self.assertEqual(hits[0].reason, "engine code")

    def test_flags_engine_tests_distinctly(self):
        hits = find_hits([".github/policy-review/tests/test_verdict.py"], POLICIES)
        self.assertEqual(hits[0].reason, "engine test")

    def test_does_not_flag_a_lookalike_directory(self):
        changed = [
            ".github/policies-draft/10-scope.md",
            ".github/workflows/policy-check-notes.md",
        ]
        self.assertEqual(find_hits(changed, POLICIES), [])

    def test_flags_only_the_system_files_in_a_mixed_change(self):
        changed = [
            "src/main.py",
            f"{POLICIES}/10-scope.md",
            ".github/workflows/deploy.yml",
            ".github/policy-review/policy_review/runner.py",
        ]
        self.assertEqual(
            paths(find_hits(changed, POLICIES)),
            # Sorted: ".github/policies/" precedes ".github/policy-review/".
            [f"{POLICIES}/10-scope.md", ".github/policy-review/policy_review/runner.py"],
        )

    def test_results_are_sorted_for_a_stable_comment(self):
        changed = [
            f"{POLICIES}/30-c.md",
            f"{POLICIES}/10-a.md",
            f"{POLICIES}/20-b.md",
        ]
        self.assertEqual(paths(find_hits(changed, POLICIES)), sorted(changed))

    def test_honours_a_custom_policies_path(self):
        hits = find_hits(["rules/10-scope.md"], "rules")
        self.assertEqual(hits[0].reason, "policy file")

    def test_the_policies_path_is_guarded_even_if_not_listed(self):
        # The configured directory is always guarded: it is the part of the
        # system present in every repository and the likeliest to be edited.
        hits = find_hits([f"{POLICIES}/10-scope.md"], POLICIES, guarded_paths=())
        self.assertEqual(len(hits), 1)


class CustomPatternTests(unittest.TestCase):
    def test_accepts_a_custom_recursive_pattern(self):
        hits = find_hits(
            ["tooling/review/engine.py", "tooling/other.py"],
            POLICIES,
            guarded_paths=("tooling/review/**",),
        )
        self.assertEqual(paths(hits), ["tooling/review/engine.py"])

    def test_accepts_a_custom_exact_pattern(self):
        hits = find_hits(
            ["ci/guard.yml", "ci/other.yml"],
            POLICIES,
            guarded_paths=("ci/guard.yml",),
        )
        self.assertEqual(paths(hits), ["ci/guard.yml"])

    def test_accepts_a_custom_glob(self):
        hits = find_hits(
            ["ci/policy-a.yml", "ci/build.yml"],
            POLICIES,
            guarded_paths=("ci/policy-*.yml",),
        )
        self.assertEqual(paths(hits), ["ci/policy-a.yml"])

    def test_a_recursive_pattern_does_not_match_a_sibling_prefix(self):
        hits = find_hits(
            ["tooling/review-notes/x.py"],
            POLICIES,
            guarded_paths=("tooling/review/**",),
        )
        self.assertEqual(hits, [])


class TouchedPoliciesTests(unittest.TestCase):
    def test_extracts_only_the_policy_files(self):
        hits = find_hits(
            [
                f"{POLICIES}/10-scope.md",
                ".github/policy-review/policy_review/runner.py",
            ],
            POLICIES,
        )
        self.assertEqual(touched_policies(hits), [f"{POLICIES}/10-scope.md"])

    def test_returns_nothing_when_no_policy_was_edited(self):
        hits = find_hits([".github/workflows/policy-check.yml"], POLICIES)
        self.assertEqual(touched_policies(hits), [])


class DefaultsTests(unittest.TestCase):
    def test_the_built_in_list_covers_the_whole_system(self):
        for expected in (
            ".github/workflows/policy-check.yml",
            ".github/workflows/policy-review.yml",
            ".github/policy-review/**",
        ):
            self.assertIn(expected, DEFAULT_GUARDED_PATHS)


if __name__ == "__main__":
    unittest.main()
