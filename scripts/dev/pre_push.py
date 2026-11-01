# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Git pre-push hook: the lint and format checks the pushed commits select.

Enable it once per clone with ``git config core.hooksPath scripts/dev/hooks``.
It runs the ``lint`` checks of ``scripts/dev/affected.toml`` that the pushed
changes select, the same commands as ``lint_prettify.yml``. ``STEP_PRE_PUSH``
chooses what runs: ``lint`` (default), ``tests`` (also the fast tests, as
``step-dev test --affected``) or ``off``. ``git push --no-verify`` skips it.
"""

import json
import os
import shutil
import subprocess
import sys
from collections.abc import Callable, Iterable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ZERO = "0" * 40
MODES = ("lint", "tests", "off")


def pushed_ranges(
    lines: Iterable[str], known: Callable[[str], bool]
) -> list[tuple[str | None, str]]:
    """(base, tip) per pushed ref, from git's ``<local ref> <local sha> <remote
    ref> <remote sha>`` lines. Deletions push nothing; a new ref, or a remote
    tip this clone lacks, has no base."""
    ranges = []
    for line in lines:
        fields = line.split()
        if len(fields) != 4 or fields[1] == ZERO:
            continue
        local, remote = fields[1], fields[3]
        ranges.append((remote if remote != ZERO and known(remote) else None, local))
    return ranges


def lint_checks(
    document: dict,
    has_frontend_dependencies: bool,
    available: Callable[[str], bool] = lambda tool: shutil.which(tool) is not None,
) -> tuple[list[str], list[tuple[str, str]]]:
    """Selected lint check IDs to run, and (ID, reason) for those skipped: a
    required tool is missing, or the Yarn workspace has no dependencies."""
    run, skipped = [], []
    for check in document["checks"]:
        if not check["selected"] or check["kind"] != "lint":
            continue
        missing = [tool for tool in check.get("requires", []) if not available(tool)]
        needs_workspace = check["cwd"].startswith("packages/") and check[
            "command"
        ].startswith("yarn")
        if missing:
            skipped.append((check["id"], f"needs {', '.join(missing)}"))
        elif needs_workspace and not has_frontend_dependencies:
            skipped.append(
                (check["id"], "install packages/ first (yarn --cwd packages install)")
            )
        else:
            run.append(check["id"])
    return run, skipped


def git(*arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def known(sha: str) -> bool:
    return (
        subprocess.run(
            ["git", "cat-file", "-e", f"{sha}^{{commit}}"],
            cwd=ROOT,
            capture_output=True,
        ).returncode
        == 0
    )


def selection(ranges: list[tuple[str | None, str]]) -> list[str]:
    """``step-dev affected`` arguments for the pushed changes. Without a base
    for every ref, the branch's own default base decides."""
    if any(base is None for base, _ in ranges):
        return []
    files = sorted(
        {
            path
            for base, tip in ranges
            for path in git("diff", "--name-only", f"{base}...{tip}").splitlines()
        }
    )
    return ["--files", *files] if files else []


def step_dev(*arguments: str, **options) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(ROOT / "scripts/dev/step-dev"), *arguments], cwd=ROOT, **options
    )


def main() -> int:
    mode = os.environ.get("STEP_PRE_PUSH", "lint")
    if mode not in MODES:
        print(
            f"pre-push: STEP_PRE_PUSH must be one of {', '.join(MODES)}",
            file=sys.stderr,
        )
        return 2
    ranges = pushed_ranges(sys.stdin, known)
    if mode == "off" or not ranges:
        return 0
    arguments = selection(ranges)
    if arguments == [] and all(base is not None for base, _ in ranges):
        return 0  # The pushed commits change no files.
    if mode == "tests":
        return step_dev("test", "--affected", "--committed", *arguments).returncode
    document = json.loads(
        step_dev(
            "affected", "--json", *arguments, check=True, capture_output=True, text=True
        ).stdout
    )
    run, skipped = lint_checks(document, (ROOT / "packages/node_modules").is_dir())
    for check, reason in skipped:
        print(f"pre-push: skipped {check}: {reason}", file=sys.stderr)
    failed = [check for check in run if step_dev("test", check).returncode != 0]
    if failed:
        print(
            f"pre-push: {', '.join(failed)} failed; fix them or push with --no-verify",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
