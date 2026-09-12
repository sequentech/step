# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Exercise the production publication lock in an isolated PostgreSQL cluster.

Run inside devenv: python3 scripts/test_ballot_publication_lifecycle.py
No benchmarks or running application databases are used.
"""

import sys
from pathlib import Path
from uuid import UUID, uuid4
import re

import psycopg

sys.path.insert(0, str(Path(__file__).parent / "voting_flow"))
from database import ROOT, local_database
from fixtures import Election

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
    database.connection.execute("""
        ALTER TABLE sequent_backend.cast_vote
        ADD CONSTRAINT cast_vote_election_event_id_fkey
        FOREIGN KEY (election_event_id) REFERENCES sequent_backend.election_event (id)
        ON UPDATE RESTRICT ON DELETE RESTRICT
    """)
    election = Election(tenant=UUID(TENANT), event=UUID(EVENT))
    election.create(database.connection)
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
            # Ballot inserts check the production FK while publication is locked.
            election.vote(contender, outcome)
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
        "Publication lifecycle: concurrent voting, commit/rollback serialization, independent events and tenant isolation passed"
    )


def production_query(connection, name, arguments):
    """Execute the checked-in SQL while preserving positional parameter reuse."""
    sql = (ROOT / "packages/windmill/src/postgres/sql" / name).read_text()
    positions = [int(value) - 1 for value in re.findall(r"\$(\d+)", sql)]
    return connection.execute(re.sub(r"\$\d+", "%s", sql), [arguments[index] for index in positions])


def run_generation_regressions(database):
    """Prove stale, expired and deleted generations cannot activate file references."""
    db = database.connection
    db.execute("""
        CREATE TABLE sequent_backend.lock (
            key text PRIMARY KEY, value text, expiry_date timestamptz
        );
        CREATE TABLE sequent_backend.ballot_publication (
            id uuid, tenant_id uuid, election_event_id uuid, annotations jsonb,
            is_generated bool DEFAULT false, published_at timestamptz, deleted_at timestamptz,
            PRIMARY KEY (id, tenant_id, election_event_id)
        );
        CREATE TABLE sequent_backend.ballot_style (
            id uuid, tenant_id uuid, election_event_id uuid, ballot_publication_id uuid
        );
    """)
    migration = ROOT / "hasura/migrations/backend-db/1788909000000_ballot_publication_style_index"
    database.apply(migration)
    assert db.execute(
        "SELECT indisvalid FROM pg_index WHERE indexrelid='sequent_backend.ballot_style_publication_page_idx'::regclass"
    ).fetchone() == (True,)
    publication = uuid4()
    first, second = str(uuid4()), str(uuid4())
    key = "publication-regression"
    scope = (TENANT, EVENT, publication)
    root = f"tenant-{TENANT}/event-{EVENT}/publication-{publication}/{second}"
    db.execute("INSERT INTO sequent_backend.ballot_publication (tenant_id,election_event_id,id) VALUES (%s,%s,%s)", scope)
    db.execute("INSERT INTO sequent_backend.lock VALUES (%s,%s,clock_timestamp()+interval '5 minutes')", (key, first))
    complete_args = (*scope, root, "ballot_files_v1")

    with psycopg.connect(database.dsn) as owner, psycopg.connect(database.dsn) as contender:
        assert production_query(owner, "lock_publication_generation.sql", (key, first)).fetchone()
        contender.execute("SET LOCAL lock_timeout='100ms'")
        try:
            contender.execute("UPDATE sequent_backend.lock SET value=%s WHERE key=%s", (second, key))
        except psycopg.errors.LockNotAvailable:
            contender.rollback()
        else:
            raise AssertionError("Lease takeover escaped the completion transaction")
        assert production_query(owner, "complete_ballot_publication_files.sql", complete_args).fetchone()
        owner.rollback()

    assert db.execute("SELECT is_generated,annotations FROM sequent_backend.ballot_publication").fetchone() == (False, None)
    db.execute("UPDATE sequent_backend.lock SET value=%s WHERE key=%s", (second, key))
    with db.transaction():
        assert production_query(db, "lock_publication_generation.sql", (key, first)).fetchone() is None
        assert production_query(db, "lock_publication_generation.sql", (key, second)).fetchone()

    for change in (
        "UPDATE sequent_backend.ballot_publication SET deleted_at=now()",
        "UPDATE sequent_backend.ballot_publication SET published_at=now()",
    ):
        with db.transaction(force_rollback=True):
            db.execute(change)
            assert production_query(db, "complete_ballot_publication_files.sql", complete_args).fetchone() is None

    with db.transaction(force_rollback=True):
        db.execute("UPDATE sequent_backend.lock SET expiry_date=clock_timestamp()-interval '1 second'")
        assert production_query(db, "lock_publication_generation.sql", (key, second)).fetchone() is None

    with db.transaction():
        assert production_query(db, "lock_publication_generation.sql", (key, second)).fetchone()
        assert production_query(db, "complete_ballot_publication_files.sql", complete_args).fetchone()
    assert db.execute("SELECT is_generated,annotations->>'ballot_files_v1' FROM sequent_backend.ballot_publication").fetchone() == (True, root)
    assert production_query(db, "complete_ballot_publication_files.sql", complete_args).fetchone() is None
    db.execute("DELETE FROM sequent_backend.ballot_publication")
    database.apply(migration, "down")
    database.apply(migration)
    print("Generation fencing: takeover, expiry, rollback, readiness, deletion and migration rollback passed")


if __name__ == "__main__":
    with local_database() as database:
        run_regressions(database)
        run_generation_regressions(database)
