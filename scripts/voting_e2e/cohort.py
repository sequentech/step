# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Capture distinct synthetic voters sequentially, with no cast retries."""

import argparse
import csv
import json
import os
from pathlib import Path
import platform
import time

from capture import run, save
from report import generate


def main() -> None:
    """Read a private voter CSV and retain separate evidence for every attempted journey."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", type=Path)
    parser.add_argument("voters", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--count", type=int, default=10)
    parser.add_argument("--offset", type=int, default=0)
    args = parser.parse_args()
    if args.count < 1 or args.offset < 0:
        parser.error("count must be positive and offset nonnegative")
    with args.voters.open() as stream:
        voters = list(csv.DictReader(stream))[args.offset : args.offset + args.count]
    if (
        len(voters) != args.count
        or len({row["username"] for row in voters}) != args.count
    ):
        parser.error(
            "The selected cohort must contain the requested number of distinct voters"
        )
    os.umask(0o077)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    inputs = output / "targets"
    inputs.mkdir()
    target = json.loads(args.target.read_text())
    manifest = {
        "schema_version": 1,
        "engine": target.get("engine", "chromium"),
        "architecture": platform.machine(),
        "logical_cpus": os.cpu_count(),
        "concurrency": 1,
        "runs": [],
    }
    started = time.monotonic()
    try:
        for index, voter in enumerate(voters, 1):
            directory = output / f"voter-{index:03d}"
            directory.mkdir()
            target_path = inputs / f"voter-{index:03d}.json"
            current = dict(target, credentials=voter)
            save(target_path, current)
            code = run(current, target_path, directory)
            manifest["runs"].append({"directory": directory.name, "exit_code": code})
            save(output / "cohort.json", manifest)
            print(
                f"Voter {index}/{args.count}: {'verified' if code == 0 else 'FAILED'}",
                flush=True,
            )
            if code:
                raise SystemExit(code)
    finally:
        manifest["wall_elapsed_ms"] = round((time.monotonic() - started) * 1000)
        save(output / "cohort.json", manifest)
        generate(output)


if __name__ == "__main__":
    main()
