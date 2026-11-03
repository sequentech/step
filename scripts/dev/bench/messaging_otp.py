# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""OTP latency while a bulk send saturates the same account (INFRA-MSG §2.8).

Three processes besides this one, all built from the checkout:

- the loopback provider (``messaging`` example ``loopback_provider``,
  test code): accepts every send after ``--provider-latency-ms``;
- harvest (the ``harvest`` binary) on a free local port, with the stack's
  environment and the account's provider endpoint variable
  (``MESSAGING_META_GRAPH_URL`` or ``MESSAGING_INFOBIP_URL``) pointed at the
  loopback provider;
- the bulk send (``windmill`` example ``messaging_bulk_load``):
  ``--bulk-workers`` concurrent workers sending notices through windmill's
  dispatch path, on the same account and the same shared bucket.

For the run, the account's limits are set to ``--messages-per-second`` and
``--otp-reserved-per-second`` (and restored afterwards), and its bucket is
emptied before each window so each starts full.

Windows: a baseline of codes alone (unless ``--no-baseline``), then the
bulk send starts and, after ``--warmup`` seconds, codes are sent for
``--duration`` seconds while it runs. Codes go to harvest's
``POST /messages/send`` (purpose OTP) with Keycloak's service token, at
``--otp-rate`` per second (open loop), each timed from request to answer.
Recorded per code: latency, HTTP status, the state Keycloak would get
(ACCEPTED; FAILED with the capacity reason past the budget) and its reason; per
window: p50/p95/p99, the bulk send's throughput and the provider's peak
concurrency.

