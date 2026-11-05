# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Command-to-ready time of a devcontainer workspace in an isolated daemon.

Cold samples start from a fresh daemon and a fresh copy of the checkout (no
images, volumes, Nix store, target directories or node_modules). Warm samples
reuse an existing isolated stack: its containers are stopped, then brought up
again. The timer covers ``devcontainer up`` (initialize command, image builds,
container start, lifecycle commands) until every readiness probe has passed.
"""

from __future__ import annotations

import json
import shlex
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .common import Run, SampleTimer, new_run_id, roles, start_run
from .isolation import (
    Dind,
    Docker,
    IsolationError,
    daemon_address,
    daemon_memory,
    daemon_storage,
    materialize_checkout,
    read_env_file,
    remove_as_root,
    require_isolated,
    seed_image,
)
from .probes import Probe, ProbeKind, ProbeRunner
from .process import BackgroundProcess
from .results import CacheState

SCENARIO = "workspace"
ENV_FILES = (".devcontainer/.env.development", ".devcontainer/.env")
AFTER_UP_PID = "/tmp/step-bench-after-up.pid"
AFTER_UP_LOG = "/tmp/step-bench-after-up.log"
STOP_TIMEOUT_SECONDS = "30"
FOOTPRINT_TIMEOUT = 3600.0
# Build outputs that the stack writes into the checkout, measured after readiness.
CHECKOUT_OUTPUTS = (
    "packages/target",
    "packages/rust-local-target",
    "packages/step-cli/rust-local-target",
    "packages/node_modules",
)


@dataclass
class WorkspaceOptions:
    checkout: Path
    label: str
    cache: CacheState
    target: str
    probes: list[Probe]
    config: str | None
    docker_host: str | None
    dind_name: str | None
    sandbox_root: Path | None
    dind_prefix: str
    dind_image: str
    copy_name: str
    empty_mounts: list[Path]
    seeds: list[tuple[str, str]]
    keep: bool
    after_up: str | None
    samples: int
    warmup: int
    timeout: float
    poll_interval: float
    stats_interval: float
    devcontainer: str
    output_dir: Path
    footprint: bool


class MemorySampler:
    """Peak memory of the stack while a sample runs."""

    def __init__(self, docker: Docker, daemon: str | None, interval: float) -> None:
        self.docker = docker
        self.daemon = daemon
        self.interval = interval
        self.peak_total = 0
        self.peak_daemon: int | None = None
        self.peak_by_container: dict[str, int] = {}
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def start(self) -> None:
        self._thread.start()

    def stop(self) -> dict[str, Any]:
        self._stop.set()
        self._thread.join(120)
        return {
            "peak_containers_total_bytes": self.peak_total,
            "peak_daemon_container_bytes": self.peak_daemon,
            "peak_by_container_bytes": dict(sorted(self.peak_by_container.items())),
            "interval_seconds": self.interval,
        }

    def _run(self) -> None:
        while not self._stop.is_set():
            try:
                usage = self.docker.memory_usage()
                self.peak_total = max(self.peak_total, sum(usage.values()))
                for name, used in usage.items():
                    self.peak_by_container[name] = max(
                        used, self.peak_by_container.get(name, 0)
                    )
                if self.daemon is not None:
                    used = daemon_memory(self.daemon)
                    if used is not None:
                        self.peak_daemon = max(used, self.peak_daemon or 0)
            except (OSError, subprocess.SubprocessError, ValueError, IsolationError):
                pass
            self._stop.wait(self.interval)


def conditions(options: WorkspaceOptions) -> list[str]:
    """What readiness means for this series, for tables that compare series."""
    optional = sorted(probe.name for probe in options.probes if not probe.required)
    return [
        f"ready: {', '.join(probe.name for probe in options.probes if probe.required)}",
        f"optional: {', '.join(optional) or 'none'}",
        f"after up: {options.after_up or 'nothing'}",
    ]


def optional_names(probes: list[Probe]) -> set[str]:
    return {
        probe.name
        for probe in probes
        if not probe.required or probe.kind is ProbeKind.FAIL
    }


def cli_outcome(log: Path) -> dict[str, Any]:
    """The JSON result the devcontainer CLI prints last."""
    for line in reversed(
        log.read_text(encoding="utf-8", errors="replace").splitlines()
    ):
        start = line.find('{"outcome"')
        if start >= 0:
            try:
                return dict(json.loads(line[start:]))
            except json.JSONDecodeError:
                continue
    return {}


def probe_env(checkout: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for name in ENV_FILES:
        values.update(read_env_file(checkout / name))
    return values


def up_command(options: WorkspaceOptions, checkout: Path) -> list[str]:
    command = [options.devcontainer, "up", "--workspace-folder", str(checkout)]
    if options.config:
        command += ["--config", str(checkout / options.config)]
    return command


def start_after_up(docker: Docker, outcome: dict[str, Any], command: str) -> None:
    """Starts a developer command in the dev container, detached, as its user."""
    container = str(outcome["containerId"])
    user = str(outcome.get("remoteUser") or "root")
    folder = str(outcome.get("remoteWorkspaceFolder") or "/")
    script = (
        f"setsid bash -lc {shlex.quote(command)} > {AFTER_UP_LOG} 2>&1 & "
        f"echo $! > {AFTER_UP_PID}"
    )
    docker.run(
        "exec", "--user", user, "--workdir", folder, container, "bash", "-c", script
    )


def stop_after_up(docker: Docker, outcome: dict[str, Any]) -> None:
    container = str(outcome.get("containerId") or "")
    if container:
        docker.run(
            "exec",
            container,
            "bash",
            "-c",
            f"test -f {AFTER_UP_PID} && kill -TERM -- -$(cat {AFTER_UP_PID}); "
            f"rm -f {AFTER_UP_PID}",
            check=False,
        )


def footprint(
    docker: Docker, daemon: str | None, checkout: Path, outcome: dict[str, Any]
) -> dict[str, Any]:
    """Disk used by images, volumes, the Nix store and checkout build outputs."""
    result: dict[str, Any] = {"docker_system_df": docker.disk_usage()}
    container = outcome.get("containerId")
    if container:
        completed = docker.run(
            "exec",
            str(container),
            "du",
            "-sxb",
            "/nix/store",
            check=False,
            timeout=FOOTPRINT_TIMEOUT,
        )
        fields = completed.stdout.split()
        result["nix_store_bytes"] = int(fields[0]) if fields else None
    outputs: dict[str, int | None] = {}
    for relative in CHECKOUT_OUTPUTS:
        path = checkout / relative
        if not path.exists():
            continue
        completed = subprocess.run(
            ["du", "-sb", str(path)],
            capture_output=True,
            text=True,
            check=False,
            timeout=FOOTPRINT_TIMEOUT,
        )
        fields = completed.stdout.split()
        outputs[relative] = int(fields[0]) if fields else None
    result["checkout_outputs_bytes"] = outputs
    if daemon is not None:
        result["daemon_storage_bytes"] = daemon_storage(daemon)
    return result


def measure(
    run: Run,
    options: WorkspaceOptions,
    timer: SampleTimer,
    docker: Docker,
    daemon: str | None,
    checkout: Path,
    temporary: Path | None,
    measure_disk: bool,
) -> None:
    """One timed ``devcontainer up`` until every probe passes; records the sample."""
    address = daemon_address(daemon) if daemon else None
    runner = ProbeRunner(
        docker, options.probes, probe_env(checkout), address, options.poll_interval
    )
    sampler = MemorySampler(docker, daemon, options.stats_interval)
    environment = dict(docker.environment)
    if temporary is not None:
        temporary.mkdir(parents=True, exist_ok=True)
        environment["TMPDIR"] = str(temporary)
    log = run.logs / f"sample-{timer.index}-devcontainer-up.log"
    phases: dict[str, float] = {}
    outcome: dict[str, Any] = {}
    error: str | None = None
    runner.start(timer.monotonic)
    sampler.start()
    cli = BackgroundProcess(
        up_command(options, checkout), cwd=checkout, log=log, env=environment
    )
    interrupted: KeyboardInterrupt | SystemExit | None = None
    try:
        while True:
            if "cli_up" not in phases and not cli.alive():
                phases["cli_up"] = timer.elapsed()
                print(
                    f"bench: devcontainer up done at {phases['cli_up']:.1f}s",
                    file=sys.stderr,
                    flush=True,
                )
                if cli.process.returncode != 0:
                    error = (
                        f"devcontainer up exited {cli.process.returncode}; see {log}"
                    )
                    break
                runner.stack_created.set()
                outcome = cli_outcome(log)
                if options.after_up:
                    start_after_up(docker, outcome, options.after_up)
                    phases["after_up_started"] = timer.elapsed()
            if runner.failed:
                error = f"failure seen by {sorted(runner.failed)}; see container logs"
                break
            if "cli_up" in phases and runner.ready():
                break
            if timer.elapsed() > options.timeout:
                error = f"not ready after {options.timeout:.0f}s: {runner.pending()}"
                break
            time.sleep(0.2)
    except (KeyboardInterrupt, SystemExit) as stop:
        # Record what was observed before an interruption, then re-raise it.
        interrupted = stop
        error = f"interrupted after {timer.elapsed():.0f}s: {runner.pending()} pending"
    finally:
        cli.stop()
        runner.stop()
        memory = sampler.stop()
    phases.update(runner.passed)
    required = [name for name in phases if name not in optional_names(options.probes)]
    ready = max(phases[name] for name in required) if error is None else None
    detail: dict[str, Any] = {
        "memory": memory,
        "cli_outcome": outcome,
        "devcontainer_up_log": str(log),
        "services": docker.names(running_only=True),
        "failures": runner.failed,
        "optional_not_passed": [
            probe.name
            for probe in options.probes
            if not probe.required and probe.name not in runner.passed
        ],
    }
    if error is None and options.footprint and measure_disk:
        detail["footprint"] = footprint(docker, daemon, checkout, outcome)
    if options.after_up and outcome:
        stop_after_up(docker, outcome)
    run.add(
        timer.finish(
            ok=error is None, seconds=ready, phases=phases, detail=detail, error=error
        )
    )
    if interrupted is not None:
        raise interrupted


def stop_stack(docker: Docker) -> list[str]:
    """Stops every running container of the isolated daemon."""
    running = docker.names(running_only=True)
    if running:
        docker.run("stop", "--time", STOP_TIMEOUT_SECONDS, *running, timeout=600)
    return running


def next_index(options: WorkspaceOptions) -> int:
    root = options.sandbox_root
    assert root is not None
    index = 1
    while (root / "envs" / f"{options.dind_prefix}{index}").exists() or (
        Docker(None).inspect(f"{options.dind_prefix}{index}") is not None
    ):
        index += 1
    return index


def cold_sample(run: Run, options: WorkspaceOptions, index: int, owner: str) -> None:
    root = options.sandbox_root
    assert root is not None
    name = f"{options.dind_prefix}{next_index(options)}"
    environment_dir = root / "envs" / name
    checkout = environment_dir / options.copy_name
    dind = Dind(name=name, root=root / "dind" / name, image=options.dind_image)
    created = False
    try:
        copy = materialize_checkout(options.checkout, checkout)
        dind.create([environment_dir], options.empty_mounts, owner)
        created = True
        docker = require_isolated(dind.docker_host)
        run.result.notes.append(
            f"sample {index}: daemon {name}, copy {json.dumps(copy)}"
        )
        for source, reference in options.seeds:
            image = seed_image(docker, source, reference)
            run.result.notes.append(
                f"sample {index}: seeded {reference} from {source} {image}"
            )
        timer = SampleTimer(index)
        measure(
            run, options, timer, docker, name, checkout, dind.root / "cli-tmp", True
        )
    finally:
        if not options.keep:
            if created:
                dind.remove(owner)
            remove_as_root(options.dind_image, [checkout])
            if environment_dir.exists():
                environment_dir.rmdir()
        else:
            run.result.notes.append(
                f"kept daemon {name} (DOCKER_HOST={dind.docker_host}) and {checkout}"
            )


def warm_samples(run: Run, options: WorkspaceOptions) -> None:
    assert options.docker_host is not None
    docker = require_isolated(options.docker_host)
    if not docker.names():
        raise IsolationError(
            f"no containers at {options.docker_host}; create them with --keep"
        )
    schedule = roles(options.samples, options.warmup)
    for index, role in enumerate(schedule, start=1):
        stopped = stop_stack(docker)
        run.result.notes.append(f"sample {index}: stopped {len(stopped)} containers")
        timer = SampleTimer(index, role)
        # Disk use barely changes between restarts; measure it once, at the end.
        last = index == len(schedule)
        measure(
            run, options, timer, docker, options.dind_name, options.checkout, None, last
        )


def run_workspace(options: WorkspaceOptions) -> Path:
    owner = f"step-bench-{new_run_id()}"
    if options.cache is CacheState.COLD:
        if options.sandbox_root is None:
            raise IsolationError("cold samples need --sandbox-root")
        detail = (
            "fresh Docker-in-Docker daemon and checkout copy per sample: no images, "
            "volumes, Nix store, build outputs or node_modules; no network caches"
        )
        services: list[str] = []
    else:
        if options.docker_host is None:
            raise IsolationError(
                "warm samples need --docker-host of an isolated daemon"
            )
        detail = (
            "existing isolated stack: images, volumes, container filesystems (Nix "
            "store) and build outputs reused; all containers stopped before each sample"
        )
        services = require_isolated(options.docker_host).names()
    run = start_run(
        scenario=SCENARIO,
        target=options.target,
        label=options.label,
        cache=options.cache,
        cache_detail=detail,
        checkout=options.checkout,
        output_dir=options.output_dir,
        services=services,
        commands=[" ".join(up_command(options, options.checkout))],
        parameters={
            "probes": [str(probe) for probe in options.probes],
            "after_up": options.after_up,
            "config": options.config,
            "timeout_seconds": options.timeout,
            "poll_interval_seconds": options.poll_interval,
            "dind_image": options.dind_image,
            "docker_host": options.docker_host,
            "seeded_images": [f"{source}={target}" for source, target in options.seeds],
            "conditions": conditions(options),
        },
        extra_tools={"devcontainer": (options.devcontainer, "--version")},
    )
    if options.cache is CacheState.COLD:
        for index in range(1, options.samples + 1):
            cold_sample(run, options, index, owner)
    else:
        warm_samples(run, options)
    return run.finish()
