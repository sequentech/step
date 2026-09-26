# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline toolchain readiness of an already built native development image.

Fresh means a new Nix volume seeded from the local image, not a cold image pull
or filesystem cache. Warm starts create new containers sharing the final store.
No checkout, credentials, Docker socket or application dependencies are mounted.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shlex
import shutil
import signal
import subprocess
import time
import uuid
from pathlib import Path

from .bench.common import Run, SampleTimer, start_run
from .bench.isolation import Docker, IsolationError
from .bench.process import run_command
from .bench.results import CacheState, SampleRole, summarize_samples, utc_now
from .prebuild import IMAGE_LABEL, fingerprint

OWNER_LABEL = "io.sequent.prebuild-smoke-owner"
READY_PREFIX = "STEP_PREBUILD_READY="
SAMPLE_TIMEOUT = 600
WARM_RESERVE = 180
CLEANUP_TIMEOUT = 90
TOOLS = ("node", "yarn", "rustc", "cargo", "wasm-pack", "wasm-bindgen")
READINESS = """\
import json, pathlib, subprocess, sys
tools = {}
for name in ('node', 'yarn', 'rustc', 'cargo', 'wasm-pack', 'wasm-bindgen'):
    result = subprocess.run([name, '--version'], check=True, capture_output=True,
                            text=True, timeout=15)
    tools[name] = (result.stdout or result.stderr).strip()
    if not tools[name]:
        raise RuntimeError(name + ' returned no version')
target = subprocess.check_output(
    ['rustc', '--print', 'target-libdir', '--target', 'wasm32-unknown-unknown'],
    text=True, timeout=15).strip()
if not list(pathlib.Path(target).glob('libstd-*.rlib')):
    raise RuntimeError('wasm32-unknown-unknown standard library is missing')
print('STEP_PREBUILD_READY=' + json.dumps({
    'tools': tools, 'python': sys.version.split()[0], 'wasm_target_ready': True
}), flush=True)
"""


class SmokeError(RuntimeError):
    pass


class OwnershipError(SmokeError):
    pass


def record_failure(output: Path, error: BaseException) -> None:
    path = output / "failure.log"
    message = str(error) + "\n"
    if not path.exists() or message not in path.read_text():
        with path.open("a") as handle:
            handle.write(message)


def local_image(docker: Docker, image: str, key: str) -> dict:
    entry = json.loads(docker.run("image", "inspect", image).stdout)[0]
    native = {"aarch64": "arm64", "x86_64": "amd64"}.get(
        platform.machine(), platform.machine()
    )
    if (
        entry["Os"] != "linux"
        or entry["Architecture"] != native
        or (entry["Config"].get("Labels") or {}).get(IMAGE_LABEL) != key
    ):
        raise SmokeError(
            "image must match this checkout's fingerprint and native Linux architecture"
        )
    return {name: entry[name] for name in ("Id", "Architecture", "Os", "Size")}


def readiness(log: Path) -> dict:
    for line in reversed(log.read_text().splitlines()):
        if line.startswith(READY_PREFIX):
            result = json.loads(line[len(READY_PREFIX) :])
            if not isinstance(result, dict):
                break
            versions = result.get("tools", {})
            if (
                isinstance(versions, dict)
                and result.get("wasm_target_ready") is True
                and all(
                    isinstance(versions.get(name), str) and versions[name]
                    for name in TOOLS
                )
            ):
                return result
            break
    raise SmokeError("container exited without complete toolchain readiness evidence")