Writes to the dev database: ledger rows with the logical keys
``bench-bulk:<run>:*`` and ``keycloak:bench-otp:<run>:*``, deleted at the end
unless ``--keep-rows``. The electoral log entries harvest posts for the codes
stay. Use a dev tenant and an account harvest chooses for codes (the
event's, with ``--election-event-id``, or the tenant default for the
channel); the run checks that before measuring.
"""

from __future__ import annotations

import asyncio
import json
import math
import os
import re
import shlex
import socket
import statistics
import subprocess
import threading
import time
import urllib.parse
import urllib.request
import uuid
from collections import Counter
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass, field
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import Any

from .common import start_run
from .http_pool import HttpError, HttpPool
from .results import CacheState, Sample, SampleRole, load_average, utc_now, write_result

SCENARIO = "messaging-otp"

# Providers the loopback provider answers for, and the deployment variable
# that sends the account's messages to it.
ENDPOINT_VARIABLES = {
    "WHATSAPP_CLOUD_API": "MESSAGING_META_GRAPH_URL",
    "MESSENGER_SEND_API": "MESSAGING_META_GRAPH_URL",
    "VIBER_INFOBIP": "MESSAGING_INFOBIP_URL",
}
# Codes go to synthetic numbers of a range no voter has.
CODE_DESTINATION = "+63918{:07d}"
CODE_LIFETIME = timedelta(minutes=5)
# What harvest records when a message waited past its capacity budget
# (`windmill::postgres::rate_bucket::CAPACITY_BUSY`).
CAPACITY_BUSY = "Not sent: the account stayed at its sending limit for too long"
UUID_PATTERN = re.compile(
    r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
)
INTERPOLATION = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}")


@dataclass
class Database:
    """The backend database, through ``docker exec … psql``."""

    container: str
    database: str
    user: str = "postgres"

    def query(self, sql: str) -> list[list[str]]:
        output = subprocess.run(
            [
                "docker",
                "exec",
                self.container,
                "psql",
                "-U",
                self.user,
                "-d",
                self.database,
                "-v",
                "ON_ERROR_STOP=1",
                "-At",
                "-F",
                "\t",
                "-c",
                sql,
            ],
            capture_output=True,
            text=True,
            check=True,
            timeout=120,
        ).stdout
        return [line.split("\t") for line in output.splitlines() if line]


@dataclass
class OtpOptions:
    label: str
    checkout: Path
    tenant_id: str
    account_id: str
    election_event_id: str | None
    messages_per_second: int
    otp_reserved_per_second: int
    otp_rate: float
    duration: float
    warmup: float
    baseline: bool
    bulk_workers: int
    provider_latency_ms: int
    profile: str
    build: bool
    keep_rows: bool
    token_command: str | None
    timeout: float
    database: Database
    output_dir: Path
    run_id: str = field(default_factory=lambda: uuid.uuid4().hex[:12])


@dataclass
class Code:
    """One code sent, as timed."""

    seconds: float
    status: int | None
    state: str | None
    reason: str | None
    error: str | None = None
    started_at: str = ""
    finished_at: str = ""


def percentile(values: Sequence[float], fraction: float) -> float | None:
    """Nearest-rank percentile; ``None`` without values."""
    if not values:
        return None
    ordered = sorted(values)
    rank = max(1, min(len(ordered), math.ceil(fraction * len(ordered) - 1e-9)))
    return ordered[rank - 1]


def window_summary(codes: Sequence[Code]) -> dict[str, Any]:
    answered = [code.seconds for code in codes if code.status == 200]
    return {
        "codes": len(codes),
        "answered": len(answered),
        "p50": percentile(answered, 0.50),
        "p95": percentile(answered, 0.95),
        "p99": percentile(answered, 0.99),
        "max": max(answered) if answered else None,
        "mean": statistics.fmean(answered) if answered else None,
        "states": dict(Counter(code.state or f"http-{code.status}" for code in codes)),
        "capacity_busy": sum(1 for code in codes if code.reason == CAPACITY_BUSY),
    }


def interpolated(values: Mapping[str, str], base: Mapping[str, str]) -> dict[str, str]:
    """``${NAME}`` references resolved as Compose does, process values first."""
    merged = {**values, **base}

    def resolve(value: str, depth: int = 0) -> str:
        if depth > 8:
            return value
        return INTERPOLATION.sub(
            lambda match: resolve(merged.get(match.group(1), ""), depth + 1), value
        )

    return {key: resolve(value) for key, value in merged.items()}


def stack_environment(checkout: Path) -> dict[str, str]:
    """The checkout's ``.devcontainer/.env`` under the process environment."""
    from ..mode.checkout import parse_dotenv

    env_file = checkout / ".devcontainer" / ".env"
    values = (
        parse_dotenv(env_file.read_text(encoding="utf-8")) if env_file.is_file() else {}
    )
    return interpolated(values, os.environ)


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("127.0.0.1", 0))
        return int(probe.getsockname()[1])


def code_request(options: OtpOptions, channel: str, number: int) -> dict[str, Any]:
    """The request Keycloak sends harvest for one code."""
    code = f"{number % 1_000_000:06d}"
    return {
        "tenant_id": options.tenant_id,
        "election_event_id": options.election_event_id,
        "voter_id": None,
        "channel": channel,
        "purpose": "OTP",
        "destination": CODE_DESTINATION.format(number % 10_000_000),
        "language": "en",
        "content": {
            "subject": None,
            "text": f"Your code is {code}",
            "html": None,
            "template_parameters": [code],
            "code": code,
        },
        "logical_key": f"bench-otp:{options.run_id}:{number}",
        "expires_at": (datetime.now(UTC) + CODE_LIFETIME).isoformat(),
    }


