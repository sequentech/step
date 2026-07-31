# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Rendering the pull request comment.

The comment is what an engineer actually reads, so it leads with the decision
and gives every finding a file, a reason and a fix.
"""

from __future__ import annotations

from .github_api import COMMENT_MARKER
from .guard import GuardHit
from .verdict import Verdict, Violation

SEVERITY_LABEL = {
    "blocker": "🔴 blocker",
    "warning": "🟠 warning",
    "info": "🔵 info",
}

_FOOTER = (
    "<sub>Automated policy review. Policies are read from the target branch, "
    "so editing them in this pull request does not change this result.</sub>"
)


def _escape(text: str) -> str:
    """Neutralise Markdown that would break out of a table cell."""
    return text.replace("|", "\\|").replace("\n", " ").strip()


def _violation_section(violation: Violation, index: int) -> str:
    label = SEVERITY_LABEL.get(violation.severity, violation.severity)
    lines = [
        f"#### {index}. {label} — `{violation.location}`",
        "",
        f"**Policy:** {violation.policy_title} (`{violation.policy_id}`)",
        "",
        violation.explanation.strip(),
        "",
        f"**How to fix:** {violation.remediation.strip()}",
    ]
    return "\n".join(lines)


def render_guard_notice(hits: list[GuardHit]) -> str:
    """Render the banner shown when the policy review system is changed.

    Deliberately placed above the verdict. A change to the machinery outranks
    the verdict that machinery just produced, because it affects every pull
    request after this one.
    """
    count = len(hits)
    subject = "1 file" if count == 1 else f"{count} files"
    verb = "is" if count == 1 else "are"

    lines = [
        "> [!WARNING]",
        "> ### This pull request changes the policy review system itself",
        ">",
        f"> {subject} belonging to the automated policy check {verb} modified"
        " here. A change to these files can weaken or disable enforcement for"
        " **every pull request that follows**, not just this one.",
        ">",
    ]
    lines += [f"> - `{hit.path}` — {hit.reason}" for hit in hits]
    lines += [
        ">",
        "> This is allowed — the system has to be maintainable — but it needs a"
        " deliberate human review rather than a routine one. A Slack alert has"
        " been sent.",
    ]
    return "\n".join(lines)


def render_comment(
    verdict: Verdict,
    *,
    policies_source: str,
    policy_count: int,
    blocking_count: int,
    diff_truncated: bool = False,
    guard_hits: list[GuardHit] | None = None,
) -> str:
    """Render the full pull request comment."""
    parts = [COMMENT_MARKER]
    if guard_hits:
        parts += [render_guard_notice(guard_hits), ""]

    if verdict.passed:
        parts += [
            "### ✅ Policy review — all policies passed",
            "",
            verdict.summary.strip(),
        ]
    else:
        total = len(verdict.violations)
        heading = f"### ❌ Policy review — {total} violation{'s' if total != 1 else ''} found"
        parts += [heading, "", verdict.summary.strip(), ""]
        if blocking_count:
            parts.append(
                f"**{blocking_count} of {total} must be resolved before this "
                "pull request can merge.**"
            )
        else:
            parts.append("None of these block the merge, but please read them before merging.")
        parts += ["", "---", ""]
        parts += [
            _violation_section(violation, index)
            for index, violation in enumerate(verdict.violations, start=1)
        ]

    notes = []
    if diff_truncated:
        notes.append(
            "This pull request was too large to review in full; only part of the diff was examined."
        )
    if notes:
        parts += ["", "---", "", *(f"> ⚠️ {note}" for note in notes)]

    parts += [
        "",
        "---",
        "",
        f"<sub>Reviewed against {policy_count} "
        f"{'policy' if policy_count == 1 else 'policies'} "
        f"from `{policies_source}`.</sub>",
        "",
        _FOOTER,
    ]
    return "\n".join(parts)


def render_review_body(verdict: Verdict, blocking: list[Violation]) -> str:
    """Render the shorter body attached to a formal "changes requested" review."""
    count = len(blocking)
    lines = [
        f"Policy review found {count} blocking violation"
        f"{'s' if count != 1 else ''} that must be resolved before merge:",
        "",
    ]
    for violation in blocking:
        lines.append(
            f"- **`{violation.location}`** — {violation.policy_title} "
            f"(`{violation.policy_id}`): {_escape(violation.explanation)}"
        )
    lines += ["", "See the full policy review comment on this pull request for the fixes."]
    return "\n".join(lines)


def render_error_comment(reason: str, guard_hits: list[GuardHit] | None = None) -> str:
    """Render the comment used when the review could not be completed.

    Explicitly not a pass: a review that did not run must not read like one.
    """
    parts = [COMMENT_MARKER]
    if guard_hits:
        parts += [render_guard_notice(guard_hits), ""]
    parts += [
        "### ⚠️ Policy review could not be completed",
        "",
        reason.strip(),
        "",
        "This is **not** a pass. Re-run the check, or ask a maintainer to "
        "review the change against the repository's policies by hand.",
        "",
        _FOOTER,
    ]
    return "\n".join(parts)
