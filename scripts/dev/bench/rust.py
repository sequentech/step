# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Incremental Rust rebuild time after one edit, with Cargo's unit timings.

Each sample saves the edit with a new marker, then runs one build; with several
builds, each gets its own fresh save so every build starts from exactly one
change. Builds reuse a warm target directory (an untimed warm-up brings it up
to date first), run with ``--timings`` for per-unit durations and link through a
wrapper that times each link. The service containers' cargo-watch builds share
one target directory instead; these direct builds isolate a single consumer.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .common import Run, SampleTimer, new_run_id, roles, start_run
from .edits import EditSpec, MarkerEdit, marker_for
from .process import run_command
from .results import CacheState

SCENARIO = "rust"
LINKER = Path(__file__).with_name("timed_linker.py")
UNIT_DATA = "const UNIT_DATA = "
LONGEST_UNITS = 5
RUST_MARKER = 'std::hint::black_box("{marker}");'

# Statement edits inside function bodies, compiled by the native builds.
RUST_EDITS: dict[str, EditSpec] = {
    "sequent-core": EditSpec(
        path="packages/sequent-core/src/util/date.rs",
        anchor="let local: DateTime<Local> = Local::now();",
        template=RUST_MARKER,
    ),
    "windmill-service": EditSpec(
        path="packages/windmill/src/services/probe.rs",
        anchor="let data = vec![",
        template=RUST_MARKER,
    ),
    "harvest-route": EditSpec(
        path="packages/harvest/src/routes/election_stats.rs",
        anchor="let input = body.into_inner();",
        template=RUST_MARKER,
    ),
}

# What each service container's cargo-watch builds, run from packages/.
BUILDS: dict[str, str] = {
    "harvest": "cargo build --timings -p harvest",
    "windmill": "cargo build --timings -p windmill --bin main",
    "beat": "cargo build --timings -p windmill --bin beat",
}


@dataclass
class RustOptions:
    checkout: Path
    label: str
    edit_name: str
    edit: EditSpec
    builds: dict[str, str]
    target_dir: Path
    samples: int
    warmup: int
    timeout: float
    output_dir: Path


def parse_timings(html: str) -> list[dict[str, Any]]:
    """The unit records Cargo embeds in its ``--timings`` HTML report."""
    start = html.find(UNIT_DATA)
    if start < 0:
        raise ValueError("no UNIT_DATA in the Cargo timings report")
    units, _ = json.JSONDecoder().raw_decode(html, start + len(UNIT_DATA))
    return list(units)


def unit_name(unit: Mapping[str, Any]) -> str:
    return f"{unit['name']}{unit.get('target', '')}".strip()


def unblocked(unit: Mapping[str, Any], kind: str) -> list[int]:
    # Cargo renamed "unlocked" to "unblocked"; accept reports from either.
    return list(unit.get(f"unblocked_{kind}", unit.get(f"unlocked_{kind}", [])))


def compiled(units: Sequence[Mapping[str, Any]]) -> list[Mapping[str, Any]]:
    """Units Cargo actually ran; fresh units are listed with no duration."""
    return [unit for unit in units if unit["duration"] > 0]


def critical_path(units: Sequence[Mapping[str, Any]]) -> list[Mapping[str, Any]]:
    """The chain of units that ends last, following each unit's latest unblocker."""
    units = compiled(units)
    if not units:
        return []
    by_index = {unit["i"]: unit for unit in units}
    unlockers: dict[int, list[tuple[float, int]]] = {}
    for unit in units:
        finished = unit["start"] + unit["duration"]
        metadata = unit["start"] + (unit.get("rmeta_time") or unit["duration"])
        for index in unblocked(unit, "units"):
            unlockers.setdefault(index, []).append((finished, unit["i"]))
        for index in unblocked(unit, "rmeta_units"):
            unlockers.setdefault(index, []).append((metadata, unit["i"]))
    current = max(units, key=lambda unit: unit["start"] + unit["duration"])
    path = [current]
    while unlockers.get(current["i"]):
        _, previous = max(unlockers[current["i"]])
        current = by_index[previous]
        path.append(current)
    return list(reversed(path))


