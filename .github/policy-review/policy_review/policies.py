# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Discovery and loading of policy files.

Policies are read from the **base branch**, never from the pull request's head.
A pull request that edits its own policy files therefore cannot weaken the rules
it is about to be judged against; the edit is reported instead.
"""

from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

POLICY_SUFFIXES = (".md", ".markdown", ".txt")

# Leading licence/SPDX blocks carry no policy meaning. Stripping them keeps the
# prompt free of boilerplate that is repeated in every file.
#
# The hash form must not swallow a Markdown `# Heading`, which is how most
# policies open, so it only applies to a leading comment block that actually
# mentions SPDX.
_HTML_COMMENT = re.compile(r"\A\s*<!--.*?-->\s*", re.DOTALL)
_HASH_HEADER = re.compile(r"\A(?:[ \t]*#[^\n]*\n)+")
_SPDX = re.compile(r"SPDX-(?:FileCopyrightText|License-Identifier)")

_TITLE = re.compile(r"^#\s+(.+?)\s*$", re.MULTILINE)


class PolicyLoadError(RuntimeError):
    """Raised when the configured policies directory cannot be read."""


@dataclass(frozen=True)
class Policy:
    """A single policy file."""

    policy_id: str
    title: str
    body: str

    @property
    def is_empty(self) -> bool:
        return not self.body.strip()


def _strip_licence_header(text: str) -> str:
    """Remove a leading licence block, leaving the policy body untouched."""
    match = _HTML_COMMENT.match(text)
    if match and _SPDX.search(match.group(0)):
        return text[match.end() :].lstrip("\n")

    match = _HASH_HEADER.match(text)
    if match and _SPDX.search(match.group(0)):
        return text[match.end() :].lstrip("\n")

    return text.lstrip("\n")


def _title_of(policy_id: str, body: str) -> str:
    match = _TITLE.search(body)
    if match:
        return match.group(1).strip()
    # Fall back to a readable form of the filename: "10-repository-scope" reads
    # as "repository scope".
    stem = re.sub(r"^\d+[-_]", "", policy_id)
    return stem.replace("-", " ").replace("_", " ").strip() or policy_id


def _git(args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )


def base_ref_available(base_ref: str, repo_root: Path) -> bool:
    """Whether ``base_ref`` can be resolved locally."""
    if not base_ref:
        return False
    return _git(["rev-parse", "--verify", "--quiet", f"{base_ref}^{{commit}}"], repo_root).returncode == 0


def _list_from_ref(policies_path: str, base_ref: str, repo_root: Path) -> list[str]:
    result = _git(["ls-tree", "-r", "--name-only", base_ref, "--", policies_path], repo_root)
    if result.returncode != 0:
        raise PolicyLoadError(
            f"could not list {policies_path!r} at {base_ref!r}: {result.stderr.strip()}"
        )
    return [line for line in result.stdout.splitlines() if line.strip()]


def _read_from_ref(path: str, base_ref: str, repo_root: Path) -> str:
    result = _git(["show", f"{base_ref}:{path}"], repo_root)
    if result.returncode != 0:
        raise PolicyLoadError(
            f"could not read {path!r} at {base_ref!r}: {result.stderr.strip()}"
        )
    return result.stdout


def _list_from_worktree(policies_path: str, repo_root: Path) -> list[str]:
    directory = repo_root / policies_path
    if not directory.is_dir():
        return []
    return sorted(
        str(item.relative_to(repo_root))
        for item in directory.rglob("*")
        if item.is_file()
    )


def _check_contained(policies_path: str, repo_root: Path) -> None:
    """Reject a policies path that points outside the repository.

    ``policies_path`` comes from the caller's workflow and is trusted, so this
    guards against a typo rather than an attack — but a path that escaped the
    checkout would read files nobody meant to publish into a prompt.
    """
    if Path(policies_path).is_absolute():
        raise PolicyLoadError(f"policies_path must be relative, got {policies_path!r}")
    resolved = (repo_root / policies_path).resolve()
    root = repo_root.resolve()
    if resolved != root and root not in resolved.parents:
        raise PolicyLoadError(
            f"policies_path must stay inside the repository, got {policies_path!r}"
        )


def load_policies(
    policies_path: str,
    base_ref: str,
    repo_root: Path,
) -> tuple[list[Policy], str]:
    """Load every policy file under ``policies_path``.

    Returns the policies and the source they were read from — the base ref when
    it resolves, otherwise the working tree (which is the case for a manual
    ``workflow_dispatch`` run, where there is no pull request to guard against).
    """
    _check_contained(policies_path, repo_root)
    use_ref = base_ref_available(base_ref, repo_root)
    source = base_ref if use_ref else "working tree"

    if use_ref:
        candidates = _list_from_ref(policies_path, base_ref, repo_root)
    else:
        candidates = _list_from_worktree(policies_path, repo_root)

    policies: list[Policy] = []
    for path in sorted(candidates):
        if not path.lower().endswith(POLICY_SUFFIXES):
            continue
        name = Path(path).name
        if name.lower() == "readme.md":
            # A directory README documents the folder; it is not a rule.
            continue

        raw = (
            _read_from_ref(path, base_ref, repo_root)
            if use_ref
            else (repo_root / path).read_text(encoding="utf-8")
        )
        body = _strip_licence_header(raw)
        policy_id = Path(path).stem
        policy = Policy(policy_id=policy_id, title=_title_of(policy_id, body), body=body)
        if policy.is_empty:
            continue
        policies.append(policy)

    return policies, source


def render_for_prompt(policies: list[Policy]) -> str:
    """Render policies into the block injected into the system prompt."""
    if not policies:
        return "(no policy files found)"
    chunks = []
    for policy in policies:
        chunks.append(
            f'<policy id="{policy.policy_id}" title="{policy.title}">\n'
            f"{policy.body.strip()}\n"
            f"</policy>"
        )
    return "\n\n".join(chunks)
