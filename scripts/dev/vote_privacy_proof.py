#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run the synthetic ballot privacy demonstration against disposable databases.

Use through `scripts/dev/step-dev vote_privacy_proof` inside devenv. The caller
supplies ELECTORAL_LOG_PG_* for a disposable event catalog and HASURA_DB__* for
its test PostgreSQL server. Tests create random event databases and a migrated
Harvest fixture database. No deployed identity provider or resolver is called.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
STAGES = (
    (
        "Signed ballot, exports, User's Logs and verification after purge",
        "windmill",
        "services::ballot_box::privacy_tests::privacy_sequencer_orders_bindings_and_deduplicates_recovery",
    ),
    (
        "Public ballot proof and authorized, audited pseudonym resolution",
        "harvest",
        "routes::ballot_privacy::route_tests::authorized_resolution_is_private_durable_and_unavailable_after_purge",
    ),
)


def main(argv=None):
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    required = ("ELECTORAL_LOG_PG_HOST", "ELECTORAL_LOG_PG_DATABASE", "HASURA_DB__HOST")
    if any(not os.environ.get(name) for name in required):
        print("Configure disposable ELECTORAL_LOG_PG_* and HASURA_DB__* databases first.", file=sys.stderr)
        return 2
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(ROOT / "packages/rust-local-target")
    env.setdefault("ENV_SLUG", "privacy-demo")
    if not env.get("CHROME") and env.get("CHROMIUM_EXECUTABLE_PATH"):
        env["CHROME"] = env["CHROMIUM_EXECUTABLE_PATH"]
    # Only synthetic secrets created by the isolated fixture use this key.
    env["SECRETS_BACKEND"] = "EnvVarMasterSecret"
    env["MASTER_SECRET"] = "11" * 32
    # Build affected native packages together: separate package invocations can
    # repeatedly rebuild Windmill under different unified dependency features.
    print("Building the affected native test targets", flush=True)
    build = subprocess.run(
        ["cargo", "test", "--no-run", "--message-format=json-render-diagnostics",
         "-p", "windmill", "-p", "harvest", "-p", "step-cli", "-p", "sequent-core",
         "--lib", "--bin", "harvest", "--bin", "step-cli",
         "--features", "sequent-core/election_config_xlsx"],
        cwd=ROOT / "packages", env=env, capture_output=True, text=True,
    )
    if build.returncode:
        sys.stderr.write(build.stderr)
        return 1
    executables = {}
    for line in build.stdout.splitlines():
        try:
            artifact = json.loads(line)
        except ValueError:
            continue
        if artifact.get("reason") == "compiler-artifact" and artifact.get("profile", {}).get("test"):
            target = artifact["target"]
            kind = {"windmill": "lib", "harvest": "bin"}.get(target["name"])
            if artifact.get("executable") and kind in target["kind"]:
                executables[target["name"]] = artifact["executable"]
    for label, package, test in STAGES:
        executable = executables.get(package)
        if not executable:
            print(f"FAIL: missing {package} test executable", file=sys.stderr)
            return 1
        print(f"Running: {label}", flush=True)
        result = subprocess.run(
            [executable, test, "--ignored", "--exact", "--test-threads=1", "--color=never"],
            cwd=ROOT / "packages", env=env, capture_output=True, text=True,
        )
        # Rust test runners also succeed when a renamed filter selects no tests.
        if result.returncode or not re.search(r"test result: ok\. 1 passed;", result.stdout):
            sys.stdout.write(result.stdout)
            sys.stderr.write(result.stderr)
            print(f"FAIL: {label} (one passing demonstration test required)", file=sys.stderr)
            return 1
        print(f"PASS: {label}", flush=True)
    print("Local synthetic demonstration passed. Record deployment and peak-load acceptance separately.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
