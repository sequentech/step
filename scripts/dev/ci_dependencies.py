# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Verify the native dependencies provided by the hosted Ubuntu runner image."""

import argparse
import subprocess
from pathlib import Path


def verify_dependencies(*, postgres=False, clang=False, run=subprocess.run):
    for command in (
        ["cc", "--version"],
        ["c++", "--version"],
        ["make", "--version"],
        ["pkg-config", "--modversion", "openssl"],
    ):
        run(command, check=True)
    if clang:
        run(["clang", "--version"], check=True)
    if postgres:
        result = run(
            ["pg_config", "--bindir"], check=True, capture_output=True, text=True
        )
        directory = Path(result.stdout.strip())
        if not directory.is_absolute():
            raise ValueError("pg_config must return an absolute PostgreSQL binary path")
        for binary in ("initdb", "pg_ctl"):
            run([str(directory / binary), "--version"], check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--postgres", action="store_true")
    parser.add_argument("--clang", action="store_true")
    args = parser.parse_args()
    try:
        verify_dependencies(postgres=args.postgres, clang=args.clang)
    except (FileNotFoundError, subprocess.CalledProcessError, ValueError) as error:
        raise SystemExit(f"Required CI dependency is unavailable: {error}") from error
