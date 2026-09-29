# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Shared fixtures. Every test runs with sockets disabled (see pyproject.toml):
the renderer must draw everything, maps included, with no network."""

from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest

PACKAGE = Path(__file__).resolve().parent.parent
MONITORING = PACKAGE.parent / "sequent-core" / "src" / "monitoring"
BOARDS = MONITORING / "fixtures" / "boards"
GOLDEN_BOARD_FILES = sorted(BOARDS.glob("*/*.json"))
TOKEN = "test-renderer-token-0123456789"


def golden(name: str) -> dict:
    return json.loads((BOARDS / name).read_text(encoding="utf-8"))


def golden_id(path: Path) -> str:
    return f"{path.parent.name}/{path.name}"


def kpi_board(**extra) -> dict:
    """A small board of the shape `build_board` sends."""
    board = {
        "charts": {"k": {"type": "kpi", "query": "data", "value": "voted", "label": "Voted"}},
        "rows": ["k"],
        "style": {"footer": {"visible": False}, "timestamp": {"visible": False}},
        "theme": "clarity",
        "queries": {"data": {"columns": ["voted"], "values": [[42]]}},
    }
    board.update(copy.deepcopy(extra))
    return board


def map_board(geo_source: str) -> dict:
    return {
        "charts": {
            "m": {
                "type": "geoshape",
                "query": "data",
                "geo_source": geo_source,
                "lookup": "country",
                "value": "pct",
                "title": "Turnout by country",
            }
        },
        "rows": ["m"],
        "style": {"footer": {"visible": False}, "timestamp": {"visible": False}},
        "queries": {
            "data": {
                "columns": ["country", "pct"],
                "values": [[840, 0.31], [724, 0.52], [608, 0.44]],
            }
        },
    }


@pytest.fixture(scope="session")
def engine():
    from monitoring_renderer.engine import Engine

    engine = Engine()
    engine.warm()
    return engine
