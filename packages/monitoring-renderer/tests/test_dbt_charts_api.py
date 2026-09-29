# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The parts of dbt-charts 0.8.0 the renderer relies on.

An upgrade that changes any of these fails here first, before a board does.
"""

from __future__ import annotations

import inspect
import json
import tempfile
from importlib.resources import files

import pytest
import yaml

from conftest import MONITORING, kpi_board


def test_the_pinned_version_is_installed():
    import dbt_charts

    assert dbt_charts.__version__ == "0.8.0"


def test_a_resident_session_renders_an_in_memory_board_to_svg():
    from dbt_charts.agent_api.project_session import ProjectSession
    from dbt_charts.core.project import InMemoryBoard

    assert "path" in inspect.signature(InMemoryBoard.__init__).parameters
    session = ProjectSession.open(tempfile.mkdtemp(), read_only=True)
    board = kpi_board(width=400)
    result = session.render_board(
        InMemoryBoard(yaml.safe_dump(board), path=None), format="svg"
    )
    assert result.status == "ok"
    assert result.data.startswith("<svg")
    assert 'width="400"' in result.data
    assert result.validation_errors == []
    assert result.chart_errors == []
    assert result.board_error is None


def test_compile_reports_coded_diagnostics():
    from dbt_charts.core.compile import compile as compile_board

    board = kpi_board()
    board["charts"]["k"]["bogus"] = 1
    result = compile_board(yaml.safe_dump(board))
    assert not result.success
    (error,) = result.errors
    assert error.code == "ERR-EXTRA-FIELD"
    assert error.level == "error"
    assert error.path == "charts.k.kpi.bogus"
    assert error.message


def test_validate_yaml_keeps_only_messages():
    # Why /validate calls `compile` rather than `validate_yaml`: the latter
    # drops the engine codes the editor links to.
    from dbt_charts.core.validate import validate_yaml

    board = kpi_board()
    board["charts"]["k"]["bogus"] = 1
    result = validate_yaml(yaml.safe_dump(board))
    assert result["success"] is False
    assert all(isinstance(error, str) for error in result["errors"])


def test_vl_convert_can_refuse_every_external_url():
    import vl_convert

    spec = {
        "data": {"url": "https://example.invalid/world.json"},
        "mark": "point",
    }
    # The engine's own function, even if the renderer has wrapped it.
    convert = getattr(vl_convert.vegalite_to_svg, "__wrapped__", vl_convert.vegalite_to_svg)
    with pytest.raises(Exception, match="not allowed"):
        convert(spec, allowed_base_urls=[])


def test_the_engine_fetches_world_maps_from_the_urls_the_renderer_replaces():
    from monitoring_renderer.geo import BUNDLED_URLS

    defaults = yaml.safe_load(
        files("dbt_charts.core").joinpath("render/geo_defaults.yml").read_text()
    )["geo_sources"]
    for name in ("world-countries", "world-50m"):
        source = defaults[name]
        assert source["url"] in BUNDLED_URLS, name
        assert source["format"] == "topojson"
        assert source["feature"] == "countries"
        assert source["key_format"] == "numeric"


def test_the_built_in_themes_are_the_ones_the_renderer_accepts():
    from monitoring_renderer.policy import THEMES

    shipped = {
        path.name.removesuffix(".yaml")
        for path in files("dbt_charts.core").joinpath("defaults/themes").iterdir()
        if path.name.endswith(".yaml") and not path.name.startswith(("_", "diagnostics-"))
    }
    assert set(THEMES) == shipped


def schema_property_names() -> set[str]:
    root = files("dbt_charts").joinpath("data/schemas/yaml")
    manifest = json.loads(root.joinpath("manifest.json").read_text())
    released = [s for s in manifest["schemas"] if s["status"] == "RELEASED"]
    latest = released[-1]
    assert latest["version"] == "0.7.0"
    schema = json.loads(root.joinpath(latest["file"]).read_text())
    names: set[str] = set()

    def walk(node):
        if isinstance(node, dict):
            properties = node.get("properties")
            if isinstance(properties, dict):
                names.update(properties)
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(schema)
    return names


def listed(path) -> list[str]:
    return [
        line.split()[0]
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.startswith("#")
    ]


def test_the_shared_property_list_is_the_installed_schema():
    expected = listed(MONITORING / "fixtures" / "dbt_charts_properties.txt")
    assert sorted(schema_property_names()) == sorted(expected)


def test_every_schema_property_has_a_policy_rule():
    keys = listed(MONITORING / "dbt_charts_keys.txt")
    assert sorted(keys) == sorted(schema_property_names())


def test_the_renderer_uses_the_policy_key_list_verbatim():
    from monitoring_renderer.policy import KEYS_FILE

    shared = (MONITORING / "dbt_charts_keys.txt").read_text(encoding="utf-8")
    assert KEYS_FILE.read_text(encoding="utf-8") == shared
