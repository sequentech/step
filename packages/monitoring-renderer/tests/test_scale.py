# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Boards the size of a large event draw within the renderer's limits.

The boards are built by sequent-core's scale tests from a payload with 100
Posts, 150 countries and thirty days of hourly buckets
(`presets_scale_tests.rs`, `fixtures/scale`). Each is posted as Harvest posts
it, at the widest width Harvest asks for, and must be accepted by the body
limit and drawn within the render time limit.

Run with `-s` to see what each board measured:
`[scale] case=.. body_bytes=.. render_ms=.. svg_bytes=..`.
"""

from __future__ import annotations

import json
import time
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from conftest import MONITORING, TOKEN
from monitoring_renderer.app import MAX_BODY_BYTES, Settings, create_app
from monitoring_renderer.svg import clean_svg

SCALE = MONITORING / "fixtures" / "scale"
SCALE_BOARD_FILES = sorted(SCALE.glob("*.json"))
HEADERS = {"X-Renderer-Token": TOKEN}

# The widest width Harvest's width buckets ask for, and the tallest widget
# the presets ship.
WIDTH = 2400
HEIGHT = 560

# The default time limit. Measured at about a quarter of it for the largest
# boards; the limit itself is the contract, so a slow machine still passes.
TIMEOUT_MS = Settings(token=TOKEN).timeout_ms

# Rows each board carries, from the payload the Rust side builds.
EXPECTED_ROWS = {
    "comelec.turnout-by-post": 100,
    "comelec.status-by-post": 100,
    "comelec.turnout-by-country": 151,
    "comelec.turnout-by-group.status": 151,
    "comelec.disapproval-reasons.all": 41,
    "comelec.voting-activity.every-hour": 720,
    "comelec.login-outcomes.every-hour": 720,
}


@pytest.fixture(scope="module")
def client(engine):
    settings = Settings(token=TOKEN, concurrency=1, timeout_ms=TIMEOUT_MS, cache_bytes=8 << 20)
    with TestClient(create_app(settings, engine=engine)) as client:
        deadline = time.monotonic() + 30
        while client.get("/ready").status_code != 200 and time.monotonic() < deadline:
            time.sleep(0.05)
        yield client


def body_bytes(board: dict) -> bytes:
    body = {"board": board, "width": WIDTH, "height": HEIGHT, "color_scheme": "light", "locale": "en-US"}
    return json.dumps(body, separators=(",", ":"), ensure_ascii=False).encode()


def case_id(path: Path) -> str:
    return path.stem


def test_every_scale_case_has_its_board():
    assert {path.stem for path in SCALE_BOARD_FILES} == set(EXPECTED_ROWS)


@pytest.mark.parametrize("path", SCALE_BOARD_FILES, ids=case_id)
def test_a_large_event_s_board_is_accepted_and_drawn_in_time(client, path):
    board = json.loads(path.read_text(encoding="utf-8"))
    rows = max(len(query["values"]) for query in board["queries"].values())
    assert rows == EXPECTED_ROWS[path.stem]
    body = body_bytes(board)
    assert len(body) <= MAX_BODY_BYTES

    started = time.perf_counter()
    response = client.post("/render", content=body, headers={**HEADERS, "Content-Type": "application/json"})
    elapsed_ms = (time.perf_counter() - started) * 1000
    assert response.status_code == 200, response.text
    answer = response.json()
    svg = answer["svg"]
    print(
        f"[scale] case={path.stem} rows={rows} body_bytes={len(body)} "
        f"render_ms={answer['render_ms']} request_ms={elapsed_ms:.0f} svg_bytes={len(svg.encode())}"
    )
    assert answer["cache"] == "MISS"
    assert elapsed_ms < TIMEOUT_MS
    assert svg.startswith("<svg")
    assert f'width="{WIDTH}"' in svg
    assert clean_svg(svg) == svg
    assert all(warning["severity"] == "warning" for warning in answer["warnings"])
