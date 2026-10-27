# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Run after election-lifecycle.sql in its disposable PostgreSQL fixture.

This exercises the actual PostgreSQL transaction-lock behavior used across the
Hasura and event-store connections. It does not replace the end-to-end cast test.
"""
import subprocess

PSQL = ["sudo", "docker", "exec", "-i", "agd-st-postgres", "psql", "-X", "-qAt",
        "-U", "postgres", "-d", "lifecycle_complete_test", "-v", "ON_ERROR_STOP=1"]
KEY = "hashtextextended('11111111-1111-4111-8111-111111111111:44444444-4444-4444-8444-444444444444',848527)"

def query(sql):
    return subprocess.run(PSQL, input=sql, text=True, capture_output=True,
                          check=True, timeout=5).stdout.strip()

def transaction(sql):
    process = subprocess.Popen(PSQL, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, bufsize=1)
    process.stdin.write(sql + "\n")
    process.stdin.flush()
    assert process.stdout.readline().strip() == "t"
    return process

# A readonly Hasura transaction can hold the safety lock while the event-store
# connection commits a received or cast ballot. Before this fix that Hasura
# transaction had already committed, allowing closure to overtake acceptance.
reader = transaction(f"BEGIN READ ONLY; SELECT pg_try_advisory_xact_lock_shared({KEY});")
assert query(f"SELECT pg_try_advisory_xact_lock({KEY});") == "f", "Close overtook an in-flight ballot"
assert query(f"SELECT pg_try_advisory_xact_lock_shared({KEY});") == "t", "Independent voters serialized unnecessarily"
reader.stdin.write("COMMIT;\n")
reader.stdin.close()
assert reader.wait(timeout=5) == 0
assert query(f"SELECT pg_try_advisory_xact_lock({KEY});") == "t", "Close could not proceed after ballot completion"

writer = transaction(f"BEGIN; SELECT pg_try_advisory_xact_lock({KEY});")
assert query(f"SELECT pg_try_advisory_xact_lock_shared({KEY});") == "f", "Voter entered during a close/maintenance transition"
writer.stdin.write("COMMIT;\n")
writer.stdin.close()
assert writer.wait(timeout=5) == 0
assert query(f"SELECT pg_try_advisory_xact_lock_shared({KEY});") == "t"
print("PASS: in-flight ballot blocks close, parallel ballots share access, close blocks new ballots, completion releases locks")
