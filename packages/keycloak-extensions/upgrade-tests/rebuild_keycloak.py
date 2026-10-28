#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Rebuilds the Keycloak image and recreates only the keycloak container.

The extensions are compiled and their tests run inside the image build. --no-deps keeps compose
from recreating anything else, the dev container included. Docker runs outside the dev container,
so bind mounts resolve on the host: the realm import directory is mapped from
$LOCAL_WORKSPACE_FOLDER, otherwise Keycloak would mount an empty host directory.
"""

import os
import re
import subprocess
import sys
import time
from pathlib import Path

from common import OUT, Checks

COMPOSE_DIR = Path(__file__).resolve().parents[3] / ".devcontainer"
OVERRIDE = OUT / "keycloak-host-paths.override.yml"
BUILD_LOG = OUT / "keycloak-build.log"
STARTED = re.compile(r"Keycloak [0-9.]+ on JVM .* started in")
ERROR_LINE = re.compile(r"^[0-9-]+ [0-9:,]+ ERROR ", re.MULTILINE)


def keycloak_log() -> str:
    return subprocess.run(
        ["docker", "logs", "keycloak"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    ).stdout


def main() -> int:
    checks = Checks()
    compose = ["docker", "compose", "-f", str(COMPOSE_DIR / "docker-compose.yml")]
    workspace = os.environ.get("LOCAL_WORKSPACE_FOLDER")
    if workspace:
        OVERRIDE.write_text(
            "services:\n"
            "  keycloak:\n"
            "    volumes:\n"
            f"      - {workspace}/.devcontainer/keycloak/import:/opt/keycloak/data/import:ro,z\n"
        )
        compose += ["-f", str(OVERRIDE)]
    else:
        checks.note(
            "LOCAL_WORKSPACE_FOLDER is not set: the realm import may mount an empty host directory"
        )

    print(f"building the keycloak image (log: {BUILD_LOG})", flush=True)
    with BUILD_LOG.open("w") as log:
        build = subprocess.run(
            compose + ["build", "keycloak"],
            cwd=COMPOSE_DIR,
            stdout=log,
            stderr=subprocess.STDOUT,
        )
    if not checks.check(
        "image build", build.returncode == 0, f"exit {build.returncode}"
    ):
        print("\n".join(BUILD_LOG.read_text().splitlines()[-30:]))
        return checks.finish()

    subprocess.run(
        compose
        + ["up", "-d", "--no-deps", "--no-build", "--force-recreate", "keycloak"],
        cwd=COMPOSE_DIR,
        check=True,
    )
    started = None
    for _ in range(150):
        match = STARTED.search(keycloak_log())
        if match:
            started = match.group(0)
            break
        state = subprocess.run(
            ["docker", "inspect", "keycloak", "--format", "{{.State.Status}}"],
            capture_output=True,
            text=True,
        ).stdout.strip()
        if state in ("exited", "dead"):
            break
        time.sleep(2)

    log = keycloak_log()
    error_lines = len(ERROR_LINE.findall(log))
    exceptions = len(re.findall(r"Exception|Caused by", log))
    checks.check("keycloak started", started is not None, started or "not started")
    checks.check(
        "no ERROR lines or exceptions at startup",
        error_lines == 0 and exceptions == 0,
        {"error_lines": error_lines, "exceptions": exceptions},
    )
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
