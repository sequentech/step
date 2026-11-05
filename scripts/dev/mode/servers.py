# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Dev servers in the devcontainer: who listens, and the ones step-dev starts."""

from __future__ import annotations

import contextlib
import os
import shlex
import shutil
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

from .checkout import Checkout, running_in_container
from .docker import ContainerState, DockerError, docker
from .manifest import Server

PROC_TCP_TABLES = ("/proc/net/tcp", "/proc/net/tcp6")
TCP_LISTEN = "0A"
# Logs and process ids of the servers step-dev starts, inside the checkout.
STATE_DIR = Path(".cache/dev-mode")
HTTP_TIMEOUT_SECONDS = 10
STOP_GRACE_SECONDS = 15
POLL_SECONDS = 1.0
SHORT_ID = 12
MAX_OWNER_LENGTH = 100


def listening_sockets(tables: Iterable[str]) -> dict[int, set[str]]:
    """Listening TCP ports and their socket inodes, from /proc/net/tcp tables."""
    sockets: dict[int, set[str]] = {}
    for table in tables:
        for line in table.splitlines():
            fields = line.split()
            if len(fields) < 10 or fields[3] != TCP_LISTEN:
                continue
            port = int(fields[1].rpartition(":")[2], 16)
            sockets.setdefault(port, set()).add(fields[9])
    return sockets


def socket_owners(inodes: set[str], proc: Path = Path("/proc")) -> dict[str, int]:
    """The process holding each socket inode, among those this user may inspect."""
    owners: dict[str, int] = {}
    for entry in proc.iterdir():
        if not entry.name.isdigit():
            continue
        try:
            descriptors = list((entry / "fd").iterdir())
        except OSError:
            continue
        for descriptor in descriptors:
            try:
                target = os.readlink(descriptor)
            except OSError:
                continue
            if target.startswith("socket:[") and target[8:-1] in inodes:
                owners.setdefault(target[8:-1], int(entry.name))
    return owners


def describe_process(pid: int, proc: Path = Path("/proc")) -> str:
    try:
        raw = (proc / str(pid) / "cmdline").read_bytes()
    except OSError:
        return f"pid {pid}"
    command = " ".join(
        part.decode(errors="replace") for part in raw.split(b"\0") if part
    )
    if len(command) > MAX_OWNER_LENGTH:
        command = command[: MAX_OWNER_LENGTH - 3] + "..."
    return f"pid {pid}: {command}"


@dataclass(frozen=True)
class ServerState:
    server: Server
    listening: bool
    # A server step-dev started in the current devcontainer and still running.
    pid: int | None
    # The process holding the port, when visible from here.
    owner: str | None

    @property
    def url(self) -> str:
        return f"http://127.0.0.1:{self.server.port}"


@dataclass(frozen=True)
class Started:
    pid: int
    # Set when this process started the server and must reap it.
    process: subprocess.Popen[bytes] | None


