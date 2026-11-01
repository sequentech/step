# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Git pre-push hook: the lint and format checks the pushed commits select.

Enable it once per clone with ``git config core.hooksPath scripts/dev/hooks``.
It runs the ``lint`` checks of ``scripts/dev/affected.toml`` that the pushed
changes select, the same commands as ``lint_prettify.yml``. ``STEP_PRE_PUSH``
chooses what runs: ``lint`` (default), ``tests`` (``step-dev test --affected``
instead: the fast tests and the lint checks) or ``off``. ``git push --no-verify``
skips it.
"""

import json
import os
import shutil
import subprocess
import sys
import tomllib
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
    """Selected lint check IDs to run, and (ID, reason) for those skipped: the
    command or a required tool is missing, or the Yarn workspace has no
    dependencies."""
    run, skipped = [], []
    for check in document["checks"]:
        if not check["selected"] or check["kind"] != "lint":
            continue
        tools = dict.fromkeys([check["command"].split()[0], *check.get("requires", [])])
        missing = [tool for tool in tools if not available(tool)]
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


def changed_files(
    ranges: list[tuple[str | None, str]], default_base: str
) -> list[str] | None:
    """Files that differ between each remote tip and the pushed one, so a
    forced push also counts what it drops, with both paths of a rename. A ref
    without a remote tip counts from its merge base with the model's default
    base; None when that is unknown too."""
    files = set()
    for base, tip in ranges:
        if base is None:
            if not known(default_base):
                return None
            base = git("merge-base", default_base, tip).strip()
        files.update(git("diff", "--name-only", "--no-renames", base, tip).splitlines())
    return sorted(files)


def default_base() -> str:
    with (ROOT / "scripts/dev/affected.toml").open("rb") as model:
        return tomllib.load(model)["default_base"]


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
    files = changed_files(ranges, default_base())
    if files == []:
        return 0  # The pushed commits change no files.
    # Without files, the selection falls back to HEAD and its default base.
    arguments = ["--files", *files] if files else []
    if mode == "tests":
        scope = arguments or ["--committed"]
        return step_dev("test", "--affected", *scope).returncode
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
