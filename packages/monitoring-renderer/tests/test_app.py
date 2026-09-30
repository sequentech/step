# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The HTTP contract Harvest's adapter implements against."""

from __future__ import annotations

import json
import threading
import time

import pytest
from fastapi.testclient import TestClient

from conftest import TOKEN, golden, kpi_board, unseen_pin_board
from monitoring_renderer.app import MAX_BODY_BYTES, Settings, create_app
from monitoring_renderer.engine import Rendered

HEADERS = {"X-Renderer-Token": TOKEN}


def settings(**overrides) -> Settings:
    values = {"token": TOKEN, "concurrency": 2, "timeout_ms": 4000, "cache_bytes": 8 << 20}
    values.update(overrides)
    return Settings(**values)


@pytest.fixture(scope="module")
def client(engine):
    with TestClient(create_app(settings(), engine=engine)) as client:
        deadline = time.monotonic() + 30
        while client.get("/ready").status_code != 200 and time.monotonic() < deadline:
            time.sleep(0.05)
        yield client


def render_body(board=None, **overrides):
    body = {
        "board": board if board is not None else golden("comelec/turnout-by-group.json"),
        "width": 640,
        "height": None,
        "color_scheme": "light",
        "locale": "en-US",
    }
    body.update(overrides)
    return body


def test_health_and_ready_need_no_token(client):
    assert client.get("/health").json() == {"status": "ok"}
    ready = client.get("/ready")
    assert ready.status_code == 200
    assert ready.json()["status"] == "ready"
    assert ready.json()["dbt_charts_version"] == "0.8.0"


@pytest.mark.parametrize("headers", [{}, {"X-Renderer-Token": "wrong"}, {"X-Renderer-Token": ""}])
@pytest.mark.parametrize("route", ["/render", "/validate"])
def test_every_other_route_needs_the_token(client, headers, route):
    response = client.post(route, json=render_body(), headers=headers)
    assert response.status_code == 401
    assert response.json()["error"] == "UNAUTHORIZED"


def test_an_unknown_route_also_needs_the_token(client):
    assert client.get("/docs").status_code == 401
    assert client.get("/openapi.json").status_code == 401


def test_a_websocket_is_closed_before_any_route(client):
    from starlette.websockets import WebSocketDisconnect

    with pytest.raises(WebSocketDisconnect) as closed:
        with client.websocket_connect("/render", headers=HEADERS):
            pass
    assert closed.value.code == 1008


def test_the_renderer_refuses_to_start_without_a_token(monkeypatch):
    for value in (None, "", "   "):
        if value is None:
            monkeypatch.delenv("RENDERER_TOKEN", raising=False)
        else:
            monkeypatch.setenv("RENDERER_TOKEN", value)
        with pytest.raises(RuntimeError, match="RENDERER_TOKEN"):
            Settings.from_env()


def test_settings_come_from_the_environment(monkeypatch):
    monkeypatch.setenv("RENDERER_TOKEN", "t" * 32)
    monkeypatch.setenv("RENDERER_CONCURRENCY", "3")
    monkeypatch.setenv("RENDERER_TIMEOUT_MS", "2500")
    loaded = Settings.from_env()
    assert (loaded.token, loaded.concurrency, loaded.timeout_ms) == ("t" * 32, 3, 2500)
    monkeypatch.delenv("RENDERER_CONCURRENCY")
    monkeypatch.delenv("RENDERER_TIMEOUT_MS")
    assert (Settings.from_env().concurrency, Settings.from_env().timeout_ms) == (2, 4000)


def test_a_golden_board_renders(client):
    response = client.post("/render", json=render_body(), headers=HEADERS)
    assert response.status_code == 200, response.text
    body = response.json()
    assert set(body) == {"svg", "render_ms", "dbt_charts_version", "cache", "warnings"}
    assert body["svg"].startswith("<svg")
    assert 'width="640"' in body["svg"]
    assert body["dbt_charts_version"] == "0.8.0"
    assert isinstance(body["render_ms"], int) and body["render_ms"] >= 0
    assert response.headers["content-type"].startswith("application/json")
    assert response.headers["cache-control"] == "no-store"


def test_a_repeated_request_is_served_from_the_cache(client):
    body = render_body(width=720)
    first = client.post("/render", json=body, headers=HEADERS).json()
    second = client.post("/render", json=body, headers=HEADERS).json()
    assert (first["cache"], second["cache"]) == ("MISS", "HIT")
    assert first["svg"] == second["svg"]
    # Key order does not make a different request.
    reordered = json.loads(json.dumps(body, sort_keys=True))
    reordered["board"] = dict(reversed(list(reordered["board"].items())))
    assert client.post("/render", json=reordered, headers=HEADERS).json()["cache"] == "HIT"
    changed = client.post("/render", json=render_body(width=720, color_scheme="dark"), headers=HEADERS)
    assert changed.json()["cache"] == "MISS"


