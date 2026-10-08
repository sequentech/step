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


def cleanup() -> bool:
    """Deletes what the scripts recorded in the state file. The file is kept when any of it could
    not be deleted, so the next run retries before creating anything."""
    results = [run("smoke_cleanup.py"), run("e2e_cleanup.py")]
    if all(results):
        STATE_FILE.unlink(missing_ok=True)
        return True
    print(f"cleanup failed, keeping {STATE_FILE} for the next attempt")
    return False


def main() -> int:
    if STATE_FILE.exists():
        print(f"{STATE_FILE} is left from an earlier run, cleaning that up first")
        if not cleanup():
            return 1
    shutil.rmtree(OUT / "screenshots", ignore_errors=True)
    try:
        if run("port_forwards.py", "start"):
            run("smoke_admin_client.py")
            run("smoke_realm_lifecycle.py")
            run("smoke_portals.py")
            if run("e2e_create_event.py"):
                if run("e2e_idp_linking_setup.py"):
                    run("e2e_idp_linking.py")
                if run("e2e_x509_setup.py"):
                    run("e2e_x509_login.py")
    finally:
        cleanup()
        run("port_forwards.py", "stop")

    print(f"\nscreenshots: {OUT / 'screenshots'}")
    if failed:
        print("FAILED: " + ", ".join(failed))
        return 1
    print("all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
