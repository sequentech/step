# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Policy discovery and loading, including the base-branch guarantee."""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from policy_review.policies import (
    Policy,
    PolicyLoadError,
    load_policies,
    policy_files_touched,
    render_for_prompt,
)

POLICY_DIR = ".github/policies"


def git(repo: Path, *args: str) -> None:
    subprocess.run(
        ["git", *args],
        cwd=str(repo),
        check=True,
        capture_output=True,
        text=True,
    )


class GitRepoTestCase(unittest.TestCase):
    """A throwaway git repository with a policies directory on `main`."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

        git(self.repo, "init", "--quiet", "--initial-branch=main")
        git(self.repo, "config", "user.email", "test@example.com")
        git(self.repo, "config", "user.name", "Test")
        git(self.repo, "config", "commit.gpgsign", "false")

        self.policies = self.repo / POLICY_DIR
        self.policies.mkdir(parents=True)

    def write(self, name: str, text: str) -> None:
        (self.policies / name).write_text(text, encoding="utf-8")

    def commit(self, message: str = "policies") -> None:
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "--quiet", "-m", message)


class LoadPoliciesTests(GitRepoTestCase):
    def test_loads_markdown_policies_from_the_base_ref(self):
        self.write("10-scope.md", "# Repository scope\n\nOnly config here.\n")
        self.write("20-secrets.md", "# No secrets\n\nNever commit credentials.\n")
        self.commit()

        policies, source = load_policies(POLICY_DIR, "main", self.repo)

        self.assertEqual(source, "main")
        self.assertEqual([p.policy_id for p in policies], ["10-scope", "20-secrets"])
        self.assertEqual(policies[0].title, "Repository scope")
        self.assertEqual(policies[1].title, "No secrets")

    def test_ignores_readme_and_non_policy_files(self):
        self.write("README.md", "# How this folder works\n\nDocs, not a rule.\n")
        self.write("config.yaml", "not: a policy\n")
        self.write("10-real.md", "# Real policy\n\nA rule.\n")
        self.commit()

        policies, _ = load_policies(POLICY_DIR, "main", self.repo)

        self.assertEqual([p.policy_id for p in policies], ["10-real"])

    def test_skips_empty_policy_files(self):
        self.write("10-empty.md", "\n   \n")
        self.write("20-real.md", "# Real\n\nA rule.\n")
        self.commit()

        policies, _ = load_policies(POLICY_DIR, "main", self.repo)

        self.assertEqual([p.policy_id for p in policies], ["20-real"])

    # The fixture below embeds a licence header as test data. REUSE would
    # otherwise try to parse it as this file's own licence and fail on the
    # escaped newlines, so it is fenced off from the linter.
    # REUSE-IgnoreStart
    def test_strips_an_html_licence_header(self):
        header = "<!--\n" + "SPDX-License-" + "Identifier: AGPL-3.0-only\n" + "-->\n"
        self.write("10-scope.md", header + "\n# Scope\n\nRule.\n")
        self.commit()

        policies, _ = load_policies(POLICY_DIR, "main", self.repo)

        self.assertNotIn("SPDX", policies[0].body)
        self.assertTrue(policies[0].body.startswith("# Scope"))

    # REUSE-IgnoreEnd

    def test_derives_a_title_when_the_file_has_no_heading(self):
        self.write("30-database-naming.md", "Databases use underscores.\n")
        self.commit()

        policies, _ = load_policies(POLICY_DIR, "main", self.repo)

        self.assertEqual(policies[0].title, "database naming")

    def test_reads_the_base_ref_not_the_working_tree(self):
        """A pull request must not be able to weaken its own policies."""
        self.write("10-scope.md", "# Scope\n\nSTRICT RULE\n")
        self.commit()

        # Simulate the pull request rewriting the rule in its own head commit.
        self.write("10-scope.md", "# Scope\n\nANYTHING GOES\n")

        policies, source = load_policies(POLICY_DIR, "main", self.repo)

        self.assertEqual(source, "main")
        self.assertIn("STRICT RULE", policies[0].body)
        self.assertNotIn("ANYTHING GOES", policies[0].body)

    def test_falls_back_to_the_working_tree_without_a_base_ref(self):
        # A manual run has no pull request to guard against.
        self.write("10-scope.md", "# Scope\n\nRule.\n")

        policies, source = load_policies(POLICY_DIR, "", self.repo)

        self.assertEqual(source, "working tree")
        self.assertEqual([p.policy_id for p in policies], ["10-scope"])

    def test_returns_nothing_when_the_directory_is_absent(self):
        policies, _ = load_policies("nope/missing", "", self.repo)
        self.assertEqual(policies, [])

    def test_rejects_a_policies_path_that_escapes_the_repository(self):
        for escaping in ("../outside", "a/../../outside", "/etc"):
            with self.subTest(path=escaping):
                with self.assertRaises(PolicyLoadError):
                    load_policies(escaping, "", self.repo)


class TouchedPolicyFileTests(unittest.TestCase):
    def test_detects_edits_inside_the_policies_directory(self):
        changed = [
            "src/main.py",
            ".github/policies/10-scope.md",
            ".github/policies/nested/20-more.md",
            ".github/policies-other/30-decoy.md",
        ]
        self.assertEqual(
            policy_files_touched(changed, ".github/policies"),
            [".github/policies/10-scope.md", ".github/policies/nested/20-more.md"],
        )

    def test_returns_nothing_when_policies_are_untouched(self):
        self.assertEqual(
            policy_files_touched(["src/main.py"], ".github/policies"), []
        )


class RenderForPromptTests(unittest.TestCase):
    def test_wraps_each_policy_with_its_identity(self):
        rendered = render_for_prompt(
            [Policy(policy_id="10-scope", title="Scope", body="Only config.")]
        )
        self.assertIn('<policy id="10-scope" title="Scope">', rendered)
        self.assertIn("Only config.", rendered)
        self.assertIn("</policy>", rendered)

    def test_says_so_when_there_are_no_policies(self):
        self.assertIn("no policy files", render_for_prompt([]))


if __name__ == "__main__":
    unittest.main()