def test_render_warnings_are_returned_not_refused(client):
    response = client.post("/render", json=render_body(unseen_pin_board()), headers=HEADERS)
    assert response.status_code == 200
    assert [w["engine_code"] for w in response.json()["warnings"]] == ["WARN-CATEGORY-COLOR-PIN-UNSEEN"]
    assert response.json()["warnings"][0]["severity"] == "warning"


def test_a_forbidden_board_gets_its_problems(client):
    board = kpi_board()
    board["charts"]["k"]["link"] = "https://example.com"
    response = client.post("/render", json=render_body(board), headers=HEADERS)
    assert response.status_code == 422
    problems = response.json()["problems"]
    (problem,) = [p for p in problems if p["code"] == "forbidden_key"]
    assert problem == {
        "severity": "error",
        "code": "forbidden_key",
        "path": "charts.k.link",
        "message": problem["message"],
        "engine_code": None,
    }


@pytest.mark.parametrize(
    "overrides, path",
    [
        ({"width": 199}, "width"),
        ({"width": 2401}, "width"),
        ({"width": "640"}, "width"),
        ({"height": 10}, "height"),
        ({"color_scheme": "sepia"}, "color_scheme"),
        ({"locale": "en_US; drop"}, "locale"),
        ({"extra": 1}, "extra"),
        ({"board": "charts: {}"}, "board"),
    ],
)
def test_a_malformed_request_is_refused_with_problems(client, overrides, path):
    response = client.post("/render", json=render_body(**overrides), headers=HEADERS)
    assert response.status_code == 422
    problems = response.json()["problems"]
    assert any(p["path"] == path and p["code"] == "invalid_value" for p in problems), problems


def test_a_body_that_is_not_json_is_refused(client):
    response = client.post(
        "/render", content=b"{not json", headers={**HEADERS, "content-type": "application/json"}
    )
    assert response.status_code == 422
    assert response.json()["problems"][0]["code"] == "unreadable"


def test_duplicate_keys_are_refused(client):
    raw = b'{"board": {"rows": ["k"], "rows": ["k"]}, "width": 640}'
    response = client.post("/render", content=raw, headers={**HEADERS, "content-type": "application/json"})
    assert response.status_code == 422
    assert response.json()["problems"][0]["code"] == "duplicate_id"


def test_a_body_over_the_limit_is_refused_unread(client):
    board = kpi_board()
    board["queries"]["data"]["values"] = [["x" * 1000]] * 300
    raw = json.dumps(render_body(board)).encode()
    assert len(raw) > MAX_BODY_BYTES
    response = client.post("/render", content=raw, headers={**HEADERS, "content-type": "application/json"})
    assert response.status_code == 413
    assert response.json()["error"] == "TOO_LARGE"


def test_a_chunked_body_over_the_limit_is_refused(client):
    def chunks():
        yield b'{"board": {"x": "'
        for _ in range(300):
            yield b"x" * 1024
        yield b'"}}'

    response = client.post("/render", content=chunks(), headers={**HEADERS, "content-type": "application/json"})
    assert response.status_code == 413


def test_validate_returns_problems(client):
    ok = client.post("/validate", json={"board": golden("comelec/turnout-by-group.json")}, headers=HEADERS)
    assert ok.status_code == 200 and ok.json() == {"problems": []}
    board = kpi_board()
    board["charts"]["k"]["type"] = "barz"
    refused = client.post("/validate", json={"board": board}, headers=HEADERS)
    assert refused.status_code == 200
    (problem, *_) = refused.json()["problems"]
    assert problem["code"] == "chart_schema" and problem["engine_code"].startswith("ERR-")
    extra = client.post("/validate", json={"board": board, "width": 3}, headers=HEADERS)
    assert extra.status_code == 422


class SlowEngine:
    """Stands in for the engine: draws after `delay` seconds."""

    version = "0.8.0"

    def __init__(self, delay: float):
        self.delay = delay
        self.running = 0
        self.most = 0
        self.lock = threading.Lock()

    def warm(self):
        pass

    def render(self, board, *, width, height=None, color_scheme="light"):
        with self.lock:
            self.running += 1
            self.most = max(self.most, self.running)
        try:
            time.sleep(self.delay)
            return Rendered(svg=f'<svg width="{width}"/>', warnings=[], render_ms=int(self.delay * 1000))
        finally:
            with self.lock:
                self.running -= 1

    def validate(self, board):
        return []


def test_a_render_over_the_time_limit_is_a_504():
    slow = SlowEngine(delay=0.5)
    with TestClient(create_app(settings(timeout_ms=100), engine=slow)) as client:
        response = client.post("/render", json=render_body(kpi_board()), headers=HEADERS)
        assert response.status_code == 504
        assert response.json()["error"] == "RENDER_TIMEOUT"


def test_renders_wait_for_a_free_slot(engine):
    slow = SlowEngine(delay=0.2)
    with TestClient(create_app(settings(concurrency=1), engine=slow)) as client:
        threads = [
            threading.Thread(
                target=client.post,
                args=("/render",),
                kwargs={"json": render_body(kpi_board(), width=300 + i), "headers": HEADERS},
            )
            for i in range(3)
        ]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
    assert slow.most == 1
