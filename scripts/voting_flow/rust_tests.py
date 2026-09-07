# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run production Rust helpers against the disposable regression database."""

import os
import subprocess

from database import ROOT


def run_rust_tests(database):
    environment = dict(
        os.environ,
        CAST_VOTE_TEST_DATABASE_URL=database.dsn,
        CARGO_TARGET_DIR=str(ROOT / "packages/windmill/rust-local-target"),
    )
    subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "windmill",
            "--lib",
            "services::insert_cast_vote::tests",
            "--",
            "--include-ignored",
        ],
        cwd=ROOT / "packages",
        env=environment,
        check=True,
    )
