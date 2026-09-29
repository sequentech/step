# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The renderer's own walk over a board: the Rust policy again, as defence in
depth, plus what only a board has (its governed queries)."""

from __future__ import annotations

import copy

import pytest
import yaml

from conftest import GOLDEN_BOARD_FILES, MONITORING, golden, golden_id, kpi_board
from monitoring_renderer.policy import MAX_COLUMNS, MAX_ROWS, check_board
from monitoring_renderer.problems import Code, Severity


def codes_at(problems):
    return {(problem.code.value, problem.path) for problem in problems}


@pytest.mark.parametrize("path", GOLDEN_BOARD_FILES, ids=golden_id)
def test_every_golden_board_passes(path):
    board = golden(f"{path.parent.name}/{path.name}")
    assert check_board(board) == []


def refused_cases():
    cases = yaml.safe_load((MONITORING / "fixtures" / "refused.yaml").read_text())
    for case in cases:
        path = case["path"]
        if case["kind"] == "widget" and path.startswith("chart.") and path != "chart.queries":
            yield pytest.param(case, id=case["name"])
        elif case["kind"] == "theme" and path.startswith("style."):
            yield pytest.param(case, id=case["name"])


def board_for(case) -> tuple[dict, str]:
    """The board `build_board` would send for a refused document, and where
    the renderer should find the problem in it."""
    document = yaml.safe_load(case["yaml"])
    if case["kind"] == "widget":
        board = copy.deepcopy(document.get("chart") or {})
        names = ["data"] if "query" in document else list(document.get("queries") or {})
        board.pop("queries", None)
        board["queries"] = {name: {"columns": ["voted"], "values": [[1]]} for name in names}
        return board, case["path"].removeprefix("chart.")
    board = kpi_board()
    board["style"] = copy.deepcopy(document.get("style") or {})
    return board, case["path"]


@pytest.mark.parametrize("case", list(refused_cases()))
def test_every_shared_refused_case_is_refused_where_the_rust_policy_refuses_it(case):
    board, path = board_for(case)
    problems = check_board(board)
    assert (case["code"], path) in codes_at(problems), problems
    assert all(problem.severity == Severity.ERROR for problem in problems)


def test_the_shared_refused_list_still_has_its_cases():
    assert len(list(refused_cases())) >= 50


def test_prose_that_mentions_data_is_accepted():
    board = kpi_board()
    board["charts"]["k"]["label"] = "Data: votes < turnout, see http notes"
    assert check_board(board) == []


@pytest.mark.parametrize(
    "key", ["sql", "source", "variables", "extends", "tabs", "html_policy", "text", "cache", "width"]
)
def test_board_fields_the_platform_supplies_or_refuses(key):
    board = kpi_board()
    board[key] = "x"
    assert ("forbidden_key", key) in codes_at(check_board(board))


def test_the_engine_footer_and_timestamp_stay_hidden():
    for key in ("footer", "timestamp"):
        board = kpi_board()
        board["style"][key] = {"visible": True}
        assert ("forbidden_key", f"style.{key}") in codes_at(check_board(board))


def test_a_theme_the_engine_does_not_ship_is_refused():
    board = kpi_board(theme="../../etc/passwd")
    assert ("invalid_value", "theme") in codes_at(check_board(board))


def test_a_board_without_queries_is_refused():
    board = kpi_board()
    del board["queries"]
    assert ("invalid_value", "queries") in codes_at(check_board(board))


def test_only_inline_values_queries_are_accepted():
    for query in ({"sql": "select 1"}, {"url": "https://x"}, {"source": "f.csv"}, "select 1"):
        board = kpi_board()
        board["queries"]["data"] = query
        problems = check_board(board)
        assert any(p.path.startswith("queries.data") for p in problems), query


def test_queries_are_capped_in_rows_and_columns():
    board = kpi_board()
    board["queries"]["data"] = {"columns": ["voted"], "values": [[1]] * (MAX_ROWS + 1)}
    assert ("too_large", "queries.data.values") in codes_at(check_board(board))
    columns = [f"c{i}" for i in range(MAX_COLUMNS + 1)]
    board["queries"]["data"] = {"columns": columns, "values": [[1] * len(columns)]}
    assert ("too_large", "queries.data.columns") in codes_at(check_board(board))


def test_query_rows_are_rows_of_scalars_as_wide_as_the_columns():
    board = kpi_board()
    board["queries"]["data"] = {"columns": ["voted"], "values": [[1, 2]]}
    assert ("invalid_value", "queries.data.values[0]") in codes_at(check_board(board))
    board["queries"]["data"] = {"columns": ["voted"], "values": [[{"sql": "x"}]]}
    assert ("invalid_value", "queries.data.values[0][0]") in codes_at(check_board(board))


def test_query_and_column_names_are_plain_names():
    board = kpi_board()
    board["queries"] = {"da ta": {"columns": ["voted"], "values": []}}
    assert ("invalid_value", "queries.da ta") in codes_at(check_board(board))
    board = kpi_board()
    board["queries"]["data"]["columns"] = ['vo"ted']
    assert ("invalid_value", "queries.data.columns[0]") in codes_at(check_board(board))


def test_data_values_are_not_read_as_configuration():
    # Governed rows may hold any text a voter typed; the engine escapes it
    # and the SVG check backs that up. Refusing it would let a voter blank a
    # dashboard.
    board = kpi_board()
    board["queries"]["data"] = {"columns": ["voted"], "values": [["<script>{{x}}</script> https://x"]]}
    assert check_board(board) == []


def test_a_chart_reading_a_query_the_board_does_not_carry_is_refused():
    board = kpi_board()
    board["charts"]["k"]["query"] = "other"
    assert ("dangling_reference", "charts.k.query") in codes_at(check_board(board))


def test_nesting_deeper_than_configuration_is_refused():
    board = kpi_board()
    deep: dict = {}
    node = deep
    for _ in range(40):
        node["style"] = {}
        node = node["style"]
    board["charts"]["k"]["style"] = deep
    assert Code.TOO_LARGE in {p.code for p in check_board(board)}


def test_a_board_that_is_not_a_mapping_is_refused():
    assert ("invalid_value", "") in codes_at(check_board(["charts"]))
