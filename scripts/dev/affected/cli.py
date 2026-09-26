# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Command line: ``step-dev affected [--base REF] [--worktree | --files ...]``."""

from __future__ import annotations

import argparse
import json
import os
import sys
from collections.abc import Sequence
from pathlib import Path

from .changes import ChangeSet, GitError, Scope, collect, from_files
from .config import ConfigError
from .model import Model, Selection, load_model
from .report import graph_report, json_report, text_report

ROOT = Path(__file__).resolve().parents[3]


def caller_cwd() -> Path:
    """``step-dev`` runs from the repository root; paths are the caller's."""
    return Path(os.environ.get("STEP_DEV_CWD", os.getcwd()))


def add_change_options(parser: argparse.ArgumentParser, default_scope: Scope) -> None:
    """Options shared with ``step-dev test --affected``."""
    parser.add_argument(
        "--base",
        metavar="REF",
        help="compare with the merge base of HEAD and REF (default: the branch's "
        "upstream unless it is the same branch, else the model's default_base)",
    )
    scope = parser.add_mutually_exclusive_group()
    if default_scope is Scope.COMMITTED:
        scope.add_argument(
            "--worktree",
            dest="scope",
            action="store_const",
            const=Scope.WORKTREE,
            help="also count staged, unstaged and untracked files",
        )
    else:
        scope.add_argument(
            "--committed",
            dest="scope",
            action="store_const",
            const=Scope.COMMITTED,
            help="count only commits, not the working tree",
        )
    scope.add_argument(
        "--files",
        nargs="+",
        metavar="PATH",
        help="treat these paths as the changes instead of asking git",
    )
    scope.add_argument(
        "--files-from",
        metavar="FILE",
        help="read changed paths, one per line, from FILE ('-' for stdin)",
    )
    parser.set_defaults(scope=default_scope)


def listed_files(arguments: argparse.Namespace) -> list[str] | None:
    if arguments.files is not None:
        return list(arguments.files)
    if arguments.files_from is None:
        return None
    if arguments.files_from == "-":
        return sys.stdin.read().splitlines()
    source = Path(arguments.files_from)
    if not source.is_absolute():
        source = caller_cwd() / source
    return source.read_text(encoding="utf-8").splitlines()


def changes_for(model: Model, arguments: argparse.Namespace) -> ChangeSet:
    files = listed_files(arguments)
    if files is not None:
        return from_files(model.root, files, caller_cwd())
    return collect(
        model.root, arguments.base, model.config.default_base, arguments.scope
    )


def select(model: Model, arguments: argparse.Namespace) -> Selection:
    return model.select(changes_for(model, arguments))


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(
        prog="step-dev affected",
        description="Show which packages a change affects and which checks it "
        "selects, from the workspace manifests and scripts/dev/affected.toml. "
        "Unknown paths, a missing merge base, toolchain or selection-model "
        "changes select every check.",
    )
    add_change_options(root, Scope.COMMITTED)
    output = root.add_mutually_exclusive_group()
    output.add_argument(
        "--json", action="store_true", help="print the versioned JSON document"
    )
    output.add_argument(
        "--graph", action="store_true", help="print every unit and its edges"
    )
    return root


def main(argv: Sequence[str] | None = None, root: Path = ROOT) -> int:
    arguments = parser().parse_args(argv)
    try:
        model = load_model(root)
        if arguments.graph:
            print(graph_report(model))
            return 0
        selection = select(model, arguments)
    except (ConfigError, GitError, OSError, ValueError) as error:
        print(f"step-dev affected: {error}", file=sys.stderr)
        return 2
    if arguments.json:
        print(json.dumps(json_report(model, selection), indent=2))
    else:
        print(text_report(selection))
    return 0
