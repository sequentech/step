#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Runs the whole suite against the running dev environment: the smoke checks, then the e2e runs.

Cleanup always runs. Exits non-zero when any check failed. Run it with the virtualenv's Python
(see README.md); every step runs with the same interpreter.
"""

import shutil
import subprocess
import sys
from pathlib import Path

from common import OUT, STATE_FILE

HERE = Path(__file__).resolve().parent
failed: list[str] = []


def run(script: str, *args: str) -> bool:
    print(f"\n=== {script} {' '.join(args)}".rstrip(), flush=True)
    ok = subprocess.run([sys.executable, str(HERE / script), *args]).returncode == 0
    if not ok:
        failed.append(script)
    return ok


def main() -> int:
    STATE_FILE.unlink(missing_ok=True)
    shutil.rmtree(OUT / "screenshots", ignore_errors=True)
    if not run("port_forwards.py", "start"):
        return 1
    try:
        run("smoke_admin_client.py")
        run("smoke_realm_lifecycle.py")
        run("smoke_portals.py")
        if run("e2e_create_event.py"):
            if run("e2e_idp_linking_setup.py"):
                run("e2e_idp_linking.py")
            if run("e2e_x509_setup.py"):
                run("e2e_x509_login.py")
    finally:
        run("smoke_cleanup.py")
        run("e2e_cleanup.py")
        run("port_forwards.py", "stop")

    print(f"\nscreenshots: {OUT / 'screenshots'}")
    if failed:
        print("FAILED: " + ", ".join(failed))
        return 1
    print("all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
