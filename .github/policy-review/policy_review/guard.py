# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Self-protection: noticing when the policy review system itself is changed.

Every other policy asks "is this change allowed?". This one asks a different
question: "does this change alter the thing that decides what is allowed?"

A pull request that edits the policies, the caller workflow, the reusable
workflow or the engine can weaken or disable enforcement for everything that
follows — quietly, and in a diff that otherwise looks like routine maintenance.
Such a change is not forbidden; the system has to be maintainable. But it must
never pass unremarked, so it is always surfaced in the review comment and always
announced in Slack, even when the review finds nothing else wrong.
"""

from __future__ import annotations

import fnmatch
from dataclasses import dataclass

# Patterns guarded in every repository, on top of the configured policies
# directory. A pattern that matches nothing in a given repository is harmless —
# the engine directory, for instance, exists only where the engine is hosted.
DEFAULT_GUARDED_PATHS: tuple[str, ...] = (
    ".github/workflows/policy-check.yml",
    ".github/workflows/policy-review.yml",
    ".github/policy-review/**",
)

# The engine's own lint and test jobs live inside each repository's shared lint
# and test workflows rather than a workflow of their own. Those files are not
# guarded: they change constantly for unrelated reasons, and guarding them would
# fire this notice on every frontend or Rust change until people stopped reading
# it. Removing the engine's job from one of them is instead caught by the
# "changes must be checked" policy, which treats narrowing an existing check's
# reach as a blocking violation.

# Ordered most to least specific, so a path inside the engine is described as
# engine code rather than as a generic match.
_DESCRIPTIONS: tuple[tuple[str, str], ...] = (
    ("/tests/", "engine test"),
    (".github/policy-review/", "engine code"),
    ("policy-review.yml", "the reusable policy workflow"),
    ("policy-check.yml", "this repository's policy check workflow"),
)


@dataclass(frozen=True)
class GuardHit:
    """One changed file that belongs to the policy review system."""

    path: str
    reason: str

    def __str__(self) -> str:
        return f"{self.path} ({self.reason})"


def _matches(path: str, pattern: str) -> bool:
    """Match a repository-relative path against one guarded pattern.

    ``fnmatch`` treats ``*`` as crossing directory separators, which would make
    a trailing ``/**`` behave surprisingly, so recursive patterns are handled as
    an explicit prefix test.
    """
    if pattern.endswith("/**"):
        return path.startswith(pattern[:-2])
    return fnmatch.fnmatch(path, pattern)


def _describe(path: str, policies_prefix: str) -> str:
    if path.startswith(policies_prefix):
        return "policy file"
    for needle, description in _DESCRIPTIONS:
        if needle in path:
            return description
    return "policy review system file"


def find_hits(
    changed_paths: list[str],
    policies_path: str,
    guarded_paths: tuple[str, ...] = DEFAULT_GUARDED_PATHS,
) -> list[GuardHit]:
    """Which changed files belong to the policy review system.

    The configured policies directory is always guarded, whether or not it
    appears in ``guarded_paths``: it is the one part of the system that exists
    in every repository, and the part most likely to be edited.
    """
    policies_prefix = policies_path.strip("/") + "/"
    patterns = (*guarded_paths, f"{policies_prefix}**")

    hits: list[GuardHit] = []
    for path in changed_paths:
        if any(_matches(path, pattern) for pattern in patterns):
            hits.append(GuardHit(path=path, reason=_describe(path, policies_prefix)))
    return sorted(hits, key=lambda hit: hit.path)


def touched_policies(hits: list[GuardHit]) -> list[str]:
    """Just the policy files among the hits, for the reviewer's prompt."""
    return [hit.path for hit in hits if hit.reason == "policy file"]