class OwnedResources:
    def __init__(self, docker: Docker, log: Path):
        self.docker = docker
        self.log = log
        self.owner = uuid.uuid4().hex
        self.containers: set[str] = set()
        self.volumes: set[str] = set()
        self.pending: set[str] = set()
        self.failed: set[str] = set()
        self.record()

    def record(self) -> None:
        (self.log.parent / "resources.json").write_text(
            json.dumps(
                {
                    "owner": self.owner,
                    "containers": sorted(self.containers),
                    "volumes": sorted(self.volumes),
                    "pending_creates": sorted(self.pending),
                },
                indent=2,
            )
            + "\n"
        )

    def volume(self) -> str:
        name = f"step-prebuild-{self.owner}-{len(self.volumes)}-{uuid.uuid4().hex[:8]}"
        if self.docker.run("volume", "inspect", name, check=False).returncode == 0:
            raise SmokeError(f"volume already exists: {name}")
        # Register before creating, so interruption after creation still cleans it.
        self.volumes.add(name)
        self.record()
        self.docker.run(
            "volume", "create", "--label", f"{OWNER_LABEL}={self.owner}", name
        )
        return name

    def container(self) -> str:
        name = f"step-prebuild-{self.owner}-{uuid.uuid4().hex[:8]}"
        self.containers.add(name)
        self.pending.add(name)
        self.record()
        return name

    def created(self, name: str) -> None:
        self.pending.remove(name)
        self.record()

    def inspect(self, kind: str, name: str) -> dict | None:
        response = self.docker.run(kind, "inspect", name, check=False, timeout=5)
        entry = json.loads(response.stdout)[0] if response.returncode == 0 else None
        diagnostic = {
            "at": utc_now(),
            "kind": kind,
            "name": name,
            "returncode": response.returncode,
            "stderr": response.stderr,
        }
        if entry:
            diagnostic["resource"] = {
                key: entry.get(key)
                for key in ("Id", "Name", "State", "Mounts", "Labels")
            }
            diagnostic["owner"] = (entry.get("Config", {}).get("Labels") or {}).get(
                OWNER_LABEL
            )
        with (self.log.parent / "inspections.jsonl").open("a") as handle:
            handle.write(json.dumps(diagnostic) + "\n")
        if response.returncode and "no such" not in response.stderr.lower():
            raise SmokeError(
                f"cannot inspect owned {kind} {name}: {response.stderr.strip()}"
            )
        return entry

    def diagnostics(self, name: str) -> None:
        """Keep container and daemon output before removing a failed container."""
        directory = self.log.parent / "diagnostics"
        directory.mkdir(exist_ok=True)
        for command, suffix in (
            (["logs", "--timestamps", name], "container.log"),
            (
                [
                    "cp",
                    f"{name}:/tmp/nix-daemon.log",
                    str(directory / f"{name}-nix-daemon.log"),
                ],
                "copy.log",
            ),
        ):
            try:
                result = self.docker.run(*command, check=False, timeout=5)
                (directory / f"{name}-{suffix}").write_text(
                    result.stdout + result.stderr
                )
            except (OSError, subprocess.SubprocessError) as error:
                (directory / f"{name}-{suffix}").write_text(str(error) + "\n")

    def remove(self, kind: str, name: str) -> bool:
        tracked = self.containers if kind == "container" else self.volumes
        if name not in tracked:
            raise OwnershipError(f"refusing untracked {kind}: {name}")
        entry = self.inspect(kind, name)
        if entry:
            labels = (
                entry.get("Config", {}).get("Labels")
                if kind == "container"
                else entry.get("Labels")
            )
            if (labels or {}).get(OWNER_LABEL) != self.owner:
                raise OwnershipError(f"refusing {kind} with another owner: {name}")
            if kind == "container" and name in self.failed:
                self.diagnostics(name)
                self.failed.remove(name)
            arguments = (
                [kind, "rm", "--force", name]
                if kind == "container"
                else [kind, "rm", name]
            )
            self.docker.run(*arguments, timeout=10)
        elif kind == "container" and name in self.pending:
            # Killing the CLI does not cancel the daemon's create/copy request.
            # Keep the exact name until it appears and can be removed by owner.
            return False
        tracked.remove(name)
        self.pending.discard(name)
        self.record()
        return True

    def cleanup(self) -> None:
        deadline = time.monotonic() + CLEANUP_TIMEOUT
        errors: dict[str, str] = {}
        refused = False
        while self.containers or self.volumes:
            # Do not remove a volume while its pending create could still attach it.
            kinds = (
                [("container", self.containers)]
                if self.containers
                else [("volume", self.volumes)]
            )
            for kind, names in kinds:
                for name in sorted(names):
                    try:
                        if self.remove(kind, name):
                            errors.pop(name, None)
                        else:
                            errors[name] = (
                                f"pending create is not yet inspectable: {name}"
                            )
                    except OwnershipError as error:
                        errors[name] = str(error)
                        refused = True
                    except (
                        OSError,
                        ValueError,
                        subprocess.SubprocessError,
                        IsolationError,
                        SmokeError,
                    ) as error:
                        errors[name] = str(error)
            if refused or time.monotonic() >= deadline:
                break
            if self.containers or self.volumes:
                time.sleep(0.5)
        if self.containers or self.volumes:
            self.log.write_text("\n".join(errors.values()) + "\n")
            raise SmokeError("owned-resource cleanup failed; see cleanup.log")


