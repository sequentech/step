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
from .bench.results import CacheState, SampleRole, summarize_samples
from .prebuild import IMAGE_LABEL, fingerprint

OWNER_LABEL = "io.sequent.prebuild-smoke-owner"
READY_PREFIX = "STEP_PREBUILD_READY="
SAMPLE_TIMEOUT = 180
WARM_RESERVE = 180
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
        self.record()

    def record(self) -> None:
        (self.log.parent / "resources.json").write_text(
            json.dumps(
                {
                    "owner": self.owner,
                    "containers": sorted(self.containers),
                    "volumes": sorted(self.volumes),
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
        self.record()
        return name

    def remove(self, kind: str, name: str) -> None:
        tracked = self.containers if kind == "container" else self.volumes
        if name not in tracked:
            raise SmokeError(f"refusing untracked {kind}: {name}")
        response = self.docker.run(kind, "inspect", name, check=False, timeout=30)
        if response.returncode == 0:
            entry = json.loads(response.stdout)[0]
            labels = (
                entry.get("Config", {}).get("Labels")
                if kind == "container"
                else entry.get("Labels")
            )
            if (labels or {}).get(OWNER_LABEL) != self.owner:
                raise SmokeError(f"refusing {kind} with another owner: {name}")
            arguments = (
                [kind, "rm", "--force", name]
                if kind == "container"
                else [kind, "rm", name]
            )
            self.docker.run(*arguments, timeout=60)
        elif "no such" not in response.stderr.lower():
            raise SmokeError(
                f"cannot inspect owned {kind} {name}: {response.stderr.strip()}"
            )
        tracked.remove(name)
        self.record()

    def cleanup(self) -> None:
        errors = []
        for kind, names in (("container", self.containers), ("volume", self.volumes)):
            for name in sorted(names):
                try:
                    self.remove(kind, name)
                except (
                    OSError,
                    ValueError,
                    subprocess.SubprocessError,
                    IsolationError,
                    SmokeError,
                ) as error:
                    errors.append(str(error))
        if errors:
            self.log.write_text("\n".join(errors) + "\n")
            raise SmokeError("owned-resource cleanup failed; see cleanup.log")


def command(image: str, container: str, volume: str, owner: str) -> list[str]:
    return [
        "docker",
        "run",
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
    log = run.logs / f"{role.value}-{index}.log"
    timer = SampleTimer(index, role)
    detail = {
        "container": container,
        "nix_volume": volume,
        "command": shlex.join(invocation),
    }
    try:
        result = run_command(
            invocation,
            cwd=root,
            log=log,
            env=resources.docker.environment,
            timeout=timeout,
        )
        if not result.ok:
            raise SmokeError(
                f"container readiness failed ({result.returncode}); see {log.name}"
            )
        detail.update(readiness(log))
        run.add(timer.finish(ok=True, seconds=result.seconds, detail=detail))
    except (OSError, ValueError, subprocess.SubprocessError, SmokeError) as error:
        run.add(timer.finish(ok=False, seconds=None, detail=detail, error=str(error)))
        raise
    finally:
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
    budget: float = 600,
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
                "docker run --pull never --network none "
                "<local image> <toolchain readiness>"
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
    try:
        # Only one copied store exists at a time, including during warm samples.
        for index in range(1, fresh + 1):
            remaining = deadline - time.monotonic()
            if index > 1 and remaining < cold.result.samples[-1].seconds + WARM_RESERVE:
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
        cold.result.notes.append(f"Incomplete smoke: {error}")
        raise
    finally:
        try:
            resources.cleanup()
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
        (args.output_dir / "failure.log").write_text(str(error) + "\n")
        print(f"Prebuild smoke failed: {error}")
        raise SystemExit(1) from error


if __name__ == "__main__":
    main()
