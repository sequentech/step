# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Save-to-result time of one focused test command after a source edit.

Each sample saves the tested source with a new marker, so caches keyed on file
content or modification time must redo that file, then runs the suite's focused
command to completion. An untimed warm-up run brings compilers and caches up to
date first. Rust suites use the developer's per-checkout target directory.
"""

from __future__ import annotations

import os
from collections.abc import Mapping
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path

from .common import SampleTimer, new_run_id, roles, start_run
from .edits import EditSpec, MarkerEdit, marker_for
from .process import run_command
from .results import CacheState
from .rust import RUST_EDITS

SCENARIO = "test"
TS_MARKER = "// {marker}"
# Synthetic configuration that harvest's SQL generation tests read; no database.
HARVEST_TEST_ENV = {
    "KEYCLOAK_DB__HOST": "127.0.0.1",
    "HASURA_DB__HOST": "127.0.0.1",
    "LOW_SQL_LIMIT": "1000",
    "DEFAULT_SQL_LIMIT": "20",
    "DEFAULT_SQL_BATCH_SIZE": "1000",
}


class Runner(Enum):
    JEST = "jest"
    VITEST = "vitest"
    CARGO = "cargo"


@dataclass(frozen=True)
class Suite:
    """A focused command, run from ``cwd`` (relative to the checkout)."""

    runner: Runner
    edit: EditSpec
    cwd: str
    command: str
    env: Mapping[str, str] = field(default_factory=dict)


SUITES: dict[str, Suite] = {
    "jest-voting": Suite(
        runner=Runner.JEST,
        edit=EditSpec(
            "packages/voting-portal/src/components/StartActions/StartActions.tsx",
            TS_MARKER,
        ),
        cwd="packages/voting-portal",
        command="yarn test src/components/StartActions/StartActions.test.tsx",
    ),
    "jest-ui-essentials": Suite(
        runner=Runner.JEST,
        edit=EditSpec(
            "packages/ui-essentials/src/components/WarnBox/WarnBox.tsx", TS_MARKER
        ),
        cwd="packages",
        command="yarn --cwd ui-essentials jest src/components/WarnBox/WarnBox.test.tsx "
        "--runInBand",
    ),
    "vitest-header-story": Suite(
        runner=Runner.VITEST,
        edit=EditSpec(
            "packages/ui-essentials/src/components/Header/Header.tsx", TS_MARKER
        ),
        cwd="packages/ui-essentials",
        command="yarn test:stories "
        "src/components/Header/__stories__/Header.stories.tsx",
    ),
    "cargo-sequent-core": Suite(
        runner=Runner.CARGO,
        edit=RUST_EDITS["sequent-core"],
        cwd="packages",
        command="cargo test --locked -p sequent-core --test sqlite_feature_boundaries "
        "--features default_features,keycloak,sqlite",
    ),
    "cargo-harvest": Suite(
        runner=Runner.CARGO,
        edit=RUST_EDITS["harvest-route"],
        cwd="packages",
        command="cargo test -p harvest --locked --offline --bin harvest -- "
        "request_boundaries",
        env=HARVEST_TEST_ENV,
    ),
}


@dataclass
class TestOptions:
    checkout: Path
    label: str
    suite_name: str
    suite: Suite
    target_dir: Path
    samples: int
    warmup: int
    timeout: float
    output_dir: Path


def run_test(options: TestOptions) -> Path:
    suite = options.suite
    edit = MarkerEdit(options.checkout, suite.edit)
    run_id = new_run_id()
    environment = dict(os.environ)
    environment.update(suite.env)
    cargo = suite.runner is Runner.CARGO
    conditions = [f"edit: {suite.edit.path}"]
    if cargo:
        environment["CARGO_TARGET_DIR"] = str(options.target_dir)
        conditions.append(f"RUSTFLAGS={os.environ.get('RUSTFLAGS', '')}")
    run = start_run(
        scenario=SCENARIO,
        target=options.suite_name,
        label=options.label,
        cache=CacheState.WARM,
        cache_detail=f"{options.warmup} untimed run(s) after the first save; "
        + (
            f"target directory {options.target_dir}"
            if cargo
            else "tool caches as found"
        ),
        checkout=options.checkout,
        output_dir=options.output_dir,
        services=[],
        commands=[f"edit {suite.edit.path}", f"cd {suite.cwd} && {suite.command}"],
        parameters={
            "edit": suite.edit.to_dict(),
            "cwd": suite.cwd,
            "command": suite.command,
            "env": dict(suite.env),
            "runner": suite.runner.value,
            "target_dir": str(options.target_dir) if cargo else None,
            "marker_run_id": run_id,
            "conditions": conditions,
        },
    )
    log = run.logs / "test.log"
    try:
        for index, role in enumerate(roles(options.samples, options.warmup), start=1):
            timer = SampleTimer(index, role)
            saved = edit.apply(marker_for(run_id, index))
            result = run_command(
                suite.command,
                cwd=options.checkout / suite.cwd,
                log=log,
                env=environment,
                timeout=options.timeout,
            )
            run.add(
                timer.finish(
                    ok=result.ok,
                    seconds=result.seconds,
                    detail={"saved_at_epoch": saved, "log": str(log)},
                    error=None
                    if result.ok
                    else f"exit {result.returncode}: {result.output_tail[-800:]}",
                )
            )
    finally:
        edit.restore()
    return run.finish()
