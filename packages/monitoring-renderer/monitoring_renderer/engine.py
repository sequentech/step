# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The resident dbt Charts engine.

One `ProjectSession` per process, opened on an empty, read-only project and
warmed by drawing one board, so a widget takes tens of milliseconds instead
of the seconds a CLI call costs. Boards are drawn in memory: nothing is
written, nothing is fetched.
"""

from __future__ import annotations

import copy
import tempfile
import threading
import time
from dataclasses import dataclass, field
from importlib.metadata import version
from pathlib import Path
from typing import Any

import yaml

from . import policy
from .geo import install_offline_geo
from .problems import Code, Problem, Severity, UNSAFE_OUTPUT, from_diagnostic, has_errors
from .svg import UnsafeSvg, clean_svg

DBT_CHARTS_VERSION = version("dbt-charts")

# dbt-charts 0.8.0 ships one dark theme; `color_scheme: dark` draws with it.
DARK_THEME = "neon"
HIDDEN = {"visible": False}

# A board of every golden board's shape, drawn once at start.
WARM_BOARD = {
    "charts": {
        "k": {"type": "kpi", "query": "total", "value": "voted", "label": "Voted"},
        "b": {"type": "bar", "query": "data", "x": "group", "y": "voted", "style": {"orientation": "horizontal"}},
        "t": {"type": "table", "query": "data", "style": {"pagination": {"enabled": False}}},
    },
    "rows": ["k", "b", "t"],
    "style": {"footer": HIDDEN, "timestamp": HIDDEN},
    "theme": "clarity",
    "queries": {
        "total": {"columns": ["voted"], "values": [[8]]},
        "data": {"columns": ["group", "voted"], "values": [["A", 3], ["B", 5]]},
    },
}

# DuckDB is not used for inline values, but the session has an adapter for
# it; it may read no file and load no extension.
DUCKDB_CONFIG = {
    "enable_external_access": False,
    "autoinstall_known_extensions": False,
    "autoload_known_extensions": False,
}


class BoardRefused(Exception):
    """The board was not drawn; `problems` say why."""

    def __init__(self, problems: list[Problem]):
        super().__init__("; ".join(f"{p.path}: {p.message}" for p in problems))
        self.problems = problems


@dataclass
class Rendered:
    svg: str
    warnings: list[Problem] = field(default_factory=list)
    render_ms: int = 0


def prepare(board: dict, *, width: int | None, height: int | None, color_scheme: str) -> dict:
    """The board as the engine draws it: `width` wide and at least `height`
    tall, with the engine's footer and freshness line off whatever the board
    says, and dbt Charts' dark theme for `dark`."""
    prepared = copy.deepcopy(board)
    if width is not None:
        prepared["width"] = width
    style = prepared.get("style")
    if not isinstance(style, dict):
        style = prepared["style"] = {}
    if height is not None:
        # A root board has no height of its own; its frame has a minimum.
        frame = style.get("frame")
        if not isinstance(frame, dict):
            frame = style["frame"] = {}
        frame["min_height"] = height
    style["footer"] = dict(HIDDEN)
    style["timestamp"] = dict(HIDDEN)
    if color_scheme == "dark":
        prepared["theme"] = DARK_THEME
    return prepared


def board_text(board: dict) -> str:
    # YAML, not JSON: the engine reads YAML 1.1, where JSON's `1e-05` is text.
    return yaml.safe_dump(board, allow_unicode=True, sort_keys=False)


class Engine:
    version = DBT_CHARTS_VERSION

    def __init__(self, project_dir: str | Path | None = None):
        from dbt_charts.agent_api.project_session import ProjectSession

        install_offline_geo()
        self.project_dir = Path(project_dir or tempfile.mkdtemp(prefix="renderer-project-"))
        self.duckdb_config = dict(DUCKDB_CONFIG)
        self._session = ProjectSession.open(
            self.project_dir,
            read_only=True,
            duckdb_config=self.duckdb_config,
            allow_external_access_in_readonly=False,
        )
        self._warm = threading.Event()

    @property
    def ready(self) -> bool:
        return self._warm.is_set()

    def warm(self) -> None:
        self.render(WARM_BOARD, width=600)
        self._warm.set()

    def render(
        self,
        board: Any,
        *,
        width: int | None = None,
        height: int | None = None,
        color_scheme: str = "light",
    ) -> Rendered:
        problems = policy.check_board(board)
        if has_errors(problems):
            raise BoardRefused(problems)
        started = time.perf_counter()
        prepared = prepare(board, width=width, height=height, color_scheme=color_scheme)
        result = self._draw(prepared)
        failures = [
            *result.validation_errors,
            *result.chart_errors,
            *([result.board_error] if result.board_error else []),
        ]
        errors = [problem for problem in map(from_diagnostic, failures) if problem.severity == Severity.ERROR]
        if result.status != "ok" or errors or not result.data:
            raise BoardRefused(errors or [from_diagnostic(f"The engine could not draw the board ({result.status}).")])
        try:
            svg = clean_svg(result.data)
        except UnsafeSvg as unsafe:
            raise BoardRefused(
                [
                    Problem(
                        severity=Severity.ERROR,
                        code=Code.CHART_SCHEMA,
                        path="",
                        message=f"The drawn chart failed the renderer's SVG check: {unsafe}.",
                        engine_code=UNSAFE_OUTPUT,
                    )
                ]
            ) from None
        warnings = [from_diagnostic(w) for w in result.warnings or []]
        return Rendered(
            svg=svg,
            warnings=[w for w in warnings if w.severity == Severity.WARNING],
            render_ms=int((time.perf_counter() - started) * 1000),
        )

    def validate(self, board: Any) -> list[Problem]:
        """The policy's problems, then the engine's: compile diagnostics, and
        what drawing the board says (errors and warnings)."""
        from dbt_charts.core.compile import compile as compile_board

        problems = policy.check_board(board)
        if has_errors(problems):
            return problems
        prepared = prepare(board, width=None, height=None, color_scheme="light")
        compiled = compile_board(board_text(prepared))
        found = [from_diagnostic(d) for d in [*compiled.errors, *compiled.warnings]]
        if not compiled.success:
            return found
        try:
            rendered = self.render(board)
        except BoardRefused as refused:
            return _unique(found + refused.problems)
        return _unique(found + rendered.warnings)

    def _draw(self, prepared: dict):
        from dbt_charts.core.project import InMemoryBoard

        return self._session.render_board(InMemoryBoard(board_text(prepared), path=None), format="svg")


def _unique(problems: list[Problem]) -> list[Problem]:
    seen = set()
    unique = []
    for problem in problems:
        key = (problem.severity, problem.code, problem.path, problem.message, problem.engine_code)
        if key not in seen:
            seen.add(key)
            unique.append(problem)
    return unique
