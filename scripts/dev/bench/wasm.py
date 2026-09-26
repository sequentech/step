# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Rust/WASM save-to-browser time through the sequent-core packaging workflow.

A sample saves a statement with a new marker string in an exported WASM
function, stops the portal's dev server, runs the build command, reinstalls
dependencies, starts the server again and times until the browser has loaded a
WASM module containing the marker and rendered its first screen. With
``--server-restart never`` the server keeps running and must reload the page
itself. Without an edit (``--no-change``) the same sequence measures a no-op
invocation; with a running server it then ends when the commands finish.

The default build command is the checkout's own
``.devcontainer/scripts/build-sequent-core.sh`` with its hard-coded
``/workspaces/step`` directory replaced by the measured checkout; nothing else in
the script changes. The tracked files it rewrites (lock file and packed
archives) are restored afterwards and dependencies reinstalled.
"""

from __future__ import annotations

import os
import re
import subprocess
import time
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Any

from .common import Run, SampleTimer, new_run_id, roles, start_run
from .edits import EditSpec, MarkerEdit, marker_for
from .process import BackgroundProcess, run_command, wait_for_http
from .results import CacheState
from .rust import RUST_MARKER
from .ui_update import TARGETS, BrowserProbe

SCENARIO = "wasm"
BUILD_SCRIPT = ".devcontainer/scripts/build-sequent-core.sh"
SCRIPT_TARGET = "TARGET_DIR=/workspaces/step/packages/sequent-core"
NO_CHANGE = "no-change"
TRACE_PROMPT = "+${EPOCHREALTIME} "
# Commands whose first trace line starts a phase of the traced build script.
SCRIPT_PHASES = (
    ("wasm_pack_build", re.compile(r"^\+ wasm-pack build")),
    ("wasm_pack_pack", re.compile(r"^\+ wasm-pack -v pack")),
    ("lock_update", re.compile(r"^\+ awk ")),
    ("archives_copied", re.compile(r"^\+ cp ")),
    ("node_modules_removed", re.compile(r"^\+ rm -rf node_modules")),
)


class ServerRestart(Enum):
    """Whether the workflow restarts the portal's dev server after a build."""

    ALWAYS = "always"
    NEVER = "never"


WASM_EDITS: dict[str, EditSpec] = {
    # sort_elections_list_js is exported, so the marker stays in the module.
    "sequent-core-wasm": EditSpec(
        path="packages/sequent-core/src/wasm/wasm.rs",
        anchor="let elections_js: Value = "
        "serde_wasm_bindgen::from_value(elections_json)",
        template=RUST_MARKER,
    ),
}


@dataclass
class WasmOptions:
    checkout: Path
    label: str
    edit_name: str
    edit: EditSpec | None
    build: str | None
    install: str
    restart: ServerRestart
    target: str
    port: int
    samples: int
    warmup: int
    timeout: float
    output_dir: Path


def patched_script(checkout: Path, destination: Path) -> Path:
    """The checkout's build script aimed at the checkout instead of /workspaces/step."""
    text = (checkout / BUILD_SCRIPT).read_text(encoding="utf-8")
    if text.count(SCRIPT_TARGET) != 1:
        raise ValueError(
            f"{BUILD_SCRIPT} no longer sets {SCRIPT_TARGET}; pass --build-cmd"
        )
    patched = text.replace(
        SCRIPT_TARGET, f"TARGET_DIR={checkout}/packages/sequent-core"
    )
    if "/workspaces/" in patched:
        raise ValueError(
            f"{BUILD_SCRIPT} still refers to /workspaces; pass --build-cmd"
        )
    destination.write_text(patched, encoding="utf-8")
    return destination


def script_phases(trace: str, started: float) -> dict[str, float]:
    """Seconds from ``started`` to the first traced line of each script phase."""
    phases: dict[str, float] = {}
    for line in trace.splitlines():
        match = re.match(r"^\+(\d+\.\d+) (.*)$", line)
        if not match:
            continue
        stamp, command = float(match.group(1)), "+ " + match.group(2)
        for name, pattern in SCRIPT_PHASES:
            if name not in phases and pattern.match(command):
                phases[name] = stamp - started
    return phases