def timings_summary(units: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    units = compiled(units)
    path = critical_path(units)
    longest = sorted(units, key=lambda unit: unit["duration"], reverse=True)
    return {
        "units_compiled": len(units),
        "unit_seconds_total": round(sum(unit["duration"] for unit in units), 2),
        "critical_path": [
            {
                "unit": unit_name(unit),
                "start": unit["start"],
                "seconds": unit["duration"],
            }
            for unit in path
        ],
        "critical_path_seconds": round(
            path[-1]["start"] + path[-1]["duration"] - path[0]["start"], 2
        )
        if path
        else 0.0,
        "longest_units": [
            {"unit": unit_name(unit), "seconds": unit["duration"]}
            for unit in longest[:LONGEST_UNITS]
        ],
    }


def newest_report(target_dir: Path, since: float) -> Path | None:
    reports = sorted(
        (target_dir / "cargo-timings").glob("cargo-timing-*.html"),
        key=lambda path: path.stat().st_mtime,
    )
    return reports[-1] if reports and reports[-1].stat().st_mtime >= since else None


def host_triple() -> str:
    output = subprocess.run(
        ["rustc", "-vV"], capture_output=True, text=True, check=True
    ).stdout
    for line in output.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    raise RuntimeError("rustc -vV reported no host triple")


def build_environment(target_dir: Path, link_log: Path) -> dict[str, str]:
    """The developer shell's environment with the target dir and timed linker."""
    linker = shutil.which("cc")
    if linker is None:
        raise RuntimeError("no cc linker on PATH")
    triple = host_triple().upper().replace("-", "_")
    environment = dict(os.environ)
    environment.update(
        CARGO_TARGET_DIR=str(target_dir),
        CARGO_TERM_COLOR="never",
        STEP_BENCH_REAL_LINKER=linker,
        STEP_BENCH_LINK_LOG=str(link_log),
    )
    environment[f"CARGO_TARGET_{triple}_LINKER"] = str(LINKER)
    return environment


def links_since(link_log: Path, offset: int) -> list[dict[str, Any]]:
    if not link_log.exists():
        return []
    with link_log.open("rb") as handle:
        handle.seek(offset)
        return [
            json.loads(line) for line in handle.read().decode().splitlines() if line
        ]


def run_rust(options: RustOptions) -> list[Path]:
    edit = MarkerEdit(options.checkout, options.edit)
    run_id = new_run_id()
    log_dir = options.output_dir / SCENARIO / "logs" / f"{options.edit_name}-{run_id}"
    log_dir.mkdir(parents=True, exist_ok=True)
    link_log = log_dir / "links.jsonl"
    environment = build_environment(options.target_dir, link_log)
    runs = {
        name: start_run(
            scenario=SCENARIO,
            target=f"{options.edit_name}@{name}",
            label=options.label,
            cache=CacheState.WARM,
            cache_detail=f"warm target directory {options.target_dir}; "
            f"{options.warmup} untimed build(s) with the edit first",
            checkout=options.checkout,
            output_dir=options.output_dir,
            services=[],
            commands=[f"edit {options.edit.path}", f"cd packages && {command}"],
            parameters={
                "edit": options.edit.to_dict(),
                "build": command,
                "target_dir": str(options.target_dir),
                "rustflags": os.environ.get("RUSTFLAGS"),
                "linker": f"{LINKER} -> {environment['STEP_BENCH_REAL_LINKER']}",
                "marker_run_id": run_id,
                "conditions": [
                    f"edit: {options.edit_name}",
                    f"RUSTFLAGS={os.environ.get('RUSTFLAGS', '')}",
                    f"CARGO_BUILD_JOBS={os.environ.get('CARGO_BUILD_JOBS', 'default')}",
                    "one fresh save before every build",
                ],
            },
        )
        for name, command in options.builds.items()
    }
    index = 0
    try:
        for role in roles(options.samples, options.warmup):
            for name, command in options.builds.items():
                index += 1
                measure(
                    options,
                    edit,
                    runs[name],
                    environment,
                    link_log,
                    log_dir,
                    command,
                    SampleTimer(index, role),
                    marker_for(run_id, index),
                )
    finally:
        edit.restore()
    return [run.finish() for run in runs.values()]


def measure(
    options: RustOptions,
    edit: MarkerEdit,
    run: Run,
    environment: dict[str, str],
    link_log: Path,
    log_dir: Path,
    command: str,
    timer: SampleTimer,
    marker: str,
) -> None:
    offset = link_log.stat().st_size if link_log.exists() else 0
    saved = edit.apply(marker)
    result = run_command(
        command,
        cwd=options.checkout / "packages",
        log=log_dir / "build.log",
        env=environment,
        timeout=options.timeout,
    )
    report = newest_report(options.target_dir, saved)
    detail: dict[str, Any] = {
        "marker": marker,
        "cargo_timings": str(report) if report else None,
    }
    if report is not None:
        detail.update(
            timings_summary(parse_timings(report.read_text(encoding="utf-8")))
        )
    links = links_since(link_log, offset)
    detail["links"] = [
        {
            "output": Path(link["output"] or "?").name,
            "seconds": round(link["seconds"], 2),
        }
        for link in links
    ]
    phases = {"link": sum(link["seconds"] for link in links)} if links else {}
    run.add(
        timer.finish(
            ok=result.ok,
            seconds=result.seconds,
            phases=phases,
            detail=detail,
            error=None
            if result.ok
            else f"exit {result.returncode}: {result.output_tail[-800:]}",
        )
    )
