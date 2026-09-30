# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Drawing boards with the resident engine, offline."""

from __future__ import annotations

import re
import socket

import pytest

from conftest import GOLDEN_BOARD_FILES, golden, golden_id, kpi_board, map_board
from monitoring_renderer.engine import BoardRefused
from monitoring_renderer.geo import BUNDLED_URLS, ExternalData, inline_bundled_data
from monitoring_renderer.svg import clean_svg


def test_sockets_are_blocked_in_these_tests():
    with pytest.raises(Exception):
        socket.create_connection(("192.0.2.1", 80), timeout=0.1)


@pytest.mark.parametrize("path", GOLDEN_BOARD_FILES, ids=golden_id)
def test_every_golden_board_renders_to_a_clean_svg(engine, path):
    rendered = engine.render(golden(f"{path.parent.name}/{path.name}"), width=872)
    assert rendered.svg.startswith("<svg")
    assert 'width="872"' in rendered.svg
    assert "/static/fonts" not in rendered.svg
    assert clean_svg(rendered.svg) == rendered.svg
    assert all(problem.severity.value == "warning" for problem in rendered.warnings)
    # A theme pins its colours for every dashboard; a board gets only the
    # pins for values it draws, so the engine never skips one.
    assert not [p for p in rendered.warnings if p.engine_code == "WARN-CATEGORY-COLOR-PIN-UNSEEN"]



def test_there_are_golden_boards_from_both_presets():
    assert {path.parent.name for path in GOLDEN_BOARD_FILES} == {"campus", "comelec"}
    assert len(GOLDEN_BOARD_FILES) >= 40


def svg_size(svg: str) -> tuple[str, str]:
    return re.search(r'<svg[^>]*?width="([^"]*)" height="([^"]*)"', svg).groups()


def test_the_requested_width_and_height_are_drawn(engine):
    board = golden("campus/participation-by-faculty.json")
    natural = svg_size(engine.render(board, width=400).svg)
    assert natural[0] == "400"
    # Height is a minimum: the frame grows to it, and charts keep their own.
    assert int(natural[1]) < 900
    assert svg_size(engine.render(board, width=1200, height=900).svg) == ("1200", "900")


def page_background(svg: str) -> str:
    return re.search(r'data-dbt-page-background="([^"]*)"', svg).group(1)


def test_dark_draws_on_a_dark_page(engine):
    board = golden("comelec/turnout-by-group.json")
    light = engine.render(board, width=600, color_scheme="light").svg
    dark = engine.render(board, width=600, color_scheme="dark").svg
    assert page_background(light).upper() != page_background(dark).upper()
    assert page_background(dark).upper() == "#161616"


def test_the_board_the_caller_sent_is_left_as_it_was(engine):
    board = kpi_board()
    before = repr(board)
    engine.render(board, width=500, color_scheme="dark")
    assert repr(board) == before


@pytest.mark.parametrize("geo_source", ["world-countries", "world-50m"])
def test_world_maps_draw_from_the_bundled_geometry(engine, geo_source, monkeypatch):
    import vl_convert

    seen = []
    patched = vl_convert.vegalite_to_svg

    def spy(spec, *args, **kwargs):
        seen.append(repr(spec))
        return patched(spec, *args, **kwargs)

    monkeypatch.setattr(vl_convert, "vegalite_to_svg", spy)
    rendered = engine.render(map_board(geo_source), width=800)
    # The engine asked for a map on the internet, and vl-convert may fetch
    # nothing: every country drawn came from the bundle.
    assert any(url in spec for spec in seen for url in BUNDLED_URLS)
    assert len(re.findall(r"<path", rendered.svg)) > 150


def test_bundled_map_urls_become_inline_geometry():
    spec = {"layer": [{"data": {"url": next(iter(BUNDLED_URLS)), "format": {"type": "topojson"}}}]}
    inlined = inline_bundled_data(spec)
    assert "url" not in inlined["layer"][0]["data"]
    assert inlined["layer"][0]["data"]["values"]["type"] == "Topology"
    assert "url" in spec["layer"][0]["data"]  # the engine's own spec is left alone
    with pytest.raises(ExternalData):
        inline_bundled_data({"data": {"url": "file:///etc/passwd"}})


def test_the_engine_is_never_allowed_to_fetch():
    import vl_convert

    from monitoring_renderer.geo import install_offline_geo

    install_offline_geo()
    spec = {"data": {"url": "https://example.invalid/x.json"}, "mark": "point"}
    with pytest.raises(Exception):
        vl_convert.vegalite_to_svg(spec)
    with pytest.raises(Exception):
        vl_convert.vegalite_to_svg(spec, allowed_base_urls=None)
    with pytest.raises(Exception):
        vl_convert.svg_to_png("<svg/>")


def test_data_text_is_drawn_as_text(engine):
    board = kpi_board()
    board["charts"]["k"] = {"type": "table", "query": "data"}
    board["queries"]["data"] = {
        "columns": ["name"],
        "values": [['<script>alert(1)</script><a href="https://x">y</a>']],
    }
    rendered = engine.render(board, width=600)
    assert "<script" not in rendered.svg
    assert "&lt;script&gt;" in rendered.svg


def test_a_forbidden_board_is_refused_before_the_engine_sees_it(engine, monkeypatch):
    board = kpi_board()
    board["charts"]["k"]["link"] = "https://example.com"
    monkeypatch.setattr(engine, "_draw", lambda *a, **k: pytest.fail("drawn"))
    with pytest.raises(BoardRefused) as refused:
        engine.render(board, width=600)
    assert ("forbidden_key", "charts.k.link") in {
        (p.code.value, p.path) for p in refused.value.problems
    }


def test_a_board_the_engine_refuses_carries_its_codes(engine):
    board = kpi_board()
    board["charts"]["k"]["type"] = "barz"
    with pytest.raises(BoardRefused) as refused:
        engine.render(board, width=600)
    (problem, *_) = refused.value.problems
    assert problem.code.value == "chart_schema"
    assert problem.engine_code.startswith("ERR-")


def test_validate_reports_engine_errors_and_warnings(engine):
    assert engine.validate(golden("comelec/turnout-by-country.json")) == []
    warnings = engine.validate(golden("campus/polls.json"))
    assert {(p.severity.value, p.engine_code) for p in warnings} == {
        ("warning", "WARN-CATEGORY-COLOR-PIN-UNSEEN")
    }
    board = kpi_board()
    board["charts"]["k"]["type"] = "barz"
    (error, *_) = engine.validate(board)
    assert error.severity.value == "error" and error.engine_code.startswith("ERR-")


def test_validate_reports_the_policy_first(engine):
    board = kpi_board()
    board["charts"]["k"]["sql"] = "select 1"
    assert ("forbidden_key", "charts.k.sql") in {
        (p.code.value, p.path) for p in engine.validate(board)
    }


def test_duckdb_cannot_reach_files_or_the_network(engine):
    config = engine.duckdb_config
    assert config["enable_external_access"] is False
    assert config["autoinstall_known_extensions"] is False
    assert config["autoload_known_extensions"] is False
