# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Pieces shared by the scenarios: result setup, sample timing and logs."""

from __future__ import annotations

import os
import secrets
import time
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .environment import git_state, harness_info, host_info, tool_versions
from .results import (
    CacheState,
    Result,
    Sample,
    SampleRole,
    load_average,
    result_path,
    utc_now,
    write_result,
)

DEFAULT_OUTPUT_DIR = Path.home() / ".cache" / "step-bench" / "results"


def new_run_id() -> str:
    """Short random identifier of one benchmark run."""
    return secrets.token_hex(3)


def default_output_dir() -> Path:
    configured = os.environ.get("STEP_BENCH_OUTPUT_DIR")
    return Path(configured) if configured else DEFAULT_OUTPUT_DIR


@dataclass
class Run:
    """A result being recorded, rewritten after every sample."""

    result: Result
    path: Path

    @property
    def logs(self) -> Path:
        directory = self.path.parent / "logs" / self.path.stem
        directory.mkdir(parents=True, exist_ok=True)
        return directory

    def add(self, sample: Sample) -> None:
        self.result.samples.append(sample)
        write_result(self.path, self.result)

    def finish(self) -> Path:
        self.result.finished_at = utc_now()
        write_result(self.path, self.result)
        return self.path


def start_run(
    *,
    scenario: str,
    target: str,
    label: str,
    cache: CacheState,
    cache_detail: str,
    checkout: Path | None,
    output_dir: Path,
    services: Sequence[str],
    commands: Sequence[str],
    parameters: Mapping[str, Any],
    tools_env: Mapping[str, str] | None = None,
    extra_tools: Mapping[str, Sequence[str]] | None = None,
) -> Run:
    result = Result(
        scenario=scenario,
        target=target,
        label=label,
        cache=cache,
        cache_detail=cache_detail,
        checkout=git_state(checkout) if checkout is not None else {},
        harness=harness_info(),
        host=host_info(),
        tools=tool_versions(extra_tools, env=tools_env),
        services=list(services),
        commands=list(commands),
        parameters=dict(parameters),
    )
    run = Run(result, result_path(output_dir, result))
    write_result(run.path, result)
    return run


class SampleTimer:
    """Wall-clock bounds and machine load around one sample."""

    def __init__(self, index: int, role: SampleRole = SampleRole.MEASURED) -> None:
        self.index = index
        self.role = role
        self.load_before = load_average()
        self.started_at = utc_now()
        self.monotonic = time.monotonic()
        self.epoch = time.time()

    def elapsed(self) -> float:
        return time.monotonic() - self.monotonic

    def finish(
        self,
        *,
        ok: bool,
        seconds: float | None,
        phases: Mapping[str, float] | None = None,
        detail: Mapping[str, Any] | None = None,
        error: str | None = None,
    ) -> Sample:
        return Sample(
            index=self.index,
            role=self.role,
            ok=ok,
            seconds=seconds if ok else None,
            started_at=self.started_at,
            finished_at=utc_now(),
            load_before=self.load_before,
            load_after=load_average(),
            phases=dict(phases or {}),
            detail=dict(detail or {}),
            error=error,
        )


def roles(samples: int, warmup: int) -> list[SampleRole]:
    """Untimed warm-up iterations first, then the measured ones."""
    if samples < 1 or warmup < 0:
        raise ValueError("need at least one measured sample and a non-negative warm-up")
    return [SampleRole.WARMUP] * warmup + [SampleRole.MEASURED] * samples