def tracked_changes(checkout: Path) -> set[str]:
    status = subprocess.run(
        [
            "git",
            "-C",
            str(checkout),
            "status",
            "--porcelain=v1",
            "--untracked-files=no",
        ],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return {line[3:] for line in status.splitlines() if line}


def restore_tracked(checkout: Path, paths: set[str]) -> None:
    if paths:
        subprocess.run(
            ["git", "-C", str(checkout), "checkout", "--", *sorted(paths)], check=True
        )


def run_wasm(options: WasmOptions) -> Path:
    before = tracked_changes(options.checkout)
    rewritten = {
        path for path in before if path == "packages/yarn.lock" or path.endswith(".tgz")
    }
    if rewritten:
        raise ValueError(
            f"the build rewrites files with local changes: {sorted(rewritten)}"
        )
    edit = MarkerEdit(options.checkout, options.edit) if options.edit else None
    run_id = new_run_id()
    target = TARGETS[options.target]
    log_dir = options.output_dir / SCENARIO / "logs" / f"{options.edit_name}-{run_id}"
    log_dir.mkdir(parents=True, exist_ok=True)
    # The trace timestamps each script command, which splits the build into phases.
    build = (
        options.build
        or f"bash -x {patched_script(options.checkout, log_dir / 'build.sh')}"
    )
    removals = []
    if options.build is None:
        script = (options.checkout / BUILD_SCRIPT).read_text(encoding="utf-8")
        removals = [line.strip() for line in script.splitlines() if "rm -" in line]
    server_command = target.server.format(port=options.port)
    package_dir = options.checkout / "packages" / target.package
    restarts = options.restart is ServerRestart.ALWAYS
    origin = f"http://127.0.0.1:{options.port}"
    run = start_run(
        scenario=SCENARIO,
        target=f"{options.edit_name}@{options.target}",
        label=options.label,
        cache=CacheState.WARM,
        cache_detail=f"{options.warmup} untimed run(s) first; Cargo target, Yarn cache "
        "and wasm-pack tools kept between samples",
        checkout=options.checkout,
        output_dir=options.output_dir,
        services=[f"{options.target}: {server_command}"],
        commands=[
            *([f"edit {options.edit.path}"] if options.edit else []),
            *(["stop the dev server"] if restarts else []),
            build,
            *([f"cd packages && {options.install}"] if options.install else []),
            *(
                [f"cd packages/{target.package} && {server_command}"]
                if restarts
                else []
            ),
        ],
        parameters={
            "edit": options.edit.to_dict() if options.edit else None,
            "build": build,
            "build_script_removals": removals,
            "install": options.install,
            "server_restart": options.restart.value,
            "marker_run_id": run_id,
            "conditions": [
                "no edit" if options.edit is None else f"edit: {options.edit.path}",
                f"build: {build if options.build else 'patched ' + BUILD_SCRIPT}",
                f"install: {options.install or 'none'}",
                f"dev server restart: {options.restart.value}",
            ],
        },
        extra_tools={"wasm-bindgen": ("wasm-bindgen", "--version")},
    )
    server: BackgroundProcess | None = None
    probe: BrowserProbe | None = None
    try:
        server = BackgroundProcess(
            server_command, cwd=package_dir, log=log_dir / "server.log"
        )
        wait_for_http(f"{origin}/", timeout=options.timeout, alive=server)
        probe = BrowserProbe(options.checkout / "packages", log_dir / "probe.log")
        probe.send(
            cmd="open",
            id=options.target,
            kind=target.probe_kind,
            origin=origin,
            timeout=options.timeout,
        )
        probe.collect("opened", "open", [options.target], options.timeout)
        for index, role in enumerate(roles(options.samples, options.warmup), start=1):
            server = measure(
                options,
                run,
                probe,
                server,
                edit,
                build,
                log_dir,
                SampleTimer(index, role),
                marker_for(run_id, index),
            )
    finally:
        if edit is not None:
            edit.restore()
        if probe is not None:
            probe.close()
        if server is not None:
            server.stop()
        changed = tracked_changes(options.checkout) - before
        restore_tracked(options.checkout, changed)
        if changed:
            run.result.notes.append(f"restored {sorted(changed)}")
            if options.install:
                run_command(
                    options.install,
                    cwd=options.checkout / "packages",
                    log=log_dir / "restore-install.log",
                )
    return run.finish()


def measure(
    options: WasmOptions,
    run: Run,
    probe: BrowserProbe,
    server: BackgroundProcess,
    edit: MarkerEdit | None,
    build: str,
    log_dir: Path,
    timer: SampleTimer,
    marker: str,
) -> BackgroundProcess:
    """One save-to-browser cycle; returns the restarted dev server."""
    target = TARGETS[options.target]
    origin = f"http://127.0.0.1:{options.port}"
    restarts = options.restart is ServerRestart.ALWAYS
    text = marker if edit is not None else None
    if not restarts and text is not None:
        # The running page watches for the new module before the save.
        probe.send(cmd="watch", id=options.target, text=text, timeout=options.timeout)
        probe.collect("waiting", "watch", [options.target], 60, text)
    saved = edit.apply(marker) if edit is not None else time.time()
    phases: dict[str, float] = {}
    detail: dict[str, Any] = {"marker": text}
    if restarts:
        probe.send(cmd="park", id=options.target)
        server.stop()
    trace = log_dir / f"build-{timer.index}.log"
    started = time.time()
    result = run_command(
        build,
        cwd=options.checkout,
        log=trace,
        env={**os.environ, "PS4": TRACE_PROMPT},
        timeout=options.timeout,
    )
    phases["build"] = time.time() - saved
    for name, seconds in script_phases(
        trace.read_text(errors="replace"), started
    ).items():
        phases[name] = started - saved + seconds
    error = None if result.ok else f"build exited {result.returncode}"
    if error is None and options.install:
        install = run_command(
            options.install,
            cwd=options.checkout / "packages",
            log=log_dir / f"install-{timer.index}.log",
            timeout=options.timeout,
        )
        phases["install"] = time.time() - saved
        error = None if install.ok else f"install exited {install.returncode}"
    if restarts:
        server = BackgroundProcess(
            target.server.format(port=options.port),
            cwd=options.checkout / "packages" / target.package,
            log=log_dir / "server.log",
        )
    if error is None:
        try:
            if restarts:
                wait_for_http(f"{origin}/", timeout=options.timeout, alive=server)
                phases["server_ready"] = time.time() - saved
                probe.send(
                    cmd="visit", id=options.target, text=text, timeout=options.timeout
                )
                event, command = "visited", "visit"
            else:
                event, command = "watched", "watch"
            if restarts or text is not None:
                observed = probe.collect(
                    event, command, [options.target], options.timeout
                )[options.target]
                phases["visible"] = observed["t"] - saved
                detail.update(
                    page_loads=observed.get("reloads"),
                    wasm_modules=observed.get("wasm_modules"),
                )
            else:
                # Nothing changed and nothing restarts: done when the commands are.
                phases["visible"] = time.time() - saved
        # Probe failures and a server that exits are RuntimeErrors.
        except (RuntimeError, TimeoutError) as failure:
            error = str(failure)
    run.add(
        timer.finish(
            ok=error is None,
            seconds=phases.get("visible"),
            phases=phases,
            detail=detail,
            error=error,
        )
    )
    return server