class Devcontainer:
    """Runs commands in the checkout's devcontainer.

    Directly when this process runs inside it, through ``docker exec`` otherwise.
    """

    def __init__(self, checkout: Checkout, container: ContainerState | None) -> None:
        self.checkout = checkout
        self.container = container
        self.local = (
            container is not None
            and running_in_container()
            and container.id.startswith(socket.gethostname())
            and len(socket.gethostname()) >= SHORT_ID
        )

    @property
    def running(self) -> bool:
        return self.container is not None and self.container.status == "running"

    def _exec(
        self, *command: str, check: bool = True
    ) -> subprocess.CompletedProcess[str]:
        if self.container is None:
            raise DockerError("the devcontainer does not exist")
        return docker(
            [
                "exec",
                "--workdir",
                self.checkout.container_root,
                self.container.id,
                *command,
            ],
            check=check,
        )

    def tcp_tables(self) -> list[str]:
        if self.local:
            return [
                Path(path).read_text()
                for path in PROC_TCP_TABLES
                if Path(path).exists()
            ]
        return [self._exec("cat", *PROC_TCP_TABLES, check=False).stdout]

    def _state_file(self, server: Server, suffix: str) -> Path:
        return self.checkout.root / STATE_DIR / f"{server.name}.{suffix}"

    def _recorded_pid(self, server: Server) -> int | None:
        """The process id step-dev recorded, if it belongs to this container."""
        try:
            pid_text, container_id = self._state_file(server, "pid").read_text().split()
        except (OSError, ValueError):
            return None
        if self.container is None or container_id != self.container.id:
            return None
        return int(pid_text) if pid_text.isdigit() else None

    def _alive(self, pid: int) -> bool:
        if self.local:
            try:
                os.kill(pid, 0)
            except OSError:
                return False
            return True
        return self._exec("kill", "-0", str(pid), check=False).returncode == 0

    def states(self, servers: Iterable[Server]) -> list[ServerState]:
        servers = list(servers)
        if not self.running:
            return [ServerState(server, False, None, None) for server in servers]
        sockets = listening_sockets(self.tcp_tables())
        owners: dict[str, int] = {}
        if self.local:
            wanted = set().union(
                *(sockets.get(server.port, set()) for server in servers)
            )
            owners = socket_owners(wanted)
        states = []
        for server in servers:
            pid = self._recorded_pid(server)
            if pid is not None and not self._alive(pid):
                pid = None
            owner = None
            for inode in sockets.get(server.port, ()):
                if inode in owners:
                    owner = describe_process(owners[inode])
            states.append(ServerState(server, server.port in sockets, pid, owner))
        return states

    def start(self, server: Server) -> Started:
        """Starts the server in a session of its own, logging to STATE_DIR."""
        if self.container is None:
            raise DockerError("the devcontainer does not exist")
        log = self._state_file(server, "log")
        log.parent.mkdir(parents=True, exist_ok=True)
        # The devenv shell provides node and yarn when the caller's environment
        # does not, as with docker exec.
        wrapped = ["devenv", "shell", "bash", "--", "-c", shlex.join(server.command)]
        if self.local:
            command = (
                list(server.command) if shutil.which(server.command[0]) else wrapped
            )
            with log.open("ab") as handle:
                process = subprocess.Popen(
                    command,
                    cwd=self.checkout.root,
                    stdin=subprocess.DEVNULL,
                    stdout=handle,
                    stderr=subprocess.STDOUT,
                    start_new_session=True,
                )
            started = Started(process.pid, process)
        else:
            relative_log = shlex.quote(str(STATE_DIR / log.name))
            script = (
                f"setsid {shlex.join(wrapped)} >> {relative_log} 2>&1 < /dev/null "
                "& echo $!"
            )
            output = self._exec("bash", "-c", script).stdout.strip()
            if not output.isdigit():
                raise DockerError(f"could not start {server.name}: {output}")
            started = Started(int(output), None)
        self._state_file(server, "pid").write_text(
            f"{started.pid} {self.container.id}\n"
        )
        return started

    def responds(self, server: Server) -> bool:
        url = f"http://127.0.0.1:{server.port}{server.ready_path}"
        if not self.local:
            return (
                self._exec(
                    "curl",
                    "-fsS",
                    "-o",
                    "/dev/null",
                    "--max-time",
                    str(HTTP_TIMEOUT_SECONDS),
                    url,
                    check=False,
                ).returncode
                == 0
            )
        try:
            with urllib.request.urlopen(url, timeout=HTTP_TIMEOUT_SECONDS) as response:
                return 200 <= response.status < 400
        except (urllib.error.URLError, OSError):
            return False

    def wait_ready(
        self, server: Server, started: Started, timeout: float
    ) -> str | None:
        """Waits for the server to answer; the reason it did not, otherwise."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.responds(server):
                return None
            exited = (
                started.process.poll() is not None
                if started.process is not None
                else not self._alive(started.pid)
            )
            if exited:
                return f"exited; see {self._state_file(server, 'log')}"
            time.sleep(POLL_SECONDS)
        return f"not ready after {timeout:.0f}s; see {self._state_file(server, 'log')}"

    def _signal_group(self, pid: int, signum: signal.Signals) -> None:
        if self.local:
            with contextlib.suppress(OSError):
                os.killpg(pid, signum)
        else:
            self._exec("kill", f"-{signum.name[3:]}", "--", f"-{pid}", check=False)

    def stop(self, server: Server, pid: int) -> None:
        self._signal_group(pid, signal.SIGTERM)
        deadline = time.monotonic() + STOP_GRACE_SECONDS
        while self._alive(pid) and time.monotonic() < deadline:
            time.sleep(POLL_SECONDS)
        if self._alive(pid):
            self._signal_group(pid, signal.SIGKILL)
        self._state_file(server, "pid").unlink(missing_ok=True)
