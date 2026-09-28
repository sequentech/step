# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Result files: one JSON document per measured series."""

from __future__ import annotations

import json
import os
import re
from collections.abc import Iterable
from dataclasses import dataclass, field
from datetime import UTC, datetime
from enum import Enum
from pathlib import Path
from typing import Any

from .stats import describe

SCHEMA = "step-bench/1"
LABEL_PATTERN = re.compile(r"^[a-z0-9][a-z0-9._-]*$")


class CacheState(Enum):
    COLD = "cold"
    WARM = "warm"
    # Hosted CI: runner images and Actions caches are as GitHub provides them.
    UNCONTROLLED = "uncontrolled"


class SampleRole(Enum):
    """Only measured samples enter the summary; the others are kept as evidence."""

    MEASURED = "measured"
    WARMUP = "warmup"
    REVERT = "revert"


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="milliseconds")


def load_average() -> list[float]:
    return [round(value, 2) for value in os.getloadavg()]


def validate_label(label: str) -> str:
    if not LABEL_PATTERN.match(label):
        raise ValueError(
            f"invalid label {label!r}: use lower-case letters, digits, '.', '_' or '-'"
        )
    return label


@dataclass
class Sample:
    index: int
    role: SampleRole
    ok: bool
    seconds: float | None
    started_at: str
    finished_at: str
    load_before: list[float]
    load_after: list[float]
    phases: dict[str, float] = field(default_factory=dict)
    detail: dict[str, Any] = field(default_factory=dict)
    error: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "index": self.index,
            "role": self.role.value,
            "ok": self.ok,
            "seconds": None if self.seconds is None else round(self.seconds, 3),
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "load_before": self.load_before,
            "load_after": self.load_after,
            "phases": {name: round(value, 3) for name, value in self.phases.items()},
            "detail": self.detail,
            "error": self.error,
        }


def summarize_samples(samples: Iterable[dict[str, Any]]) -> dict[str, Any]:
    """Summary of the measured, successful samples of a serialized result."""
    measured = [s for s in samples if s.get("role") == SampleRole.MEASURED.value]
    ok = [s for s in measured if s.get("ok") and s.get("seconds") is not None]
    summary: dict[str, Any] = describe([s["seconds"] for s in ok])
    summary["failed"] = len(measured) - len(ok)
    phase_names = sorted({name for s in ok for name in s.get("phases", {})})
    summary["phases"] = {
        name: describe([s["phases"][name] for s in ok if name in s["phases"]])
        for name in phase_names
    }
    summary["load_1m"] = describe(
        [s["load_before"][0] for s in ok if s.get("load_before")]
    )
    return summary


@dataclass
class Result:
    scenario: str
    target: str
    label: str
    cache: CacheState
    cache_detail: str
    checkout: dict[str, Any]
    harness: dict[str, Any]
    host: dict[str, Any]
    tools: dict[str, str | None]
    services: list[str]
    commands: list[str]
    parameters: dict[str, Any]
    started_at: str = field(default_factory=utc_now)
    finished_at: str | None = None
    samples: list[Sample] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        samples = [sample.to_dict() for sample in self.samples]
        return {
            "schema": SCHEMA,
            "scenario": self.scenario,
            "target": self.target,
            "label": self.label,
            "cache": self.cache.value,
            "cache_detail": self.cache_detail,
            "checkout": self.checkout,
            "harness": self.harness,
            "host": self.host,
            "tools": self.tools,
            "services": self.services,
            "commands": self.commands,
            "parameters": self.parameters,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "notes": self.notes,
            "summary": summarize_samples(samples),
            "samples": samples,
        }


def result_path(output_dir: Path, result: Result) -> Path:
    stamp = result.started_at.replace("-", "").replace(":", "")[:15] + "Z"
    name = f"{result.target}--{result.cache.value}--{result.label}--{stamp}.json"
    return output_dir / result.scenario / name


def write_result(path: Path, result: Result) -> None:
    """Rewrites the file atomically, so an interrupted run keeps its samples."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".json.tmp")
    temporary.write_text(
        json.dumps(result.to_dict(), indent=2) + "\n", encoding="utf-8"
    )
    temporary.replace(path)


def read_result(path: Path) -> dict[str, Any]:
    document = json.loads(path.read_text(encoding="utf-8"))
    if document.get("schema") != SCHEMA:
        raise ValueError(f"{path}: not a {SCHEMA} result")
    return document
