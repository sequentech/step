# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Concurrent viewers of a monitoring dashboard, against a running stack.

Each simulated viewer does what the Admin Portal's Dashboard tab does,
through Hasura: ``monitoringListDashboards``, ``monitoringGetDashboard``,
then ``monitoringRenderWidget`` for every widget the dashboard places, at
the widths, colour scheme, locale and scope of that viewer. Viewers are
spread over a few screen profiles, so the render cache sees realistic keys.

``--cycle reload`` (the default) repeats all of it every poll interval, as if
every viewer reloaded the page each poll: an upper bound, since every
request reaches Harvest. ``--cycle portal`` is the portal's steady state:
the list and the renders once, then ``monitoringGetDashboard`` every poll,
and the renders again only when the snapshot revision changes or a widget
failed.

A run measures an idle baseline window first, then each viewer count in
turn: a warm-up of one poll interval, then a window of ``--duration``
seconds. For each window it records the latency of each operation
(p50/p95/p99), errors, the renders Harvest asked the renderer for (its
``render`` spans in ``docker logs``), the CPU of the stack's containers
(``docker stats``), the load generator's own CPU and event-loop lag, and
``pg_stat_user_tables`` before and after, for the backend and Keycloak
databases (through ``docker exec … psql``). Source-table reads are
reported per minute and per snapshot pass, to compare with the baseline:
the design says the number of viewers never changes them.

