# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Mechanical enforcement of the non-disclosure constraint.

This repository is public. The policy review engine that lives here must not
name, depend on, or otherwise reveal any repository that is not public — and
that must hold as people add policies and edit workflows over time, not just on
the day this was written.

The check is deliberately expressed without naming any private repository: it
asserts that the only repository referenced is this one. That way the test can
live in the open without itself disclosing what it is guarding against.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

# The only repository these files may reference.
ALLOWED_REPOSITORIES = {"step"}

# Owner/name pairs, whether written as a `uses:` target, a clone target or a URL.
_REPO_REFERENCE = re.compile(r"\bsequentech/([A-Za-z0-9._-]+)")

# Any github.com repository URL, so a link is caught even without the org prefix
# being adjacent to a slash-separated owner.
_GITHUB_URL = re.compile(r"github\.com[:/]([A-Za-z0-9._-]+)/([A-Za-z0-9._-]+)")

SCANNED_DIRECTORIES = (
    ".github/policy-review",
    ".github/policies",
)

SCANNED_WORKFLOWS = (
    ".github/workflows/policy-review.yml",
    ".github/workflows/policy-check.yml",
    # The engine's lint and test jobs live in the repository's shared lint and
    # test workflows, so those are scanned too — a `uses:` pointing at another
    # repository would disclose it just as readily from there.
    ".github/workflows/lint_prettify.yml",
    ".github/workflows/tests.yml",
)

TEXT_SUFFIXES = {".py", ".yml", ".yaml", ".md", ".txt", ".json", ".cfg", ".toml"}


def scanned_files() -> list[Path]:
    """Every file this constraint applies to."""
    found: list[Path] = []
    for directory in SCANNED_DIRECTORIES:
        root = REPO_ROOT / directory
        if not root.is_dir():
            continue
        found.extend(
            path
            for path in sorted(root.rglob("*"))
            if path.is_file() and path.suffix in TEXT_SUFFIXES and "__pycache__" not in path.parts
        )
    found.extend(
        REPO_ROOT / workflow for workflow in SCANNED_WORKFLOWS if (REPO_ROOT / workflow).is_file()
    )
    return found


class NonDisclosureTests(unittest.TestCase):
    def setUp(self):
        self.files = scanned_files()
        self.assertTrue(self.files, "found no files to check — has the layout moved?")

    def test_references_no_repository_other_than_this_one(self):
        offenders: list[str] = []
        for path in self.files:
            text = path.read_text(encoding="utf-8", errors="replace")
            for name in _REPO_REFERENCE.findall(text):
                if name not in ALLOWED_REPOSITORIES:
                    offenders.append(f"{path.relative_to(REPO_ROOT)} -> {name}")
        self.assertEqual(
            offenders,
            [],
            "These public files reference a repository that is not this one. "
            "Public files must not disclose other repositories; move the rule "
            "into the repository it applies to.\n" + "\n".join(offenders),
        )

    def test_links_to_no_repository_other_than_this_one(self):
        offenders: list[str] = []
        for path in self.files:
            text = path.read_text(encoding="utf-8", errors="replace")
            for owner, name in _GITHUB_URL.findall(text):
                # Third-party actions and unrelated projects are fine; only this
                # organisation's other repositories are in scope.
                if owner != "sequentech":
                    continue
                if name not in ALLOWED_REPOSITORIES:
                    offenders.append(f"{path.relative_to(REPO_ROOT)} -> {owner}/{name}")
        self.assertEqual(offenders, [], "\n".join(offenders))


def without_comments(text: str) -> str:
    """Drop whole-line comments, so prose about a risk is not read as the risk."""
    return "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("#"))


class WorkflowSafetyTests(unittest.TestCase):
    def test_the_policy_workflows_never_use_pull_request_target(self):
        """`pull_request_target` would hand repository secrets to fork code."""
        for workflow in SCANNED_WORKFLOWS:
            path = REPO_ROOT / workflow
            if not path.is_file():
                continue
            with self.subTest(workflow=workflow):
                self.assertNotIn(
                    "pull_request_target",
                    without_comments(path.read_text(encoding="utf-8")),
                    f"{workflow} must trigger on pull_request, not "
                    "pull_request_target: the latter runs with access to "
                    "secrets on code the pull request author controls.",
                )

    def test_the_reusable_workflow_requests_least_privilege(self):
        text = (REPO_ROOT / ".github/workflows/policy-review.yml").read_text(encoding="utf-8")
        self.assertIn("contents: read", text)
        self.assertIn("pull-requests: write", text)
        for excessive in ("contents: write", "permissions: write-all", "id-token: write"):
            self.assertNotIn(excessive, text, f"unexpected permission: {excessive}")


class PublicPolicyContentTests(unittest.TestCase):
    def test_this_repository_ships_at_least_one_policy(self):
        policies = REPO_ROOT / ".github/policies"
        self.assertTrue(policies.is_dir(), "no policies directory")
        files = [path for path in policies.glob("*.md") if path.name.lower() != "readme.md"]
        self.assertTrue(files, "no policy files found")


if __name__ == "__main__":
    unittest.main()
