# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Exercise the production publication lock in an isolated PostgreSQL cluster.

Run inside devenv: python3 scripts/test_ballot_publication_lifecycle.py
No benchmarks or running application databases are used.
"""

import sys
from pathlib import Path

import psycopg

sys.path.insert(0, str(Path(__file__).parent / "voting_flow"))
from database import ROOT, local_database

TENANT = "10000000-0000-4000-8000-000000000001"
EVENT = "10000000-0000-4000-8000-000000000002"
OTHER_EVENT = "10000000-0000-4000-8000-000000000003"
OTHER_TENANT = "20000000-0000-4000-8000-000000000001"
LOCK = (
    (ROOT / "packages/windmill/src/postgres/sql/lock_publication_event.sql")
    .read_text()
    .replace("$1", "%s")
    .replace("$2", "%s")
)


def run_regressions(database):
    """Prove serialization, tenant isolation, and release at both transaction outcomes."""
    database.connection.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id) VALUES (%s, %s), (%s, %s)",
        (EVENT, TENANT, OTHER_EVENT, TENANT),
    )
    for outcome in ("commit", "rollback"):
        with psycopg.connect(database.dsn) as worker, psycopg.connect(
            database.dsn
        ) as contender:
            assert worker.execute(LOCK, (TENANT, EVENT)).fetchone() is not None
            contender.execute("SET lock_timeout = '100ms'")
            # A mismatched tenant cannot acquire or observe the event lock.
            assert contender.execute(LOCK, (OTHER_TENANT, EVENT)).fetchone() is None
            # Independent events do not serialize with this worker.
            assert contender.execute(LOCK, (TENANT, OTHER_EVENT)).fetchone() is not None
            contender.commit()
            contender.execute("SET lock_timeout = '100ms'")
            try:
                contender.execute(LOCK, (TENANT, EVENT))
            except psycopg.errors.LockNotAvailable:
                contender.rollback()
            else:
                raise AssertionError("Concurrent publication escaped the event lock")
            getattr(worker, outcome)()
            contender.execute("SET lock_timeout = '100ms'")
            assert contender.execute(LOCK, (TENANT, EVENT)).fetchone() is not None
    print(
        "Publication lifecycle: commit/rollback serialization, independent events and tenant isolation passed"
    )


if __name__ == "__main__":
    with local_database() as database:
        run_regressions(database)
