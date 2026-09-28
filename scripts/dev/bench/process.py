# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Timed commands, owned background servers and readiness waits."""

from __future__ import annotations

import contextlib
import os
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from types import TracebackType

OUTPUT_TAIL_BYTES = 4000
STOP_GRACE_SECONDS = 15.0

Command = str | Sequence[str]


@dataclass
class CommandResult:
    command: str
    returncode: int
    seconds: float
    output_tail: str

    @property
    def ok(self) -> bool:
        return self.returncode == 0


def describe_command(command: Command) -> str:
    return command if isinstance(command, str) else " ".join(command)


def _tail(log: Path, offset: int) -> str:
    with log.open("rb") as handle:
        handle.seek(max(offset, handle.seek(0, os.SEEK_END) - OUTPUT_TAIL_BYTES))
        return handle.read().decode("utf-8", errors="replace")


def run_command(
    command: Command,
    *,
    cwd: Path,
    log: Path,
    env: Mapping[str, str] | None = None,
    timeout: float | None = None,
) -> CommandResult:
    """Runs to completion, appending output to ``log``; strings run in a shell."""
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("ab") as handle:
        offset = handle.tell()
        handle.write(f"$ {describe_command(command)}\n".encode())
        handle.flush()
        started = time.monotonic()
        process = subprocess.Popen(
            command,
            cwd=cwd,
            env=dict(env) if env is not None else None,
            shell=isinstance(command, str),
            stdout=handle,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        try:
            returncode = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            _kill_group(process)
            returncode = -signal.SIGKILL
        except BaseException:
            _kill_group(process)
            raise
        seconds = time.monotonic() - started
    return CommandResult(
        describe_command(command), returncode, seconds, _tail(log, offset)
    )


def _kill_group(process: subprocess.Popen[bytes]) -> None:
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=STOP_GRACE_SECONDS)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
    except ProcessLookupError:
        pass


class BackgroundProcess:
    """A server started in its own process group and always stopped with it."""

    def __init__(
        self,
        command: Command,
        *,
        cwd: Path,
        log: Path,
        env: Mapping[str, str] | None = None,
    ) -> None:
        self.command = describe_command(command)
        self.log = log
        log.parent.mkdir(parents=True, exist_ok=True)
        self._handle = log.open("ab")
        self._handle.write(f"$ {self.command}\n".encode())
        self._handle.flush()
        self.process = subprocess.Popen(
            command,
            cwd=cwd,
            env=dict(env) if env is not None else None,
            shell=isinstance(command, str),
            stdout=self._handle,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )

    def alive(self) -> bool:
        return self.process.poll() is None

    def stop(self) -> None:
        if self.alive():
            _kill_group(self.process)
        else:
            # The leader may be gone while children it spawned still run.
            with contextlib.suppress(ProcessLookupError):
                os.killpg(self.process.pid, signal.SIGKILL)
        self._handle.close()

    def __enter__(self) -> BackgroundProcess:
        return self

    def __exit__(
        self,
        kind: type[BaseException] | None,
        value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        self.stop()


def port_in_use(port: int, host: str = "127.0.0.1") -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.settimeout(1.0)
        return probe.connect_ex((host, port)) == 0


def http_status(url: str, timeout: float = 5.0) -> int | None:
    """HTTP status of a GET, or ``None`` when nothing answers."""
    request = urllib.request.Request(url, headers={"Accept": "*/*"})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return int(response.status)
    except urllib.error.HTTPError as error:
        return int(error.code)
    except (urllib.error.URLError, OSError, ValueError):
        return None


def wait_for_http(
    url: str,
    *,
    timeout: float,
    interval: float = 0.5,
    alive: BackgroundProcess | None = None,
) -> float:
    """Seconds until ``url`` answers 2xx; fails early if its server exits."""
    started = time.monotonic()
    while True:
        status = http_status(url)
        if status is not None and 200 <= status < 300:
            return time.monotonic() - started
        if alive is not None and not alive.alive():
            raise RuntimeError(f"server exited before {url} was ready; see {alive.log}")
        if time.monotonic() - started > timeout:
            raise TimeoutError(f"{url} not ready after {timeout:.0f}s (last {status})")
        time.sleep(interval)
