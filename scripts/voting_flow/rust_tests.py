# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run production Rust helpers against the disposable regression database."""

import os
import re
import subprocess

from database import ROOT


def run_rust_tests(database):
    environment = dict(
        os.environ,
        CAST_VOTE_TEST_DATABASE_URL=database.dsn,
        CARGO_TARGET_DIR=str(ROOT / "packages/windmill/rust-local-target"),
    )
    result = subprocess.run(
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
        text=True,
        stdout=subprocess.PIPE,
    )
    print(result.stdout, end="")
    # Cargo exits successfully for an empty filter; that must not silently
    # remove the production database helpers from this validation workflow.
    executed = re.search(r"test result: ok\. ([0-9]+) passed;", result.stdout)
    if executed is None or int(executed.group(1)) == 0:
        raise RuntimeError("The voting-flow Rust filter did not execute any tests")
