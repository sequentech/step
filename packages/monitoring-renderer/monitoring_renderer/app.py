# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The renderer's HTTP API. README.md is its contract."""

from __future__ import annotations

import asyncio
import hmac
import json
import logging
import os
import time
from concurrent.futures import ThreadPoolExecutor
from contextlib import asynccontextmanager
from dataclasses import dataclass
from enum import Enum
from typing import Annotated, Any

from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel, ConfigDict, Field, ValidationError

from .cache import LruCache, request_key
from .engine import DBT_CHARTS_VERSION, BoardRefused, Engine, Rendered
from .problems import Code, Problem

log = logging.getLogger("monitoring_renderer")

MAX_BODY_BYTES = 256 * 1024
TOKEN_HEADER = "x-renderer-token"
OPEN_ROUTES = {("GET", "/health"), ("GET", "/ready")}
LOCALE = r"^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$"


@dataclass(frozen=True)
class Settings:
    token: str
    concurrency: int = 2
    timeout_ms: int = 4000
    cache_bytes: int = 64 << 20

    @classmethod
    def from_env(cls) -> Settings:
        token = os.environ.get("RENDERER_TOKEN", "").strip()
        if not token:
            raise RuntimeError("RENDERER_TOKEN is not set: the renderer does not start without one")
        return cls(
            token=token,
            concurrency=_positive("RENDERER_CONCURRENCY", 2),
            timeout_ms=_positive("RENDERER_TIMEOUT_MS", 4000),
            cache_bytes=_positive("RENDERER_CACHE_BYTES", 64 << 20),
        )


def _positive(name: str, default: int) -> int:
    raw = os.environ.get(name, "").strip()
    if not raw:
        return default
    value = int(raw)
    if value < 1:
        raise RuntimeError(f"{name} must be a positive integer")
    return value


class ColorScheme(str, Enum):
    LIGHT = "light"
    DARK = "dark"


class CacheOutcome(str, Enum):
    HIT = "HIT"
    MISS = "MISS"


class RenderRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")

    board: dict[str, Any]
    width: Annotated[int, Field(strict=True, ge=200, le=2400)]
    height: Annotated[int | None, Field(strict=True, ge=80, le=4000)] = None
    color_scheme: ColorScheme = ColorScheme.LIGHT
    locale: Annotated[str, Field(pattern=LOCALE, max_length=35)] = "en"


class ValidateRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")

    board: dict[str, Any]


class RequestProblem(Exception):
    def __init__(self, problems: list[Problem]):
        self.problems = problems


def _error(status: int, error: str, message: str) -> JSONResponse:
    return JSONResponse({"error": error, "message": message}, status_code=status)


def _problems(problems: list[Problem], status: int = 422) -> JSONResponse:
    return JSONResponse({"problems": [p.as_json() for p in problems]}, status_code=status)


def _refuse_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    seen: dict[str, Any] = {}
    for key, value in pairs:
        if key in seen:
            raise RequestProblem([Problem.error(Code.DUPLICATE_ID, key, f"'{key}' is given twice.")])
        seen[key] = value
    return seen


def parse_body(raw: bytes, model: type[BaseModel]) -> BaseModel:
    try:
        data = json.loads(raw, object_pairs_hook=_refuse_duplicates)
    except RequestProblem:
        raise
    except (ValueError, RecursionError) as error:
        raise RequestProblem([Problem.error(Code.UNREADABLE, "", f"The body is not JSON: {error}.")]) from None
    try:
        return model.model_validate(data)
    except ValidationError as invalid:
        raise RequestProblem(
            [
                Problem.error(Code.INVALID_VALUE, ".".join(str(part) for part in error["loc"]), error["msg"])
                for error in invalid.errors(include_url=False, include_input=False)
            ]
        ) from None


class GuardMiddleware:
    """Before any route: the token, then the body limit.

    Pure ASGI, so a refused request is answered before its body is read.
    """

    def __init__(self, app, token: str):
        self.app = app
        self.token = token.encode("utf-8")

    async def __call__(self, scope, receive, send):
        if scope["type"] == "lifespan":
            await self.app(scope, receive, send)
            return
        if scope["type"] != "http":
            # No websockets or anything else: closed before any route.
            await send({"type": "websocket.close", "code": 1008})
            return
        if (scope["method"], scope["path"]) in OPEN_ROUTES:
            await self.app(scope, receive, send)
            return
        headers = dict(scope.get("headers") or [])
        given = headers.get(TOKEN_HEADER.encode(), b"")
        if not hmac.compare_digest(given, self.token):
            await _error(401, "UNAUTHORIZED", "X-Renderer-Token is missing or wrong.")(scope, receive, send)
            return
        length = headers.get(b"content-length")
        if length is not None and (not length.isdigit() or int(length) > MAX_BODY_BYTES):
            await self._too_large(scope, receive, send)
            return
        body = bytearray()
        while True:
            message = await receive()
            if message["type"] == "http.disconnect":
                return
            body.extend(message.get("body", b""))
            if len(body) > MAX_BODY_BYTES:
                await self._too_large(scope, receive, send)
                return
            if not message.get("more_body", False):
                break
        replayed = False

        async def replay():
            nonlocal replayed
            if replayed:
                return await receive()
            replayed = True
            return {"type": "http.request", "body": bytes(body), "more_body": False}

        await self.app(scope, replay, send)

    @staticmethod
    async def _too_large(scope, receive, send):
        await _error(413, "TOO_LARGE", f"A request body is at most {MAX_BODY_BYTES} bytes.")(scope, receive, send)