Only reads: nothing is written to either database, and the viewers only
call read actions.
"""

from __future__ import annotations

import asyncio
import base64
import json
import multiprocessing
import os
import random
import re
import shlex
import subprocess
import threading
import time
from collections import Counter
from collections.abc import Callable, Mapping, Sequence
from concurrent.futures import ProcessPoolExecutor
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any

from .common import SampleTimer, start_run
from .environment import git_state
from .http_pool import HttpError, HttpPool
from .results import CacheState, SampleRole

SCENARIO = "monitoring-viewers"


class Cycle(Enum):
    """What a viewer asks for on each poll."""

    RELOAD = "reload"
    PORTAL = "portal"


class Operation(Enum):
    LIST = "list-dashboards"
    GET = "get-dashboard"
    RENDER = "render-widget"


# ---------------------------------------------------------------------------
# Viewers
# ---------------------------------------------------------------------------

# The portal's layout: a 12-column MUI grid with spacing 2 (16 px), cards
# with 16 px padding each side, and widths sent in 40 px steps, at least 360.
GRID_COLUMNS = 12
GUTTER_PX = 16
CARD_PADDING_PX = 32
WIDTH_BUCKET_PX = 40
MIN_RENDER_WIDTH_PX = 360


@dataclass(frozen=True)
class ScreenProfile:
    """A screen: the width the dashboard gets, and whether cells stack."""

    name: str
    container_width: int
    # Below the md breakpoint (900 px) every cell takes the full row.
    stacked: bool
    weight: float


PROFILES: tuple[ScreenProfile, ...] = (
    ScreenProfile("desktop-1920", 1630, False, 0.35),
    ScreenProfile("laptop-1440", 1150, False, 0.20),
    ScreenProfile("laptop-1366", 1080, False, 0.25),
    ScreenProfile("tablet-820", 780, True, 0.12),
    ScreenProfile("phone-390", 358, True, 0.08),
)


def widget_width(profile: ScreenProfile, grid_width: int) -> int:
    """The width the portal sends for a cell ``grid_width`` columns wide."""
    columns = GRID_COLUMNS if profile.stacked else max(1, min(grid_width, GRID_COLUMNS))
    body = (
        profile.container_width * columns / GRID_COLUMNS - GUTTER_PX - CARD_PADDING_PX
    )
    steps = int(max(0.0, body) // WIDTH_BUCKET_PX)
    return max(MIN_RENDER_WIDTH_PX, steps * WIDTH_BUCKET_PX)


def viewer_counts(text: str) -> list[int]:
    counts = [int(part) for part in text.split(",") if part.strip()]
    if not counts or any(count < 1 for count in counts):
        raise ValueError(f"viewer counts must be positive integers: {text!r}")
    return counts


@dataclass(frozen=True)
class ViewerSetup:
    index: int
    profile: ScreenProfile
    color_scheme: str
    locale: str
    # A Post the viewer narrowed the dashboard to; None for the whole event.
    post: str | None


def _weighted(rng: random.Random, weights: Mapping[Any, float]) -> Any:
    choices = list(weights)
    return rng.choices(choices, weights=[weights[choice] for choice in choices])[0]


def viewer_setups(
    count: int,
    *,
    seed: int,
    dark_share: float,
    locales: Mapping[str, float],
    post_share: float,
    posts: Sequence[str],
    profiles: Sequence[ScreenProfile] = PROFILES,
) -> list[ViewerSetup]:
    """``count`` viewers, the same for the same seed."""
    rng = random.Random(seed)
    setups = []
    for index in range(count):
        profile = _weighted(rng, {profile: profile.weight for profile in profiles})
        dark = rng.random() < dark_share
        locale = _weighted(rng, locales)
        on_post = rng.random() < post_share
        setups.append(
            ViewerSetup(
                index=index,
                profile=profile,
                color_scheme="DARK" if dark else "LIGHT",
                locale=locale,
                post=rng.choice(list(posts)) if on_post and posts else None,
            )
        )
    return setups


def selector_values(
    widget: Mapping[str, Any],
    dashboard_values: Mapping[str, str],
    event_days: Sequence[str],
) -> dict[str, str]:
    """The selector values the portal sends for a widget nobody has touched:
    the dashboard's value if it is an option, else the default, else the first
    option (the latest day for day selectors); hidden selectors send nothing.
    """
    states: dict[str, str | None] = {}
    values: dict[str, str] = {}
    for name, selector in (widget.get("selectors") or {}).items():
        condition = selector.get("when")
        if condition and states.get(condition.get("selector")) not in condition.get(
            "in", []
        ):
            states[name] = None
            continue
        if selector.get("options_from"):
            options = list(event_days)
        else:
            options = list((selector.get("options") or {}).keys())
        dashboard = dashboard_values.get(name)
        if dashboard in options:
            chosen = dashboard
        elif selector.get("options_from"):
            chosen = options[-1] if options else None
        elif selector.get("default") in options:
            chosen = selector["default"]
        else:
            chosen = options[0] if options else None
        states[name] = chosen
        if chosen is not None:
            values[name] = chosen
    return values


def render_requests(
    election_event_id: str,
    dashboard: Mapping[str, Any],
    setup: ViewerSetup,
    election_id: str | None = None,
) -> list[dict[str, Any]]:
    """The ``monitoringRenderWidget`` variables of every widget a viewer's
    portal draws, from a ``monitoringGetDashboard`` answer."""
    widgets = dashboard.get("widgets") or {}
    sources = dashboard.get("sources") or {}
    snapshot = dashboard.get("snapshot") or {}
    days = dashboard.get("event_days") or []
    scope = {"post": setup.post} if setup.post else {}
    requests = []
    for item in dashboard["dashboard"].get("layout", []):
        entry = widgets.get(item["widget"])
        if entry is None:
            continue
        widget = entry["definition"]
        source = sources.get(widget.get("source"), {})
        if source.get("producer") == "NOT_CONNECTED":
            continue
        requests.append(
            {
                "electionEventId": election_event_id,
                "electionId": election_id,
                "dashboardId": dashboard["dashboard"]["id"],
                "widgetId": item["widget"],
                "scope": dict(scope),
                "selectorValues": selector_values(
                    widget, item.get("values") or {}, days
                ),
                "snapshotRevision": snapshot.get("revision"),
                "width": widget_width(setup.profile, int(item.get("width", 12))),
                "colorScheme": setup.color_scheme,
                "locale": setup.locale,
            }
        )
    return requests


# States a viewer sees as a broken widget; pending and not-connected widgets
# say what they are waiting for.
ERROR_STATES = frozenset({"RENDER_FAILED", "INVALID"})


def render_error(answer: Mapping[str, Any]) -> str | None:
    state = answer.get("state")
    if state not in ERROR_STATES:
        return None
    reason = answer.get("reason")
    return f"{state}:{reason}" if reason else str(state)


def percentile(values: Sequence[float], q: float) -> float | None:
    """Nearest-rank percentile."""
    if not values:
        return None
    ordered = sorted(values)
    rank = max(1, -(-len(ordered) * q // 100))
    return ordered[int(rank) - 1]


def latency_summary(seconds: Sequence[float]) -> dict[str, Any]:
    summary: dict[str, Any] = {"n": len(seconds)}
    if not seconds:
        return summary
    for q in (50, 95, 99):
        summary[f"p{q}_ms"] = round(percentile(seconds, q) * 1000, 1)
    summary["max_ms"] = round(max(seconds) * 1000, 1)
    summary["mean_ms"] = round(sum(seconds) / len(seconds) * 1000, 1)
    return summary


def token_expiry(token: str) -> int | None:
    """The ``exp`` claim of a JWT, or None when it has none."""
    try:
        payload = token.split(".")[1]
        payload += "=" * (-len(payload) % 4)
        return int(json.loads(base64.urlsafe_b64decode(payload))["exp"])
    except (IndexError, KeyError, TypeError, ValueError):
        return None


LIST_QUERY = """
query MonitoringListDashboards($electionEventId: uuid!, $electionId: uuid) {
  monitoringListDashboards(
    election_event_id: $electionEventId
    election_id: $electionId
  ) {
    mode
    dashboards { id title requirements widget_count }
    snapshot { revision as_of checked_at }
  }
}"""

GET_QUERY = """
query MonitoringGetDashboard(
  $electionEventId: uuid!
  $electionId: uuid
  $dashboardId: String!
) {
  monitoringGetDashboard(
    election_event_id: $electionEventId
    election_id: $electionId
    dashboard_id: $dashboardId
  ) {
    dashboard dashboard_revision widgets theme { id revision }
    settings settings_revision
    scope_options {
      regions { key label } posts { key label region } countries { key label }
    }
    restricted pinned_post sources snapshot { revision as_of checked_at }
    event_days
  }
}"""

RENDER_QUERY = """
query MonitoringRenderWidget(
  $electionEventId: uuid!, $electionId: uuid, $dashboardId: String!,
  $widgetId: String!, $scope: jsonb!, $selectorValues: jsonb!,
  $snapshotRevision: Int, $width: Int!, $colorScheme: String!,
  $locale: String!, $draft: jsonb
) {
  monitoringRenderWidget(
    election_event_id: $electionEventId, election_id: $electionId,
    dashboard_id: $dashboardId, widget_id: $widgetId, scope: $scope,
    selector_values: $selectorValues, snapshot_revision: $snapshotRevision,
    width: $width, color_scheme: $colorScheme, locale: $locale, draft: $draft
  ) {
    state reason svg table tables { query table } notices
    diagnostics { severity code path message } ignored_selectors render_ms
    snapshot_revision as_of
  }
}"""


# ---------------------------------------------------------------------------
# Load generation (one asyncio loop per worker process)
# ---------------------------------------------------------------------------


@dataclass
class WorkerPlan:
    graphql_url: str
    token: str
    token_command: str | None
    election_event_id: str
    election_id: str | None
    dashboard_id: str
    viewers: list[ViewerSetup]
    total_viewers: int
    cycle: Cycle
    poll_interval: float
    start_at: float
    measure_from: float
    measure_until: float
    connections: int
    timeout: float


@dataclass
class WorkerReport:
    latencies: dict[str, list[float]] = field(default_factory=dict)
    errors: Counter = field(default_factory=Counter)
    states: Counter = field(default_factory=Counter)
    requests: int = 0
    render_keys: set = field(default_factory=set)
    snapshot_revisions: set = field(default_factory=set)
    cpu_seconds: float = 0.0
    lag_ms: list[float] = field(default_factory=list)
    connections_opened: int = 0
    token_refreshes: int = 0


class Token:
    """A bearer token, fetched again by ``command`` shortly before it expires."""

    MARGIN_SECONDS = 60

    def __init__(self, token: str, command: str | None) -> None:
        self.value = token
        self.command = command
        self.refreshes = 0
        self._lock = asyncio.Lock()

    def _stale(self) -> bool:
        expiry = token_expiry(self.value)
        return expiry is not None and expiry - time.time() < self.MARGIN_SECONDS

    async def get(self) -> str:
        if self.command and self._stale():
            async with self._lock:
                if self._stale():
                    self.value = await asyncio.to_thread(fetch_token, self.command)
                    self.refreshes += 1
        return self.value


def fetch_token(command: str) -> str:
    completed = subprocess.run(
        command, shell=True, capture_output=True, text=True, timeout=120, check=False
    )
    token = (
        completed.stdout.strip().splitlines()[-1] if completed.stdout.strip() else ""
    )
    if completed.returncode != 0 or not token:
        raise ValueError(
            f"token command failed ({completed.returncode}): {completed.stderr[-500:]}"
        )
    return token


class Viewers:
    def __init__(self, plan: WorkerPlan) -> None:
        self.plan = plan
        self.report = WorkerReport(latencies={op.value: [] for op in Operation})
        self.pool = HttpPool(plan.graphql_url, plan.connections, timeout=plan.timeout)
        self.token = Token(plan.token, plan.token_command)

    def measuring(self, started: float) -> bool:
        return self.plan.measure_from <= started < self.plan.measure_until

    async def call(
        self, operation: Operation, query: str, variables: dict[str, Any]
    ) -> dict[str, Any] | None:
        headers = {"Authorization": f"Bearer {await self.token.get()}"}
        started = time.time()
        clock = time.perf_counter()
        error = None
        data = None
        try:
            status, body = await self.pool.post_json(
                {"query": query, "variables": variables}, headers
            )
            if status != 200:
                error = f"http:{status}"
            elif body.get("errors"):
                message = str(body["errors"][0].get("message", ""))[:80]
                error = f"graphql:{message}"
            else:
                data = body["data"]
        except HttpError as failure:
            error = f"transport:{str(failure)[:80]}"
        elapsed = time.perf_counter() - clock
        if self.measuring(started):
            self.report.requests += 1
            self.report.latencies[operation.value].append(elapsed)
            if error:
                self.report.errors[f"{operation.value} {error}"] += 1
        return data

    async def render(self, variables: dict[str, Any]) -> bool:
        """Draws one widget; whether the viewer sees it drawn or waiting."""
        started = time.time()
        data = await self.call(
            Operation.RENDER, RENDER_QUERY, {**variables, "draft": None}
        )
        if data is None:
            return False
        answer = data["monitoringRenderWidget"]
        failure = render_error(answer)
        if self.measuring(started):
            self.report.states[answer.get("state")] += 1
            key = json.dumps(
                {name: variables[name] for name in sorted(variables)}, sort_keys=True
            )
            self.report.render_keys.add(key)
            if answer.get("snapshot_revision") is not None:
                self.report.snapshot_revisions.add(answer["snapshot_revision"])
            if failure:
                self.report.errors[f"{Operation.RENDER.value} {failure}"] += 1
        return failure is None

    async def viewer(self, setup: ViewerSetup) -> None:
        plan = self.plan
        # Viewers arrive spread over one poll interval, so the load is steady.
        offset = plan.poll_interval * setup.index / max(1, plan.total_viewers)
        next_cycle = plan.start_at + offset
        variables = {
            "electionEventId": plan.election_event_id,
            "electionId": plan.election_id,
        }
        drawn_revision: Any = object()
        first = True
        while True:
            now = time.time()
            if next_cycle >= plan.measure_until:
                return
            if next_cycle > now:
                await asyncio.sleep(next_cycle - now)
            next_cycle += plan.poll_interval
            if first or plan.cycle is Cycle.RELOAD:
                await self.call(Operation.LIST, LIST_QUERY, variables)
            data = await self.call(
                Operation.GET,
                GET_QUERY,
                {**variables, "dashboardId": plan.dashboard_id},
            )
            first = False
            if data is None:
                continue
            dashboard = data["monitoringGetDashboard"]
            revision = (dashboard.get("snapshot") or {}).get("revision")
            if plan.cycle is Cycle.PORTAL and revision == drawn_revision:
                continue
            requests = render_requests(
                plan.election_event_id, dashboard, setup, plan.election_id
            )
            drawn = await asyncio.gather(
                *(self.render(request) for request in requests)
            )
            # A widget that failed asks again on the next poll.
            drawn_revision = revision if all(drawn) else object()

    async def lag(self) -> None:
        """Event-loop lag: how late a 100 ms timer fires."""
        while time.time() < self.plan.measure_until:
            expected = time.perf_counter() + 0.1
            await asyncio.sleep(0.1)
            late = (time.perf_counter() - expected) * 1000
            if self.measuring(time.time()):
                self.report.lag_ms.append(late)

    async def run(self) -> WorkerReport:
        cpu_from: float | None = None

        async def cpu_marks() -> None:
            nonlocal cpu_from
            await asyncio.sleep(max(0.0, self.plan.measure_from - time.time()))
            cpu_from = time.process_time()

        marks = asyncio.create_task(cpu_marks())
        await asyncio.gather(
            self.lag(), *(self.viewer(setup) for setup in self.plan.viewers)
        )
        await marks
        self.report.cpu_seconds = time.process_time() - (cpu_from or 0.0)
        self.report.connections_opened = self.pool.opened
        self.report.token_refreshes = self.token.refreshes
        await self.pool.close()
        return self.report


def run_worker(plan: WorkerPlan) -> WorkerReport:
    return asyncio.run(Viewers(plan).run())


def merge_reports(reports: Sequence[WorkerReport]) -> WorkerReport:
    merged = WorkerReport(latencies={op.value: [] for op in Operation})
    for report in reports:
        for name, values in report.latencies.items():
            merged.latencies.setdefault(name, []).extend(values)
        merged.errors.update(report.errors)
        merged.states.update(report.states)
        merged.requests += report.requests
        merged.render_keys |= report.render_keys
        merged.snapshot_revisions |= report.snapshot_revisions
        merged.cpu_seconds += report.cpu_seconds
        merged.lag_ms.extend(report.lag_ms)
        merged.connections_opened += report.connections_opened
        merged.token_refreshes += report.token_refreshes
    return merged


# ---------------------------------------------------------------------------
# What the stack did: table statistics, renderer calls, container CPU
# ---------------------------------------------------------------------------

STAT_COLUMNS = ("seq_scan", "seq_tup_read", "idx_scan", "idx_tup_fetch", "n_tup_ins")

STAT_QUERY = (
    "SELECT schemaname, relname, seq_scan, seq_tup_read, idx_scan, idx_tup_fetch, "
    "n_tup_ins FROM pg_stat_user_tables WHERE schemaname = '{schema}' ORDER BY 1, 2"
)

# The tables the snapshot job counts from; Harvest must not read them.
BACKEND_SOURCES = (
    "sequent_backend.cast_vote",
    "sequent_backend.applications",
    "sequent_backend.monitoring_voter",
    "sequent_backend.monitoring_login_counter",
    "sequent_backend.tally_session",
)
KEYCLOAK_SOURCES = (
    "public.user_entity",
    "public.user_attribute",
    "public.user_group_membership",
    "public.credential",
)
# One row per snapshot pass: its inserts count the passes.
PASS_TABLE = "sequent_backend.monitoring_snapshot_run"


def parse_table_stats(text: str) -> dict[str, dict[str, int]]:
    """``psql -At -F '\\t'`` rows of :data:`STAT_QUERY`, by ``schema.table``."""
    stats = {}
    for line in text.splitlines():
        cells = line.split("\t")
        if len(cells) != 2 + len(STAT_COLUMNS):
            continue
        schema, table, *counters = cells
        stats[f"{schema}.{table}"] = {
            name: int(value) if value.strip() else 0
            for name, value in zip(STAT_COLUMNS, counters, strict=True)
        }
    return stats


def read_deltas(
    before: Mapping[str, Mapping[str, int]], after: Mapping[str, Mapping[str, int]]
) -> dict[str, dict[str, int]]:
    """Per table: scans (sequential and index), tuples read, and inserts."""
    zero = dict.fromkeys(STAT_COLUMNS, 0)
    deltas = {}
    for table, now in after.items():
        then = before.get(table, zero)
        change = {name: now[name] - then.get(name, 0) for name in STAT_COLUMNS}
        deltas[table] = {
            "scans": change["seq_scan"] + change["idx_scan"],
            "tuples_read": change["seq_tup_read"] + change["idx_tup_fetch"],
            "seq_scan": change["seq_scan"],
            "idx_scan": change["idx_scan"],
            "n_tup_ins": change["n_tup_ins"],
        }
    return deltas


# `render` is HttpMonitoringRenderer::render's span: one per renderer call.
RENDER_SPAN = re.compile(r"┐render width=\d+")


RENDER_SPAN_BYTES = "┐render width=".encode()


def is_render_span(line: bytes) -> bool:
    return (
        RENDER_SPAN_BYTES in line
        and RENDER_SPAN.search(line.decode("utf-8", "replace")) is not None
    )


def count_renderer_calls(log: str) -> int:
    return sum(1 for line in log.encode().splitlines() if is_render_span(line))


def parse_docker_stats(text: str) -> dict[str, dict[str, Any]]:
    """``docker stats --format '{{.Name}}\\t{{.CPUPerc}}\\t{{.MemUsage}}'``."""
    stats = {}
    for line in text.splitlines():
        cells = line.split("\t")
        if len(cells) < 3:
            continue
        name, cpu, memory = cells[0].strip(), cells[1].strip(), cells[2]
        try:
            percent = float(cpu.rstrip("%"))
        except ValueError:
            continue
        stats[name] = {"cpu": percent, "memory": memory.split("/")[0].strip()}
    return stats


@dataclass
class Stack:
    """Where the running stack's pieces are."""

    backend_container: str
    backend_database: str
    keycloak_container: str
    keycloak_database: str
    harvest_container: str
    containers: list[str]

    def table_stats(self) -> dict[str, dict[str, int]]:
        stats = {}
        for container, database, schema in (
            (self.backend_container, self.backend_database, "sequent_backend"),
            (self.keycloak_container, self.keycloak_database, "public"),
        ):
            output = subprocess.run(
                [
                    "docker",
                    "exec",
                    container,
                    "psql",
                    "-U",
                    "postgres",
                    "-d",
                    database,
                    "-At",
                    "-F",
                    "\t",
                    "-c",
                    STAT_QUERY.format(schema=schema),
                ],
                capture_output=True,
                text=True,
                check=True,
                timeout=60,
            ).stdout
            prefix = "keycloak:" if container == self.keycloak_container else ""
            stats.update(
                {
                    prefix + name: value
                    for name, value in parse_table_stats(output).items()
                }
            )
        return stats

    def renderer_calls(self, since: float, until: float) -> int:
        process = subprocess.Popen(
            [
                "docker",
                "logs",
                "--since",
                f"{since:.3f}",
                "--until",
                f"{until:.3f}",
                self.harvest_container,
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        calls = 0
        assert process.stdout is not None
        for line in process.stdout:
            if is_render_span(line):
                calls += 1
        process.wait()
        return calls

    def docker_stats(self) -> dict[str, dict[str, Any]]:
        output = subprocess.run(
            [
                "docker",
                "stats",
                "--no-stream",
                "--format",
                "{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}",
                *self.containers,
            ],
            capture_output=True,
            text=True,
            check=False,
            timeout=60,
        ).stdout
        return parse_docker_stats(output)


class StatsSampler(threading.Thread):
    """``docker stats`` every ``interval`` seconds until stopped."""

    def __init__(self, stack: Stack, interval: float) -> None:
        super().__init__(daemon=True)
        self.stack = stack
        self.interval = interval
        self.samples: list[dict[str, dict[str, Any]]] = []
        self._stop = threading.Event()

    def run(self) -> None:
        while not self._stop.is_set():
            self.samples.append(self.stack.docker_stats())
            self._stop.wait(self.interval)

    def stop(self) -> dict[str, dict[str, float]]:
        self._stop.set()
        self.join()
        cpu: dict[str, list[float]] = {}
        for sample in self.samples:
            for name, values in sample.items():
                cpu.setdefault(name, []).append(values["cpu"])
        return {
            name: {
                "mean": round(sum(values) / len(values), 1),
                "max": round(max(values), 1),
                "samples": len(values),
            }
            for name, values in cpu.items()
        }


def source_reads(deltas: Mapping[str, Mapping[str, int]]) -> dict[str, dict[str, int]]:
    tables = [*BACKEND_SOURCES, *(f"keycloak:{name}" for name in KEYCLOAK_SOURCES)]
    return {table: dict(deltas[table]) for table in tables if table in deltas}


def harvest_reads(deltas: Mapping[str, Mapping[str, int]]) -> dict[str, dict[str, int]]:
    """Reads of the tables Harvest's read routes use (they grow with viewers)."""
    return {
        table: dict(values)
        for table, values in deltas.items()
        if table.startswith("sequent_backend.monitoring_snapshot")
        or table.startswith("sequent_backend.monitoring_config")
        or table
        in (
            "sequent_backend.monitoring_event",
            "sequent_backend.monitoring_election_set",
            "sequent_backend.election",
        )
    }


# ---------------------------------------------------------------------------
# A run
# ---------------------------------------------------------------------------


@dataclass
class ViewersOptions:
    label: str
    graphql_url: str
    tenant_id: str
    election_event_id: str
    election_id: str | None
    dashboard_id: str
    token_command: str
    viewers: list[int]
    cycle: Cycle
    poll_interval: float
    warmup: float
    duration: float
    baseline: bool
    processes: int
    connections: int
    timeout: float
    seed: int
    dark_share: float
    locales: dict[str, float]
    post_share: float
    stats_interval: float
    settle: float
    stack: Stack
    output_dir: Path
    checkout: Path | None


def probe_dashboard(options: ViewersOptions, token: str) -> dict[str, Any]:
    async def probe() -> dict[str, Any]:
        pool = HttpPool(options.graphql_url, 1, timeout=options.timeout)
        try:
            status, body = await pool.post_json(
                {
                    "query": GET_QUERY,
                    "variables": {
                        "electionEventId": options.election_event_id,
                        "electionId": options.election_id,
                        "dashboardId": options.dashboard_id,
                    },
                },
                {"Authorization": f"Bearer {token}"},
            )
        finally:
            await pool.close()
        if status != 200 or body.get("errors"):
            raise ValueError(f"monitoringGetDashboard failed: {status} {body}")
        return body["data"]["monitoringGetDashboard"]

    return asyncio.run(probe())


def level_summary(
    viewers: int,
    report: WorkerReport,
    deltas: Mapping[str, Mapping[str, int]],
    renderer_calls: int,
    cpu: Mapping[str, Any],
    window: float,
    workers: int,
) -> dict[str, Any]:
    sources = source_reads(deltas)
    passes = deltas.get(PASS_TABLE, {}).get("n_tup_ins", 0)
    total_scans = sum(values["scans"] for values in sources.values())
    total_tuples = sum(values["tuples_read"] for values in sources.values())
    minutes = window / 60
    lag = sorted(report.lag_ms)
    return {
        "viewers": viewers,
        "window_seconds": round(window, 1),
        "operations": {
            name: latency_summary(values) for name, values in report.latencies.items()
        },
        "requests": report.requests,
        "requests_per_second": round(report.requests / window, 1),
        "errors": sum(report.errors.values()),
        "error_kinds": dict(report.errors.most_common(20)),
        "render_states": dict(report.states),
        "renderer_calls": renderer_calls,
        "distinct_render_requests": len(report.render_keys),
        "snapshot_revisions": sorted(report.snapshot_revisions),
        "passes": passes,
        "source_reads": sources,
        "source_scans": total_scans,
        "source_tuples_read": total_tuples,
        "source_reads_per_minute": round(total_scans / minutes, 1),
        "source_reads_per_pass": round(total_scans / passes, 1) if passes else None,
        "harvest_table_reads": harvest_reads(deltas),
        "cpu": dict(cpu),
        "generator": {
            "workers": workers,
            "cpu_cores": round(report.cpu_seconds / window, 2),
            "loop_lag_p99_ms": round(percentile(lag, 99), 1) if lag else None,
            "loop_lag_max_ms": round(lag[-1], 1) if lag else None,
            "connections_opened": report.connections_opened,
            "token_refreshes": report.token_refreshes,
        },
    }


def markdown_table(
    levels: Sequence[Mapping[str, Any]],
    operations: Sequence[str],
    containers: Sequence[str],
) -> str:
    header = ["viewers"]
    header += [f"{name} p50/p95/p99 ms" for name in operations]
    header += ["errors / requests", "renderer calls", "source reads/min", "passes"]
    header += [f"{name} CPU % mean (max)" for name in containers]
    lines = ["| " + " | ".join(header) + " |", "|" + "---|" * len(header)]

    def number(value: Any) -> str:
        if value is None:
            return "-"
        return f"{value:g}" if isinstance(value, float) else str(value)

    for level in levels:
        cells = [
            f"{level['viewers']} (idle)"
            if level["viewers"] == 0
            else str(level["viewers"])
        ]
        for name in operations:
            summary = level["operations"].get(name) or {}
            if summary.get("n"):
                cells.append(
                    " / ".join(number(summary[f"p{q}_ms"]) for q in (50, 95, 99))
                )
            else:
                cells.append("-")
        cells.append(f"{level['errors']} / {level['requests']}")
        cells.append(str(level["renderer_calls"]))
        cells.append(number(level["source_reads_per_minute"]))
        cells.append(str(level["passes"]))
        for name in containers:
            cpu = level["cpu"].get(name)
            cells.append(f"{cpu['mean']:.1f} ({cpu['max']:.1f})" if cpu else "-")
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines)


def measure_level(
    options: ViewersOptions,
    viewers: int,
    setups: Sequence[ViewerSetup],
    token: str,
    say: Callable[[str], None],
) -> dict[str, Any]:
    workers = min(options.processes, viewers) if viewers else 0
    start_at = time.time() + (3.0 if workers else 0.0)
    warmup = options.warmup if viewers else 0.0
    measure_from = start_at + warmup
    measure_until = measure_from + options.duration
    plans = [
        WorkerPlan(
            graphql_url=options.graphql_url,
            token=token,
            token_command=options.token_command,
            election_event_id=options.election_event_id,
            election_id=options.election_id,
            dashboard_id=options.dashboard_id,
            viewers=list(setups[worker::workers]),
            total_viewers=viewers,
            cycle=options.cycle,
            poll_interval=options.poll_interval,
            start_at=start_at,
            measure_from=measure_from,
            measure_until=measure_until,
            connections=max(1, options.connections // workers),
            timeout=options.timeout,
        )
        for worker in range(workers)
    ]
    executor = (
        ProcessPoolExecutor(workers, mp_context=multiprocessing.get_context("spawn"))
        if workers
        else None
    )
    try:
        futures = (
            [executor.submit(run_worker, plan) for plan in plans] if executor else []
        )
        say(
            f"{viewers} viewers on {workers} workers: warm-up {warmup:.0f}s, "
            f"window {options.duration:.0f}s"
        )
        time.sleep(max(0.0, measure_from - time.time()))
        before = options.stack.table_stats()
        sampler = StatsSampler(options.stack, options.stats_interval)
        sampler.start()
        time.sleep(max(0.0, measure_until - time.time()))
        cpu = sampler.stop()
        after = options.stack.table_stats()
        reports = [future.result() for future in futures]
    finally:
        if executor:
            executor.shutdown(cancel_futures=True)
    window = measure_until - measure_from
    renderer_calls = options.stack.renderer_calls(measure_from, measure_until)
    report = merge_reports(reports)
    return level_summary(
        viewers,
        report,
        read_deltas(before, after),
        renderer_calls,
        cpu,
        window,
        workers,
    )


def say_now(text: str) -> None:
    print(text, flush=True)


def run_viewers(options: ViewersOptions, say: Callable[[str], None] = say_now) -> Path:
    token = fetch_token(options.token_command)
    dashboard = probe_dashboard(options, token)
    posts = [post["key"] for post in dashboard["scope_options"]["posts"]]
    widgets = len(render_requests(options.election_event_id, dashboard, PROFILE_PROBE))
    parameters = {
        "graphql_url": options.graphql_url,
        "tenant_id": options.tenant_id,
        "election_event_id": options.election_event_id,
        "election_id": options.election_id,
        "dashboard_id": options.dashboard_id,
        "viewers": options.viewers,
        "cycle": options.cycle.value,
        "poll_interval": options.poll_interval,
        "warmup": options.warmup,
        "duration": options.duration,
        "processes": options.processes,
        "connections": options.connections,
        "seed": options.seed,
        "dark_share": options.dark_share,
        "locales": options.locales,
        "post_share": options.post_share,
        "profiles": [asdict(profile) for profile in PROFILES],
        "posts": posts,
        "widgets_drawn": widgets,
        "snapshot_revision_at_start": (dashboard.get("snapshot") or {}).get("revision"),
        "stack": asdict(options.stack),
        "services_checkout": git_state(options.checkout) if options.checkout else None,
        "conditions": [
            f"cycle {options.cycle.value}",
            f"poll {options.poll_interval:g}s",
            "live stack; the snapshot job keeps running",
        ],
    }
    run = start_run(
        scenario=SCENARIO,
        target=options.dashboard_id,
        label=options.label,
        cache=CacheState.WARM,
        cache_detail="Harvest's render cache fills during each level's warm-up; "
        "a new snapshot revision starts it again",
        checkout=None,
        output_dir=options.output_dir,
        services=list(options.stack.containers),
        commands=[f"token: {options.token_command}"],
        parameters=parameters,
        extra_tools={"docker": ["docker", "--version"]},
    )
    levels = []
    counts = ([0] if options.baseline else []) + list(options.viewers)
    for index, viewers in enumerate(counts):
        if index:
            time.sleep(options.settle)
            token = fetch_token(options.token_command)
        timer = SampleTimer(
            index, SampleRole.BASELINE if viewers == 0 else SampleRole.MEASURED
        )
        setups = viewer_setups(
            viewers,
            seed=options.seed,
            dark_share=options.dark_share,
            locales=options.locales,
            post_share=options.post_share,
            posts=posts,
        )
        level = measure_level(options, viewers, setups, token, say)
        levels.append(level)
        render = level["operations"].get(Operation.RENDER.value, {})
        p95 = render.get("p95_ms")
        run.add(
            timer.finish(
                ok=viewers == 0 or render.get("n", 0) > 0,
                seconds=p95 / 1000 if p95 is not None else None,
                detail=level,
            )
        )
        say(markdown_table([level], [op.value for op in Operation], ["harvest"]))
    run.result.notes.append(
        markdown_table(levels, [op.value for op in Operation], options.stack.containers)
    )
    path = run.finish()
    say(
        markdown_table(levels, [op.value for op in Operation], options.stack.containers)
    )
    return path


# The widgets any viewer draws do not depend on the screen.
PROFILE_PROBE = ViewerSetup(0, PROFILES[0], "LIGHT", "en", None)


def locale_weights(text: str) -> dict[str, float]:
    weights = {}
    for part in text.split(","):
        if not part.strip():
            continue
        name, _, weight = part.partition("=")
        weights[name.strip()] = float(weight) if weight else 1.0
    if not weights or any(weight < 0 for weight in weights.values()):
        raise ValueError(f"locales are NAME=WEIGHT pairs: {text!r}")
    return weights


def default_token_command() -> str:
    python = shlex.quote(os.environ.get("PYTHON", "python3"))
    return f"{python} -m scripts.dev.bench.admin_token"