def command(image: str, container: str, volume: str, owner: str) -> list[str]:
    return [
        "docker",
        "create",
        "--pull",
        "never",
        "--name",
        container,
        "--label",
        f"{OWNER_LABEL}={owner}",
        "--network",
        "none",
        "--mount",
        f"type=volume,source={volume},target=/nix",
        "--tmpfs",
        "/nix/var/nix/daemon-socket:mode=0755",
        "--user",
        "vscode",
        "--workdir",
        "/opt/step-env",
        "--entrypoint",
        "/nix-entrypoint.sh",
        image,
        "bash",
        "-lc",
        shlex.join(["devenv", "shell", "python", "--", "-c", READINESS]),
    ]


def sample(
    run: Run,
    resources: OwnedResources,
    image: str,
    volume: str,
    index: int,
    role: SampleRole,
    root: Path,
    timeout: float,
) -> None:
    container = resources.container()
    invocation = command(image, container, volume, resources.owner)
    logs = run.logs / f"{role.value}-{index}"
    timer = SampleTimer(index, role)
    phases = {}
    detail = {
        "container": container,
        "nix_volume": volume,
        "create_command": shlex.join(invocation),
        "phase": "create",
    }
    try:
        created = run_command(
            invocation,
            cwd=root,
            log=logs.with_suffix(".create.log"),
            env=resources.docker.environment,
            timeout=timeout,
        )
        phases["create"] = created.seconds
        if not created.ok:
            raise SmokeError(
                f"container create failed ({created.returncode}) after "
                f"{created.seconds:.3f} s; see {logs.name}.create.log"
            )
        resources.created(container)
        detail["phase"] = "toolchain"
        remaining = timeout - timer.elapsed()
        if remaining <= 0:
            raise SmokeError("sample time budget exhausted after container creation")
        started = run_command(
            ["docker", "start", "--attach", container],
            cwd=root,
            log=logs.with_suffix(".start.log"),
            env=resources.docker.environment,
            timeout=remaining,
        )
        phases["toolchain"] = started.seconds
        if not started.ok:
            raise SmokeError(
                f"toolchain readiness failed ({started.returncode}); "
                f"see {logs.name}.start.log"
            )
        detail.update(readiness(logs.with_suffix(".start.log")))
        run.add(
            timer.finish(ok=True, seconds=timer.elapsed(), phases=phases, detail=detail)
        )
    except (OSError, ValueError, subprocess.SubprocessError, SmokeError) as error:
        resources.failed.add(container)
        # A pending create may have no logs yet; cleanup captures its output as
        # soon as the exact owned container becomes inspectable.
        run.add(
            timer.finish(
                ok=False, seconds=None, phases=phases, detail=detail, error=str(error)
            )
        )
        raise
    resources.remove("container", container)


def write_summary(runs: list[Run], path: Path) -> None:
    lines = [
        "## Offline prebuild toolchain readiness",
        "",
        "Local native image; networking disabled; no application or service startup.",
        "",
        "| Nix volume | Successful samples | Median (s) | Range (s) | Failures |",
        "| --- | ---: | ---: | --- | ---: |",
    ]
    for run in runs:
        summary = summarize_samples([sample.to_dict() for sample in run.result.samples])
        lines.append(
            f"| {run.result.cache.value} | {summary['n']} | {summary['median']} | "
            f"{summary['min']} - {summary['max']} | {summary['failed']} |"
        )
        lines.extend(f"\n{note}" for note in run.result.notes)
    lines += [
        "",
        "Fresh starts include copying the image's Nix store into a new volume. "
        "Warm starts use new containers and the final seeded volume; "
        "warmup is excluded. "
        "Image build/pull, empty-volume creation and cleanup are outside the samples.",
    ]
    path.write_text("\n".join(lines) + "\n")


