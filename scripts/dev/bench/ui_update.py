# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Save-to-visible time of a UI edit in running dev servers.

Dev servers start first and the browser probe (``probe.mjs``) opens each target
until its first screen renders. Each sample then writes the edited source with a
new unique marker, optionally runs a rebuild command (the workflow a shared
package needs when consumers read its build output), and times until headless
Chromium shows the marker. Samples chain markers instead of reverting in
between, so every sample is the same one-line change; the final revert is timed
separately and not counted.
"""

from __future__ import annotations

import json
import queue
import subprocess
import threading
import time
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .common import Run, SampleTimer, new_run_id, roles, start_run
from .edits import EditSpec, MarkerEdit, marker_for
from .process import (
    BackgroundProcess,
    CommandResult,
    port_in_use,
    run_command,
    wait_for_http,
)
from .results import CacheState, SampleRole

SCENARIO = "ui-update"
PROBE = Path(__file__).with_name("probe.mjs")


@dataclass(frozen=True)
class UiTarget:
    """A dev server and the page the probe opens on it."""

    package: str
    server: str
    probe_kind: str
    ready_path: str


# Portals keep their own start scripts; the port and host flags override the
# hard-coded development ports.
WEBPACK_SERVER = "yarn start --port {port} --host 127.0.0.1 --no-open"
TARGETS: dict[str, UiTarget] = {
    "voting": UiTarget("voting-portal", WEBPACK_SERVER, "voting", "/"),
    "admin": UiTarget("admin-portal", WEBPACK_SERVER, "admin", "/"),
    "verifier": UiTarget("ballot-verifier", WEBPACK_SERVER, "verifier", "/"),
    "results": UiTarget("results-portal", WEBPACK_SERVER, "results", "/"),
    "storybook": UiTarget(
        "ui-essentials",
        "yarn storybook --port {port} --host 127.0.0.1 --ci",
        "storybook",
        "/index.json",
    ),
}

EDITS: dict[str, EditSpec] = {
    # Rendered by the header of every portal and by its own story.
    "shared-header": EditSpec(
        path="packages/ui-essentials/src/components/Header/Header.tsx",
        anchor='<Version version={appVersion ?? {main: "0.0.0"}} />',
        template="<span>{marker}</span>",
    ),
    # The voting portal's first authenticated screen.
    "voting-screen": EditSpec(
        path="packages/voting-portal/src/routes/ElectionSelectionScreen.tsx",
        anchor='{t("electionSelectionScreen.title")}',
        template="<span>{marker}</span>",
    ),
}


@dataclass
class UiUpdateOptions:
    checkout: Path
    label: str
    edit_name: str
    edit: EditSpec
    targets: list[str]
    servers: dict[str, str]
    rebuild: str | None
    samples: int
    warmup: int
    port_base: int
    startup_timeout: float
    sample_timeout: float
    settle: float
    output_dir: Path


class BrowserError(RuntimeError):
    pass


def ends_wait(
    message: Mapping[str, Any], command: str, ids: Sequence[str], text: str | None
) -> bool:
    """Whether a probe error concerns the awaited command, not an abandoned wait."""
    return (
        message.get("id") in ids
        and message.get("cmd") == command
        and (text is None or message.get("text") == text)
    )


class BrowserProbe:
    """The Node probe process, spoken to in JSON lines."""

    def __init__(self, packages: Path, log: Path) -> None:
        self._log = log.open("ab")
        self.process = subprocess.Popen(
            ["node", str(PROBE), "--packages", str(packages)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self._log,
            text=True,
            bufsize=1,
            start_new_session=True,
        )
        self.events: queue.Queue[dict[str, Any]] = queue.Queue()
        self._reader = threading.Thread(target=self._read, daemon=True)
        self._reader.start()

    def _read(self) -> None:
        assert self.process.stdout is not None
        for line in self.process.stdout:
            if line.strip():
                self.events.put(json.loads(line))
        self.events.put({"event": "exit"})

    def send(self, **message: Any) -> None:
        assert self.process.stdin is not None
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def collect(
        self,
        event: str,
        command: str,
        ids: Sequence[str],
        timeout: float,
        text: str | None = None,
    ) -> dict[str, dict[str, Any]]:
        """The ``event`` that ``command`` sends for every id (and ``text``).

        Fails when that command fails or the timeout passes.
        """
        received: dict[str, dict[str, Any]] = {}
        deadline = time.monotonic() + timeout
        while len(received) < len(ids):
            try:
                message = self.events.get(timeout=max(0.1, deadline - time.monotonic()))
            except queue.Empty:
                missing = sorted(set(ids) - set(received))
                raise BrowserError(
                    f"no {event} from {missing} within {timeout:.0f}s"
                ) from None
            if message["event"] == "exit":
                raise BrowserError(f"the probe exited; see {self._log.name}")
            if message["event"] == "error":
                if ends_wait(message, command, ids, text):
                    raise BrowserError(f"probe {message.get('id')}: {message}")
                continue
            if (
                message["event"] == event
                and message.get("id") in ids
                and (text is None or message.get("text") == text)
            ):
                received[message["id"]] = message
        return received

    def close(self) -> None:
        if self.process.poll() is None:
            try:
                self.send(cmd="close")
                self.process.wait(timeout=30)
            except (OSError, subprocess.TimeoutExpired):
                self.process.kill()
        self._log.close()


def start_servers(
    options: UiUpdateOptions, log_dir: Path
) -> tuple[dict[str, BackgroundProcess], dict[str, float]]:
    servers: dict[str, BackgroundProcess] = {}
    startup: dict[str, float] = {}
    try:
        for offset, name in enumerate(options.targets):
            port = options.port_base + offset
            if port_in_use(port):
                raise RuntimeError(f"port {port} for {name} is in use")
            target = TARGETS[name]
            command = options.servers.get(name, target.server).format(port=port)
            started = time.monotonic()
            servers[name] = BackgroundProcess(
                command,
                cwd=options.checkout / "packages" / target.package,
                log=log_dir / f"server-{name}.log",
            )
            wait_for_http(
                f"http://127.0.0.1:{port}{target.ready_path}",
                timeout=options.startup_timeout,
                alive=servers[name],
            )
            startup[name] = time.monotonic() - started
    except BaseException:
        for server in servers.values():
            server.stop()
        raise
    return servers, startup


def conditions(options: UiUpdateOptions) -> list[str]:
    rebuild = f"rebuild: {options.rebuild}" if options.rebuild else "no rebuild command"
    servers = ", ".join(options.targets)
    return [
        rebuild,
        f"dev servers running together: {servers}",
        f"edit: {options.edit_name}",
    ]


def run_ui_update(options: UiUpdateOptions) -> list[Path]:
    edit = MarkerEdit(options.checkout, options.edit)
    run_id = new_run_id()
    log_dir = options.output_dir / SCENARIO / "logs" / f"{options.edit_name}-{run_id}"
    log_dir.mkdir(parents=True, exist_ok=True)
    servers: dict[str, BackgroundProcess] = {}
    probe: BrowserProbe | None = None
    runs: dict[str, Run] = {}
    try:
        servers, startup = start_servers(options, log_dir)
        probe = BrowserProbe(options.checkout / "packages", log_dir / "probe.log")
        opened_at = time.monotonic()
        for offset, name in enumerate(options.targets):
            probe.send(
                cmd="open",
                id=name,
                kind=TARGETS[name].probe_kind,
                origin=f"http://127.0.0.1:{options.port_base + offset}",
                timeout=options.startup_timeout,
            )
        probe.collect("opened", "open", options.targets, options.startup_timeout)
        first_render = time.monotonic() - opened_at
        for name in options.targets:
            runs[name] = start_run(
                scenario=SCENARIO,
                target=f"{options.edit_name}@{name}",
                label=options.label,
                cache=CacheState.WARM,
                cache_detail="dev servers running with their first compile done; "
                f"{options.warmup} untimed warm-up edit(s) before the samples",
                checkout=options.checkout,
                output_dir=options.output_dir,
                services=[
                    f"{target}: {server.command}" for target, server in servers.items()
                ],
                commands=[
                    f"edit {options.edit.path}",
                    *([options.rebuild] if options.rebuild else []),
                ],
                parameters={
                    "edit": options.edit.to_dict(),
                    "rebuild": options.rebuild,
                    "targets": options.targets,
                    "server_startup_seconds": startup,
                    "first_render_seconds": first_render,
                    "settle_seconds": options.settle,
                    "marker_run_id": run_id,
                    "conditions": conditions(options),
                },
            )
        for index, role in enumerate(roles(options.samples, options.warmup), start=1):
            marker = marker_for(run_id, index)
            measure(
                options, probe, edit, runs, log_dir, index, role, marker, apply=True
            )
        measure(
            options,
            probe,
            edit,
            runs,
            log_dir,
            0,
            SampleRole.REVERT,
            marker,
            apply=False,
        )
    finally:
        rebuild_after_restore = edit.modified() and options.rebuild is not None
        edit.restore()
        if rebuild_after_restore and options.rebuild is not None:
            # Leave the consumers' build output matching the restored source.
            run_command(
                options.rebuild, cwd=options.checkout, log=log_dir / "rebuild.log"
            )
        if probe is not None:
            probe.close()
        for server in servers.values():
            server.stop()
    return [run.finish() for run in runs.values()]


def measure(
    options: UiUpdateOptions,
    probe: BrowserProbe,
    edit: MarkerEdit,
    runs: Mapping[str, Run],
    log_dir: Path,
    index: int,
    role: SampleRole,
    marker: str,
    *,
    apply: bool,
) -> None:
    """One edit (or the final revert) until every target shows its result."""
    event, command = ("visible", "wait") if apply else ("gone", "gone")
    timers = {name: SampleTimer(index, role) for name in options.targets}
    for name in options.targets:
        probe.send(cmd=command, id=name, text=marker, timeout=options.sample_timeout)
    probe.collect("waiting", command, options.targets, 60, marker)
    saved = edit.apply(marker) if apply else edit.restore()
    phases: dict[str, float] = {}
    rebuild: CommandResult | None = None
    error: str | None = None
    if options.rebuild:
        rebuild = run_command(
            options.rebuild,
            cwd=options.checkout,
            log=log_dir / "rebuild.log",
            timeout=options.sample_timeout,
        )
        phases["rebuild"] = rebuild.seconds
        if not rebuild.ok:
            error = f"rebuild exited {rebuild.returncode}: {rebuild.output_tail[-800:]}"
    events: dict[str, dict[str, Any]] = {}
    if error is None:
        try:
            events = probe.collect(
                event, command, options.targets, options.sample_timeout, marker
            )
        except BrowserError as failure:
            error = str(failure)
    for name, run in runs.items():
        observed = events.get(name)
        seconds = None if observed is None else observed["t"] - saved
        detail = {"marker": marker, "saved_at_epoch": saved}
        if observed is not None:
            detail.update(
                visible_at_epoch=observed["t"],
                page_loads=observed.get("reloads"),
                page_errors=observed.get("page_errors"),
                mock_violations=observed.get("violations"),
            )
        run.add(
            timers[name].finish(
                ok=seconds is not None,
                seconds=seconds,
                phases={
                    **phases,
                    **({"visible": seconds} if seconds is not None else {}),
                },
                detail=detail,
                error=None if seconds is not None else error or "not observed",
            )
        )
    time.sleep(options.settle)
