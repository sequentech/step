#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Linker wrapper that appends each link's output and duration to a log.

The rust benchmark sets it as the target linker; ``STEP_BENCH_REAL_LINKER`` is
the linker it runs and ``STEP_BENCH_LINK_LOG`` the JSON-lines log.
"""

import json
import os
import subprocess
import sys
import time


def expanded(arguments: list[str]) -> list[str]:
    """Arguments with rustc's ``@file`` response files read, one per line."""
    result: list[str] = []
    for argument in arguments:
        if argument.startswith("@") and os.path.isfile(argument[1:]):
            with open(argument[1:], encoding="utf-8", errors="replace") as handle:
                result.extend(line.strip().strip('"') for line in handle)
        else:
            result.append(argument)
    return result


def output_of(arguments: list[str]) -> str | None:
    values = expanded(arguments)
    for index, argument in enumerate(values):
        if argument == "-o" and index + 1 < len(values):
            return values[index + 1]
    return None


def main() -> int:
    arguments = sys.argv[1:]
    started = time.time()
    status = subprocess.call([os.environ["STEP_BENCH_REAL_LINKER"], *arguments])
    finished = time.time()
    record = {
        "output": output_of(arguments),
        "started": started,
        "seconds": finished - started,
        "status": status,
    }
    with open(os.environ["STEP_BENCH_LINK_LOG"], "a", encoding="utf-8") as log:
        log.write(json.dumps(record) + "\n")
    return status


if __name__ == "__main__":
    sys.exit(main())
