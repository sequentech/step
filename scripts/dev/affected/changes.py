# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Changed files against the merge base with a base branch."""

from __future__ import annotations

import enum
import os
import subprocess
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

GIT_STATUS = {
    "A": "added",
    "M": "modified",
    "D": "deleted",
    "T": "type-changed",
    "U": "unmerged",
}


class Status(enum.Enum):
    ADDED = "added"
    MODIFIED = "modified"
    DELETED = "deleted"
    TYPE_CHANGED = "type-changed"
    UNMERGED = "unmerged"
    UNTRACKED = "untracked"
    # Given on the command line; nothing is known about how it changed.
    LISTED = "listed"


class BaseSource(enum.Enum):
    """Where the base branch came from."""

    ARGUMENT = "argument"
    UPSTREAM = "upstream"
    DEFAULT = "default"
    # Explicit file lists compare with nothing.
    NONE = "none"


class Scope(enum.Enum):
    """Which changes count: commits since the merge base, or the working tree too."""

    COMMITTED = "committed"
    WORKTREE = "worktree"
    FILES = "files"


class GitError(Exception):
    pass


@dataclass(frozen=True)
class Change:
    path: str
    status: Status


@dataclass
class ChangeSet:
    root: Path
    scope: Scope
    changes: list[Change]
    base_ref: str | None = None
    base_source: BaseSource = BaseSource.NONE
    base_commit: str | None = None
    merge_base: str | None = None
    head: str | None = None
    # Why the changes may be incomplete; any problem selects every check.
    problems: list[str] = field(default_factory=list)

    def old_text(self, path: str) -> str | None:
        """The file at the merge base, or None when unknown or absent."""
        if self.merge_base is None:
            return None
        return show(self.root, self.merge_base, path)

    def new_text(self, path: str) -> str | None:
        if self.scope is Scope.COMMITTED:
            return show(self.root, "HEAD", path)
        try:
            return (self.root / path).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            return None


def git(root: Path, *arguments: str) -> str:
    try:
        result = subprocess.run(
            ["git", *arguments],
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as error:
        raise GitError("git is not installed") from error
    if result.returncode != 0:
        raise GitError(result.stderr.strip() or f"git {arguments[0]} failed")
    return result.stdout


def show(root: Path, revision: str, path: str) -> str | None:
    try:
        return git(root, "show", f"{revision}:{path}")
    except GitError:
        return None


def resolve_commit(root: Path, ref: str) -> str | None:
    try:
        return git(
            root, "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}"
        ).strip()
    except GitError:
        return None


def upstream(root: Path) -> str | None:
    """The branch's upstream, unless it is the same branch on a remote.

    Pushing with -u makes a branch track its own remote copy, which is not the
    base of its changes.
    """
    try:
        branch = git(root, "symbolic-ref", "--quiet", "--short", "HEAD").strip()
        tracked = git(root, "rev-parse", "--abbrev-ref", "@{upstream}").strip()
    except GitError:
        return None
    _, _, tracked_branch = tracked.partition("/")
    return None if tracked_branch == branch else tracked


def parse_name_status(output: str) -> list[Change]:
    """``git diff --name-status -z --no-renames`` output."""
    fields = output.split("\0")
    changes = []
    for index in range(0, len(fields) - 1, 2):
        letter, path = fields[index], fields[index + 1]
        status = Status(GIT_STATUS.get(letter[:1], Status.MODIFIED.value))
        changes.append(Change(path, status))
    return changes


def shallow(root: Path) -> bool:
    try:
        return git(root, "rev-parse", "--is-shallow-repository").strip() == "true"
    except GitError:
        return False


def normalize(root: Path, path: str, cwd: Path) -> str:
    """A repository-relative POSIX path for a path given relative to ``cwd``."""
    absolute = Path(os.path.normpath(cwd / path))
    relative = PurePosixPath(os.path.relpath(absolute, root))
    if relative.parts[:1] == ("..",) or relative.is_absolute():
        raise ValueError(f"{path} is outside the repository")
    return relative.as_posix()


def from_files(root: Path, paths: Iterable[str], cwd: Path) -> ChangeSet:
    changes = sorted(
        {Change(normalize(root, path, cwd), Status.LISTED) for path in paths if path},
        key=lambda change: change.path,
    )
    return ChangeSet(root=root, scope=Scope.FILES, changes=changes)


def collect(root: Path, base: str | None, default_base: str, scope: Scope) -> ChangeSet:
    """Changes since the merge base of HEAD and the base branch.

    Without a usable merge base the set is empty and records the problem, which
    makes the selection fall back to every check.
    """
    if base is not None:
        ref, source = base, BaseSource.ARGUMENT
    else:
        tracked = upstream(root)
        ref, source = (
            (tracked, BaseSource.UPSTREAM)
            if tracked
            else (default_base, BaseSource.DEFAULT)
        )
    changeset = ChangeSet(
        root=root, scope=scope, changes=[], base_ref=ref, base_source=source
    )
    changeset.head = resolve_commit(root, "HEAD")
    changeset.base_commit = resolve_commit(root, ref)
    if changeset.base_commit is None:
        changeset.problems.append(
            f"base {ref} is not a known commit; fetch it or pass --base"
        )
        return changeset
    if changeset.head is None:
        changeset.problems.append("HEAD is not a commit")
        return changeset
    try:
        changeset.merge_base = git(
            root, "merge-base", changeset.base_commit, changeset.head
        ).strip()
    except GitError:
        hint = (
            "; this is a shallow clone, fetch more history (git fetch --deepen or "
            "--unshallow)"
            if shallow(root)
            else ""
        )
        changeset.problems.append(f"HEAD and {ref} have no merge base{hint}")
        return changeset
    diff: Sequence[str] = ("diff", "--name-status", "-z", "--no-renames")
    if scope is Scope.COMMITTED:
        output = git(root, *diff, changeset.merge_base, "HEAD")
    else:
        output = git(root, *diff, changeset.merge_base)
    changes = {change.path: change for change in parse_name_status(output)}
    if scope is Scope.WORKTREE:
        untracked = git(root, "ls-files", "--others", "--exclude-standard", "-z")
        for path in filter(None, untracked.split("\0")):
            changes.setdefault(path, Change(path, Status.UNTRACKED))
    changeset.changes = sorted(changes.values(), key=lambda change: change.path)
    return changeset