def service_token(
    environment: Mapping[str, str], tenant_id: str, command: str | None
) -> str:
    """Keycloak's service token for the tenant: what its extensions send."""
    if command:
        return subprocess.run(
            command, shell=True, capture_output=True, text=True, check=True, timeout=60
        ).stdout.strip()
    keycloak = environment.get("KEYCLOAK_URL", "http://keycloak:8090").rstrip("/")
    body = urllib.parse.urlencode(
        {
            "grant_type": "client_credentials",
            "client_id": environment.get("KEYCLOAK_CLIENT_ID", "service-account"),
            "client_secret": environment.get("KEYCLOAK_CLIENT_SECRET", ""),
        }
    ).encode()
    request = urllib.request.Request(
        f"{keycloak}/realms/tenant-{tenant_id}/protocol/openid-connect/token",
        data=body,
        headers={"Content-Type": "application/x-www-form-urlencoded"},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return str(json.load(response)["access_token"])


class Process:
    """A child process whose stdout lines are kept and whose last JSON line
    is read as its latest counts."""

    def __init__(
        self, command: Sequence[str], *, cwd: Path, env: Mapping[str, str], log: Path
    ):
        self.command = shlex.join(command)
        log.parent.mkdir(parents=True, exist_ok=True)
        self._log = log.open("a", encoding="utf-8")
        self._log.write(f"$ {self.command}\n")
        self.lines: list[str] = []
        self.process = subprocess.Popen(
            list(command),
            cwd=cwd,
            env=dict(env),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
        self._reader = threading.Thread(target=self._read, daemon=True)
        self._reader.start()

    def _read(self) -> None:
        assert self.process.stdout is not None
        for line in self.process.stdout:
            self.lines.append(line.rstrip("\n"))
            self._log.write(line)
            self._log.flush()

    def wait_for_line(self, predicate: Callable[[str], bool], timeout: float) -> str:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            for line in list(self.lines):
                if predicate(line):
                    return line
            if self.process.poll() is not None:
                break
            time.sleep(0.1)
        raise RuntimeError(f"{self.command}: not ready; see {self._log.name}")

    def last_json(self) -> dict[str, Any]:
        for line in reversed(self.lines):
            if line.startswith("{"):
                try:
                    return dict(json.loads(line))
                except ValueError:
                    continue
        return {}

    def stop(self) -> None:
        if self.process.poll() is None:
            os.killpg(self.process.pid, 15)
            try:
                self.process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, 9)
        self._reader.join(timeout=5)
        self._log.close()


def wait_for_port(port: int, timeout: float, alive: Process) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
            probe.settimeout(0.5)
            if probe.connect_ex(("127.0.0.1", port)) == 0:
                return
        if alive.process.poll() is not None:
            raise RuntimeError(f"{alive.command} exited before listening")
        time.sleep(0.25)
    raise TimeoutError(f"nothing listens on {port} after {timeout:.0f}s")


async def send_codes(
    options: OtpOptions,
    harvest_url: str,
    token: str,
    channel: str,
    first: int,
    count: int,
) -> list[Code]:
    """``count`` codes at ``--otp-rate`` per second, each timed."""
    pool = HttpPool(f"{harvest_url}/messages/send", limit=256, timeout=options.timeout)
    headers = {"Authorization": f"Bearer {token}"}
    started = time.monotonic()

    async def one(index: int) -> Code:
        await asyncio.sleep(
            max(0.0, started + index / options.otp_rate - time.monotonic())
        )
        asked, started_at = time.monotonic(), utc_now()
        try:
            status, answer = await pool.post_json(
                code_request(options, channel, first + index), headers
            )
        except HttpError as error:
            return Code(
                time.monotonic() - asked,
                None,
                None,
                None,
                str(error),
                started_at,
                utc_now(),
            )
        answer = answer if isinstance(answer, dict) else {}
        return Code(
            time.monotonic() - asked,
            status,
            answer.get("state"),
            answer.get("reason"),
            None,
            started_at,
            utc_now(),
        )

    try:
        return list(await asyncio.gather(*(one(index) for index in range(count))))
    finally:
        await pool.close()


def run_messaging_otp(options: OtpOptions) -> Path:
    for name, value in (("tenant", options.tenant_id), ("account", options.account_id)):
        if not UUID_PATTERN.match(value):
            raise ValueError(f"--{name}-id is not a UUID: {value}")
    packages = options.checkout / "packages"
    target = packages / "rust-local-target"
    profile_dir = "debug" if options.profile == "dev" else options.profile
    builds = [
        [
            "cargo",
            "build",
            "--profile",
            options.profile,
            "-p",
            "messaging",
            "--features",
            "test-support",
            "--example",
            "loopback_provider",
        ],
        [
            "cargo",
            "build",
            "--profile",
            options.profile,
            "-p",
            "windmill",
            "--example",
            "messaging_bulk_load",
        ],
        [
            "cargo",
            "build",
            "--profile",
            options.profile,
            "-p",
            "harvest",
            "--bin",
            "harvest",
        ],
    ]
    database = options.database
    rows = database.query(
        "SELECT channel, provider, limits::text FROM sequent_backend.messaging_account "
        f"WHERE id = '{options.account_id}' AND tenant_id = '{options.tenant_id}'"
    )
    if not rows:
        raise ValueError("no such account in the tenant")
    channel, provider, original_limits = rows[0]
    # The run changes the account's limits and restores them at the end;
    # a run killed hard can't, so the statement is printed first.
    restore_statement = (
        "UPDATE sequent_backend.messaging_account SET limits = '"
        + original_limits.replace("'", "''")
        + f"'::jsonb WHERE id = '{options.account_id}';"
    )
    print(f"to restore the account's limits by hand: {restore_statement}", flush=True)
    variable = ENDPOINT_VARIABLES.get(provider)
    if variable is None:
        raise ValueError(f"{provider}: the loopback provider can't answer for it")
    environment = stack_environment(options.checkout)
    environment["CARGO_TARGET_DIR"] = str(target)
    run = start_run(
        scenario=SCENARIO,
        target=channel.lower(),
        label=options.label,
        cache=CacheState.WARM,
        cache_detail=(
            "running stack; harvest, windmill and the provider as local processes"
        ),
        checkout=options.checkout,
        output_dir=options.output_dir,
        services=["postgres", "keycloak"],
        commands=[shlex.join(command) for command in builds],
        parameters={
            "run_id": options.run_id,
            "provider": provider,
            "messages_per_second": options.messages_per_second,
            "otp_reserved_per_second": options.otp_reserved_per_second,
            "otp_rate": options.otp_rate,
            "duration": options.duration,
            "warmup": options.warmup,
            "bulk_workers": options.bulk_workers,
            "provider_latency_ms": options.provider_latency_ms,
            "profile": options.profile,
        },
    )
    if options.build:
        for command in builds:
            subprocess.run(command, cwd=packages, env=environment, check=True)
    binaries = target / profile_dir
    processes: list[Process] = []
    index = 0
    try:
        provider_port = free_port()
        loopback = Process(
            [
                str(binaries / "examples" / "loopback_provider"),
                "--port",
                str(provider_port),
                "--latency-ms",
                str(options.provider_latency_ms),
            ],
            cwd=packages,
            env=environment,
            log=run.logs / "provider.log",
        )
        processes.append(loopback)
        loopback.wait_for_line(lambda line: line.startswith("listening "), 30)
        environment[variable] = f"http://127.0.0.1:{provider_port}"

        harvest_port = free_port()
        harvest_environment = {
            **environment,
            "ROCKET_ADDRESS": "127.0.0.1",
            "ROCKET_PORT": str(harvest_port),
            "HARVEST_PROBE_ADDR": f"127.0.0.1:{free_port()}",
        }
        harvest = Process(
            [str(binaries / "harvest")],
            cwd=packages,
            env=harvest_environment,
            log=run.logs / "harvest.log",
        )
        processes.append(harvest)
        wait_for_port(harvest_port, 300, harvest)
        harvest_url = f"http://127.0.0.1:{harvest_port}"

        database.query(
            "UPDATE sequent_backend.messaging_account "
            "SET limits = limits || jsonb_build_object("
            f"'messages_per_second', {options.messages_per_second}, "
            f"'otp_reserved_per_second', {options.otp_reserved_per_second}) "
            f"WHERE id = '{options.account_id}'"
        )
        token = service_token(environment, options.tenant_id, options.token_command)

        # One code first: harvest must send it through this account.
        smoke = asyncio.run(send_codes(options, harvest_url, token, channel, index, 1))[
            0
        ]
        index += 1
        used = database.query(
            "SELECT account_id FROM sequent_backend.message WHERE tenant_id = "
            f"'{options.tenant_id}' AND logical_key = "
            f"'keycloak:bench-otp:{options.run_id}:0'"
        )
        if (
            smoke.status != 200
            or smoke.state != "ACCEPTED"
            or not used
            or used[0][0] != options.account_id
        ):
            raise ValueError(
                "the first code wasn't accepted through the account: "
                f"status {smoke.status}, "
                f"state {smoke.state}, reason {smoke.reason!r}, account {used}"
            )

        def measure(role: SampleRole, window: str) -> dict[str, Any]:
            nonlocal index
            count = max(1, int(options.duration * options.otp_rate))
            load = load_average()
            codes = asyncio.run(
                send_codes(options, harvest_url, token, channel, index, count)
            )
            for offset, code in enumerate(codes):
                run.result.samples.append(
                    Sample(
                        index=index + offset,
                        role=role,
                        ok=code.status == 200,
                        seconds=code.seconds,
                        started_at=code.started_at,
                        finished_at=code.finished_at,
                        load_before=load,
                        load_after=load_average(),
                        detail={
                            "window": window,
                            "state": code.state,
                            "reason": code.reason,
                        },
                        error=code.error,
                    )
                )
            index += count
            write_result(run.path, run.result)
            return window_summary(codes)

        summaries: dict[str, Any] = {}
        if options.baseline:
            database.query(
                "DELETE FROM sequent_backend.messaging_rate_bucket "
                f"WHERE account_id = '{options.account_id}'"
            )
            summaries["baseline"] = measure(SampleRole.BASELINE, "baseline")

        database.query(
            "DELETE FROM sequent_backend.messaging_rate_bucket "
            f"WHERE account_id = '{options.account_id}'"
        )
        bulk = Process(
            [
                str(binaries / "examples" / "messaging_bulk_load"),
                "--tenant-id",
                options.tenant_id,
                "--account-id",
                options.account_id,
                "--run",
                options.run_id,
                "--workers",
                str(options.bulk_workers),
                "--messages",
                "10000000",
                "--seconds",
                str(int(options.warmup + options.duration + 5)),
            ],
            cwd=packages,
            env=environment,
            log=run.logs / "bulk.log",
        )
        processes.append(bulk)
        time.sleep(options.warmup)
        before = bulk.last_json()
        saturated = measure(SampleRole.MEASURED, "saturated")
        after = bulk.last_json()
        seconds = float(after.get("seconds", 0)) - float(before.get("seconds", 0))
        saturated["bulk_per_second"] = (
            (int(after.get("accepted", 0)) - int(before.get("accepted", 0))) / seconds
            if seconds > 0
            else None
        )
        saturated["bulk_failed"] = int(after.get("failed", 0))
        saturated["provider"] = loopback.last_json()
        summaries["saturated"] = saturated
        for window, summary in summaries.items():
            run.result.notes.append(f"{window}: {json.dumps(summary, sort_keys=True)}")
        print(json.dumps(summaries, indent=2, sort_keys=True))
    finally:
        for process in reversed(processes):
            process.stop()
        restore = original_limits.replace("'", "''")
        database.query(
            f"UPDATE sequent_backend.messaging_account SET limits = '{restore}'::jsonb "
            f"WHERE id = '{options.account_id}'"
        )
        if not options.keep_rows:
            database.query(
                "DELETE FROM sequent_backend.message WHERE tenant_id = "
                f"'{options.tenant_id}' AND "
                f"(logical_key LIKE 'bench-bulk:{options.run_id}:%' "
                f"OR logical_key LIKE 'keycloak:bench-otp:{options.run_id}:%')"
            )
    return run.finish()
