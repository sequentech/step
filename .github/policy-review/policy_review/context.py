# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Gathering the pull request context that the review runs against.

Everything here is **untrusted input**: titles, descriptions, file names and
diff hunks are all attacker-controlled on a public repository. Nothing collected
in this module is ever executed, interpolated into a shell command, or treated
as an instruction — see :mod:`policy_review.prompts` for how it is fenced off
before it reaches the model.
"""

from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

# "Related: https://github.com/<owner>/<repo>/issues/<n>", "Parent issue: <url>",
# or a bare URL anywhere in the description.
_ISSUE_URL = re.compile(
    r"https://github\.com/([\w.-]+)/([\w.-]+)/issues/(\d+)", re.IGNORECASE
)

_TRUNCATION_NOTE = "\n\n[diff truncated: {omitted} of {total} bytes omitted]\n"


@dataclass(frozen=True)
class LinkedIssue:
    """An issue referenced from the pull request description."""

    owner: str
    repo: str
    number: int
    title: str = ""
    body: str = ""

    @property
    def url(self) -> str:
        return f"https://github.com/{self.owner}/{self.repo}/issues/{self.number}"

    @property
    def resolved(self) -> bool:
        return bool(self.title)


@dataclass(frozen=True)
class PullRequestContext:
    """Everything known about the pull request under review."""

    number: int
    title: str
    body: str
    author: str
    base_ref: str
    head_sha: str
    is_draft: bool
    changed_paths: list[str] = field(default_factory=list)
    diff: str = ""
    diff_truncated: bool = False
    linked_issue: LinkedIssue | None = None

    @property
    def url_path(self) -> str:
        return f"pull/{self.number}"


def find_issue_reference(body: str) -> tuple[str, str, int] | None:
    """Extract the first ``owner, repo, number`` issue reference in ``body``."""
    if not body:
        return None
    match = _ISSUE_URL.search(body)
    if not match:
        return None
    return match.group(1), match.group(2), int(match.group(3))


def _git(args: list[str], repo_root: Path) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=str(repo_root),
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout


def merge_base(base_ref: str, head_sha: str, repo_root: Path) -> str:
    """Resolve the commit the pull request branched from."""
    return _git(["merge-base", base_ref, head_sha], repo_root).strip()


def changed_paths(base: str, head: str, repo_root: Path) -> list[str]:
    """Files added, modified, renamed or deleted by the pull request."""
    output = _git(["diff", "--name-only", base, head], repo_root)
    return [line for line in output.splitlines() if line.strip()]


def collect_diff(
    base: str,
    head: str,
    repo_root: Path,
    max_bytes: int,
) -> tuple[str, bool]:
    """Return the pull request diff, truncated to ``max_bytes`` if oversized.

    Truncating on a whole-file boundary keeps every hunk the model does see
    syntactically intact, so it never reasons about a half-shown change.
    """
    raw = _git(
        [
            "diff",
            "--unified=3",
            "--no-color",
            # Binary blobs carry no reviewable policy signal and would blow the
            # budget; the file name still appears in the changed-paths list.
            "--diff-filter=ACMRT",
            base,
            head,
        ],
        repo_root,
    )
    encoded = raw.encode("utf-8")
    if len(encoded) <= max_bytes:
        return raw, False

    kept: list[str] = []
    used = 0
    for chunk in raw.split("\ndiff --git ")[:]:
        piece = chunk if not kept else "\ndiff --git " + chunk
        size = len(piece.encode("utf-8"))
        if used + size > max_bytes:
            break
        kept.append(piece)
        used += size

    body = "".join(kept)
    if not body:
        # A single file larger than the whole budget: keep a prefix so the review
        # is still anchored on something real.
        body = encoded[:max_bytes].decode("utf-8", errors="ignore")
    note = _TRUNCATION_NOTE.format(
        omitted=len(encoded) - len(body.encode("utf-8")), total=len(encoded)
    )
    return body + note, True