def measure(
    root: Path,
    output: Path,
    docker: Docker,
    image: dict,
    fresh: int = 3,
    warm: int = 10,
    budget: float = 720,
) -> list[Run]:
    if fresh < 1 or warm < 1 or budget <= 0:
        raise ValueError("need positive fresh/warm counts and a positive time budget")
    output.mkdir(parents=True, exist_ok=True)
    deadline = time.monotonic() + budget
    runs = [
        start_run(
            scenario="prebuild-startup",
            target=image["Architecture"],
            label="offline",
            cache=cache,
            cache_detail=(
                "New Nix volume seeded from local image; filesystem cache retained."
                if cache is CacheState.COLD
                else "New containers reuse the final Nix volume; one excluded warmup."
            ),
            checkout=root,
            output_dir=output,
            services=[],
            commands=[
                "docker create --pull never --network none <local image>; "
                "docker start --attach <owned container>"
            ],
            parameters={
                "image": image,
                "requested_fresh": fresh,
                "requested_warm": warm,
                "budget_seconds": budget,
                "scope": "source-free toolchains only",
            },
        )
        for cache in (CacheState.COLD, CacheState.WARM)
    ]
    resources = OwnedResources(docker, output / "cleanup.log")
    cold, hot = runs
    volume = None
    primary = None
    try:
        # Only one copied store exists at a time, including during warm samples.
        for index in range(1, fresh + 1):
            remaining = deadline - time.monotonic()
            expected = (
                (fresh - index + 1) * cold.result.samples[-1].seconds
                if index > 1
                else 0
            )
            if index > 1 and remaining < expected + WARM_RESERVE:
                cold.result.notes.append(
                    f"Fresh series limited to {index - 1}/{fresh}: "
                    "observed copy time leaves insufficient budget for more "
                    "fresh starts and warm samples."
                )
                break
            if volume:
                resources.remove("volume", volume)
            volume = resources.volume()
            sample(
                cold,
                resources,
                image["Id"],
                volume,
                index,
                SampleRole.MEASURED,
                root,
                min(SAMPLE_TIMEOUT, max(1, remaining)),
            )
        for index in range(warm + 1):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise SmokeError(
                    f"time budget exhausted after {max(0, index - 1)}/{warm} "
                    "warm samples"
                )
            sample(
                hot,
                resources,
                image["Id"],
                volume,
                index,
                SampleRole.WARMUP if index == 0 else SampleRole.MEASURED,
                root,
                min(SAMPLE_TIMEOUT, remaining),
            )
    except (
        OSError,
        ValueError,
        subprocess.SubprocessError,
        IsolationError,
        SmokeError,
    ) as error:
        primary = error
        record_failure(output, error)
        cold.result.notes.append(f"Incomplete smoke: {error}")
        raise
    finally:
        try:
            try:
                resources.cleanup()
            except SmokeError as error:
                record_failure(output, error)
                cold.result.notes.append(str(error))
                if primary is None:
                    raise
        finally:
            for run in runs:
                run.finish()
            write_summary(runs, output / "summary.md")
    return runs


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument(
        "--root", default=Path(__file__).resolve().parents[2], type=Path
    )
    parser.add_argument("--docker-host", default=os.environ.get("DOCKER_HOST"))
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    docker = Docker(args.docker_host)

    def interrupted(_signum, _frame):
        raise SmokeError("interrupted")

    signal.signal(signal.SIGTERM, interrupted)
    try:
        image = local_image(docker, args.image, fingerprint(args.root))
        # A fresh volume duplicates /nix. Image size is a conservative upper bound.
        directory = docker.run("info", "--format", "{{.DockerRootDir}}").stdout.strip()
        if not args.docker_host and directory and Path(directory).exists():
            available = shutil.disk_usage(directory).free
            if available < image["Size"] + 1024**3:
                raise SmokeError(
                    f"fresh Nix volume needs up to {image['Size']} bytes plus "
                    f"1 GiB headroom; {available} bytes free"
                )
        measure(args.root.resolve(), args.output_dir, docker, image)
    except (
        OSError,
        ValueError,
        KeyError,
        IndexError,
        subprocess.SubprocessError,
        IsolationError,
        SmokeError,
    ) as error:
        record_failure(args.output_dir, error)
        print(f"Prebuild smoke failed: {error}")
        raise SystemExit(1) from error


if __name__ == "__main__":
    main()
