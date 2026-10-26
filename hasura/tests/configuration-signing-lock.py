#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise the trust-update/import lock in an isolated empty PostgreSQL container.

Run: python3 hasura/tests/configuration-signing-lock.py --container <disposable-pg>
The container must have an empty postgres database and local trust authentication.
No host ports are required. Existing sequent_backend schemas are never changed.
"""
import argparse
import json
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--container", required=True)
args = parser.parse_args()
command = ["docker", "exec", "-i", args.container, "psql", "-X", "-U", "postgres", "-v", "ON_ERROR_STOP=1", "-qAt"]


def sql(text, check=True):
    return subprocess.run(command, input=text, text=True, capture_output=True, check=check, timeout=5)


tenant = "00000000-0000-4000-8000-000000000001"
pem = "-----BEGIN CERTIFICATE-----\nY2E=\n-----END CERTIFICATE-----\n"
config = {"policy": "required", "package_roots": pem, "staff_roots": pem, "required_approvals": 2}
migration = Path(__file__).parents[1] / "migrations/backend-db/1793020000000_configuration_signing_settings/up.sql"
# Refuse an occupied schema before entering cleanup: only our own tables are dropped.
sql("CREATE SCHEMA sequent_backend;")
try:
    sql("CREATE TABLE sequent_backend.tenant (id uuid PRIMARY KEY, settings jsonb); CREATE TABLE sequent_backend.election_event (tenant_id uuid, is_archived boolean);")
    sql(migration.read_text())
    sql(f"INSERT INTO sequent_backend.tenant VALUES ('{tenant}', '{json.dumps({'configuration_signing': config})}'::jsonb);")
    imported = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    imported.stdin.write(f"BEGIN; SELECT pg_advisory_xact_lock(hashtextextended('configuration-package-import:{tenant}', 0)); INSERT INTO sequent_backend.election_event VALUES ('{tenant}', false); SELECT pg_sleep(2); COMMIT;")
    imported.stdin.close()
    deadline = time.monotonic() + 1.5
    while sql("SELECT EXISTS (SELECT 1 FROM pg_locks WHERE locktype='advisory' AND granted);").stdout.strip() != "t":
        if time.monotonic() > deadline:
            raise AssertionError("import lock did not appear")
        time.sleep(0.05)
    changed = {**config, "staff_roots": pem + pem}
    update = f"UPDATE sequent_backend.tenant SET settings = jsonb_build_object('configuration_signing', '{json.dumps(changed)}'::jsonb) WHERE id = '{tenant}';"
    refused = sql("SET lock_timeout='250ms'; " + update, check=False)
    assert refused.returncode != 0 and "lock timeout" in refused.stderr, refused.stderr
    imported.wait(timeout=4)
    assert imported.returncode == 0, imported.stderr.read()
    refused = sql(update, check=False)
    assert refused.returncode != 0 and "configuration_signing_active_elections" in refused.stderr, refused.stderr
    assert json.loads(sql(f"SELECT settings -> 'configuration_signing' FROM sequent_backend.tenant WHERE id='{tenant}';").stdout) == config
    print("PASS: trust update waits for the import lock, then refuses the new active election; original trust preserved")
finally:
    sql("DROP SCHEMA sequent_backend CASCADE;")