class Renderer:
    """The engine behind a fixed number of slots and a time limit."""

    def __init__(self, settings: Settings, engine: Any):
        self.settings = settings
        self.engine = engine
        self.cache: LruCache[Rendered] = LruCache(settings.cache_bytes)
        self.executor = ThreadPoolExecutor(max_workers=settings.concurrency, thread_name_prefix="render")
        self.slots: asyncio.Semaphore | None = None
        self.ready = False
        self.failed = False

    async def start(self) -> None:
        self.slots = asyncio.Semaphore(self.settings.concurrency)
        loop = asyncio.get_running_loop()
        warming = loop.run_in_executor(self.executor, self.engine.warm)
        warming.add_done_callback(self._warmed)

    def _warmed(self, future) -> None:
        if future.cancelled() or future.exception() is not None:
            self.failed = True
            log.error("warm-up render failed: %r", None if future.cancelled() else future.exception())
        else:
            self.ready = True
            log.info("engine warm, dbt-charts %s", DBT_CHARTS_VERSION)

    async def run(self, work):
        """`work()` on a render thread, within the time limit; the slot stays
        taken until the thread is done, even after a timeout."""
        loop = asyncio.get_running_loop()
        deadline = loop.time() + self.settings.timeout_ms / 1000
        try:
            await asyncio.wait_for(self.slots.acquire(), timeout=self.settings.timeout_ms / 1000)
        except TimeoutError:
            raise RenderTimeout() from None
        try:
            future = loop.run_in_executor(self.executor, work)
        except BaseException:
            self.slots.release()
            raise
        future.add_done_callback(lambda _: self.slots.release())
        try:
            return await asyncio.wait_for(asyncio.shield(future), timeout=max(0.0, deadline - loop.time()))
        except TimeoutError:
            raise RenderTimeout() from None


class RenderTimeout(Exception):
    pass


def create_app(settings: Settings | None = None, engine: Any | None = None) -> FastAPI:
    settings = settings or Settings.from_env()
    if not settings.token.strip():
        raise RuntimeError("RENDERER_TOKEN is not set: the renderer does not start without one")

    @asynccontextmanager
    async def lifespan(app: FastAPI):
        renderer = Renderer(settings, engine if engine is not None else Engine())
        app.state.renderer = renderer
        await renderer.start()
        yield
        renderer.executor.shutdown(wait=False, cancel_futures=True)

    app = FastAPI(
        title="monitoring-renderer",
        docs_url=None,
        redoc_url=None,
        openapi_url=None,
        lifespan=lifespan,
    )
    app.add_middleware(GuardMiddleware, token=settings.token)

    @app.middleware("http")
    async def no_store(request: Request, call_next):
        response = await call_next(request)
        response.headers["Cache-Control"] = "no-store"
        response.headers["X-Content-Type-Options"] = "nosniff"
        return response

    @app.exception_handler(RequestProblem)
    async def request_problem(_request: Request, problem: RequestProblem):
        return _problems(problem.problems)

    @app.exception_handler(RenderTimeout)
    async def render_timeout(_request: Request, _timeout: RenderTimeout):
        return _error(504, "RENDER_TIMEOUT", f"The board did not draw within {settings.timeout_ms} ms.")

    @app.get("/health")
    async def health():
        return {"status": "ok"}

    @app.get("/ready")
    async def ready(request: Request):
        renderer: Renderer = request.app.state.renderer
        if renderer.ready:
            return {"status": "ready", "dbt_charts_version": DBT_CHARTS_VERSION}
        status = "failed" if renderer.failed else "starting"
        return JSONResponse({"status": status, "dbt_charts_version": DBT_CHARTS_VERSION}, status_code=503)

    @app.post("/render")
    async def render(request: Request):
        renderer: Renderer = request.app.state.renderer
        body = parse_body(await request.body(), RenderRequest)
        key = request_key(
            {
                "board": body.board,
                "width": body.width,
                "height": body.height,
                "color_scheme": body.color_scheme.value,
                "locale": body.locale,
                "dbt_charts_version": DBT_CHARTS_VERSION,
            }
        )
        cached = renderer.cache.get(key)
        outcome = CacheOutcome.HIT
        if cached is None:
            outcome = CacheOutcome.MISS
            started = time.perf_counter()
            try:
                cached = await renderer.run(
                    lambda: renderer.engine.render(
                        body.board,
                        width=body.width,
                        height=body.height,
                        color_scheme=body.color_scheme.value,
                    )
                )
            except BoardRefused as refused:
                return _problems(refused.problems)
            except RenderTimeout:
                raise
            except Exception:
                log.exception("render failed")
                return _error(500, "RENDER_FAILED", "The engine failed while drawing the board.")
            cached.render_ms = cached.render_ms or int((time.perf_counter() - started) * 1000)
            renderer.cache.put(key, cached, len(cached.svg))
        return {
            "svg": cached.svg,
            "render_ms": cached.render_ms,
            "dbt_charts_version": DBT_CHARTS_VERSION,
            "cache": outcome.value,
            "warnings": [warning.as_json() for warning in cached.warnings],
        }

    @app.post("/validate")
    async def validate(request: Request):
        renderer: Renderer = request.app.state.renderer
        body = parse_body(await request.body(), ValidateRequest)
        try:
            problems = await renderer.run(lambda: renderer.engine.validate(body.board))
        except RenderTimeout:
            raise
        except Exception:
            log.exception("validation failed")
            return _error(500, "VALIDATE_FAILED", "The engine failed while checking the board.")
        return {"problems": [problem.as_json() for problem in problems]}

    return app

