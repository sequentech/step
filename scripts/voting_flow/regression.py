# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Database invariants tested through the real migration and production query."""

from concurrent.futures import ThreadPoolExecutor
import threading
from pathlib import Path
import subprocess
import unittest

import psycopg
from psycopg.types.json import Jsonb

from database import (
    CHANNELS_MIGRATION,
    ONLINE_WINDOW_MIGRATION,
    CONFIGURATION_QUERY,
    SCHEDULE_MIGRATION,
    WINDOW_MIGRATION,
)
from fixtures import Election

# In Hasura order; roll them back in reverse.
SCHEDULE_MIGRATIONS = (
    SCHEDULE_MIGRATION,
    WINDOW_MIGRATION,
    CHANNELS_MIGRATION,
    ONLINE_WINDOW_MIGRATION,
)


class VotingFlowTests(unittest.TestCase):
    database = None

    def setUp(self):
        """Create an independent open election for each regression in the shared test database."""
        self.db = self.database
        self.connection = self.db.connection
        self.election = Election()
        self.election.create(self.connection)

    def dates(self, election=None, connection=None):
        """Read projected start/end dates for the chosen fixture and optional transaction."""
        election = election or self.election
        connection = connection or self.connection
        row = connection.execute(CONFIGURATION_QUERY, election.scope).fetchone()
        return row[3], row[4]

    def concurrent(self, operations):
        """Race operations on independent transactions and return success or primary SQL error messages."""
        barrier = threading.Barrier(len(operations))

        def run(operation):
            """Synchronize one operation with its peers and capture database failures for assertions."""
            with psycopg.connect(self.db.dsn, autocommit=True) as connection:
                barrier.wait(timeout=20)
                try:
                    with connection.transaction():
                        operation(connection)
                        # Widen the overlap so the test exercises lock waits.
                        connection.execute("SELECT pg_sleep(0.03)")
                    return None
                except psycopg.Error as error:
                    return error.diag.message_primary

        with ThreadPoolExecutor(max_workers=len(operations)) as executor:
            return list(executor.map(run, operations))

    def test_concurrent_reference_index_prebuild_and_rollback(self):
        """Reuse a concurrently built index in the transactional migration, then roll it back."""
        root = Path(__file__).resolve().parents[2]
        migration = root / "hasura/migrations/backend-db/1788808561206_ballot_style_voter_reference_index"
        self.connection.execute("""
            CREATE TABLE sequent_backend.ballot_style (
                tenant_id uuid, election_event_id uuid, area_id uuid,
                election_id uuid, id uuid, ballot_publication_id uuid, deleted_at timestamptz
            )
        """)
        try:
            for script in ["ballot_style_voter_reference_index.sql", "drop_ballot_style_voter_reference_index.sql"]:
                subprocess.run([
                    "psql", "-X", "--quiet", "--file", str(root / "scripts/postgres" / script),
                ], env=self.db.env, check=True, stdout=subprocess.DEVNULL)
                direction = "down" if script.startswith("drop_") else "up"
                with self.connection.transaction():
                    self.connection.execute((migration / f"{direction}.sql").read_text())
                if direction == "up":
                    self.assertEqual(self.connection.execute(
                        "SELECT indisvalid FROM pg_index WHERE indexrelid = 'sequent_backend.ballot_style_voter_reference_idx'::regclass"
                    ).fetchone(), (True,))
                else:
                    self.assertEqual(self.connection.execute(
                        "SELECT to_regclass('sequent_backend.ballot_style_voter_reference_idx')"
                    ).fetchone(), (None,))
        finally:
            self.connection.execute("DROP TABLE sequent_backend.ballot_style")

    def roll_back_schedule_migrations(self):
        """Roll back every schedule migration, latest first."""
        for migration in reversed(SCHEDULE_MIGRATIONS):
            self.db.apply(migration, "down")

    def apply_schedule_migrations(self):
        """Reapply every schedule migration in Hasura order."""
        for migration in SCHEDULE_MIGRATIONS:
            self.db.apply(migration)

    def test_schedule_migration_rejects_duplicates_atomically_and_recovers(self):
        """Roll back index creation on duplicate schedules and migrate after correction."""
        self.roll_back_schedule_migrations()
        duplicates = [self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z") for _ in range(2)]
        with self.assertRaises(psycopg.errors.UniqueViolation):
            self.db.apply(SCHEDULE_MIGRATION)
        for name in ("scheduled_event_active_scope_task_idx", "scheduled_event_active_voting_task_idx"):
            self.assertEqual(self.connection.execute(
                "SELECT to_regclass(%s)", (f"sequent_backend.{name}",)
            ).fetchone(), (None,))
        self.connection.execute("DELETE FROM sequent_backend.scheduled_event WHERE id=%s", (duplicates[1],))
        self.apply_schedule_migrations()
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))
        for name in ("scheduled_event_active_scope_task_idx", "scheduled_event_active_voting_task_idx"):
            self.assertEqual(self.connection.execute(
                "SELECT indisvalid FROM pg_index WHERE indexrelid=to_regclass(%s)",
                (f"sequent_backend.{name}",),
            ).fetchone(), (True,))

    def test_schedule_migration_and_rollback(self):
        """Verify constraint and projection backfill reapplication preserve valid schedules."""
        self.roll_back_schedule_migrations()
        self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        self.apply_schedule_migrations()
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))

    def test_voting_task_payload_matches_its_canonical_identity(self):
        """Reject name-matching schedules whose payload is foreign or has unexpected fields."""
        for payload in (
            {"election_id": str(self.election.other_area)},
            {"election_id": str(self.election.election), "unexpected": True},
        ):
            with self.assertRaises(psycopg.errors.CheckViolation):
                self.connection.execute(
                    """
                    INSERT INTO sequent_backend.scheduled_event
                        (tenant_id, election_event_id, task_id, event_payload, cron_config)
                    VALUES (%s, %s, %s, %s, %s)
                    """,
                    (
                        self.election.tenant, self.election.event, self.election.task_name("END"),
                        Jsonb(payload), Jsonb({"scheduled_date": "2026-10-02T12:00:00Z"}),
                    ),
                )
        self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))

    def test_unrelated_tasks_are_not_subject_to_voting_date_validation(self):
        """Non-voting schedules can retain their own payload and cron formats."""
        self.connection.execute(
            """
            INSERT INTO sequent_backend.scheduled_event
                (tenant_id, election_event_id, task_id, event_payload, cron_config)
            VALUES (%s, %s, 'another-task', %s, %s)
            """,
            (self.election.tenant, self.election.event, Jsonb([42]), Jsonb({"scheduled_date": 42})),
        )
        self.assertEqual(self.dates(), (None, None))

    def test_online_window_follows_schedule_channels(self):
        """Count only schedules that include ONLINE; no channel list means ONLINE and KIOSK."""
        start = self.election.schedule(self.connection, "START", "2026-10-01T10:00:00Z")
        end = self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")

        def select_channels(schedule, channels):
            payload = {"election_id": str(self.election.election), "voting_channels": channels}
            self.connection.execute(
                "UPDATE sequent_backend.scheduled_event SET event_payload = %s WHERE id = %s",
                (Jsonb(payload), schedule),
            )

        select_channels(start, ["KIOSK", "ONLINE"])
        select_channels(end, ["KIOSK"])
        self.assertEqual(self.dates(), ("2026-10-01T10:00:00Z", None))
        select_channels(end, [])
        self.assertEqual(self.dates(), ("2026-10-01T10:00:00Z", "2026-10-01T12:00:00Z"))
        select_channels(start, ["TELEPHONE"])
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))

    def test_schedule_writes_do_not_block_other_tenants(self):
        """An open schedule transaction must not delay another tenant's stop task."""
        other = Election()
        other.create(self.connection)
        own_end = self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        other_end = other.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        with psycopg.connect(self.db.dsn) as owner, psycopg.connect(self.db.dsn) as contender:
            owner.execute(
                "UPDATE sequent_backend.scheduled_event SET labels = '{}' WHERE id = %s",
                (own_end,),
            )
            contender.execute("SET LOCAL lock_timeout = '100ms'")
            contender.execute(
                "UPDATE sequent_backend.scheduled_event SET stopped_at = now() WHERE id = %s",
                (other_end,),
            )
            contender.commit()
            owner.rollback()

    def test_deadlines_require_an_explicit_rfc3339_timestamp(self):
        """PostgreSQL shorthand dates must not become missing deadlines in Rust."""
        for value in ("2026-10-01", "tomorrow", "infinity"):
            with self.subTest(value=value), self.connection.transaction(force_rollback=True):
                with self.assertRaises(psycopg.Error):
                    self.election.schedule(self.connection, "END", value)

    def test_concurrent_first_endpoints_do_not_lose_an_update(self):
        """Preserve both endpoints when separate transactions create the first opening and closing tasks."""
        operations = [
            lambda connection: self.election.schedule(
                connection, "START", "2026-10-01T10:00:00Z"
            ),
            lambda connection: self.election.schedule(
                connection, "END", "2026-10-01T12:00:00Z"
            ),
        ]
        self.assertEqual(self.concurrent(operations), [None, None])
        self.assertEqual(self.dates(), ("2026-10-01T10:00:00Z", "2026-10-01T12:00:00Z"))

    def test_inverse_scope_order_across_statements(self):
        """Serialize configuration transactions before inverse scope order can deadlock."""
        other = Election()
        other.create(self.connection)

        def write(elections, endpoint):
            def operation(connection):
                connection.execute("SET LOCAL statement_timeout = '5s'")
                for election in elections:
                    election.schedule(connection, endpoint, "2026-10-01T12:00:00Z")
                    connection.execute("SELECT pg_sleep(0.05)")
            return operation

        self.assertEqual(self.concurrent([
            write([self.election, other], "START"),
            write([other, self.election], "END"),
        ]), [None, None])
        for election in [self.election, other]:
            self.assertEqual(self.dates(election), ("2026-10-01T12:00:00Z",) * 2)

    def test_inverse_scope_order_in_multirow_statements(self):
        """Two bulk inserts with opposing row orders preserve every projected endpoint."""
        other = Election()
        other.create(self.connection)

        def write(elections, endpoint):
            def operation(connection):
                connection.execute("SET LOCAL statement_timeout = '5s'")
                parameters = []
                for election in elections:
                    parameters.extend([
                        election.tenant, election.event, election.task_name(endpoint),
                        Jsonb({"election_id": str(election.election)}),
                        Jsonb({"scheduled_date": "2026-10-01T12:00:00Z"}),
                    ])
                connection.execute("""
                    INSERT INTO sequent_backend.scheduled_event
                        (tenant_id, election_event_id, task_id, event_payload, cron_config)
                    VALUES (%s, %s, %s, %s, %s), (%s, %s, %s, %s, %s)
                """, parameters)
            return operation

        self.assertEqual(self.concurrent([
            write([self.election, other], "START"),
            write([other, self.election], "END"),
        ]), [None, None])
        for election in [self.election, other]:
            self.assertEqual(self.dates(election), ("2026-10-01T12:00:00Z",) * 2)

    def test_concurrent_reschedules_do_not_lose_an_update(self):
        """Preserve both changed dates when opening and closing tasks are updated concurrently."""
        start = self.election.schedule(self.connection, "START", "2026-10-01T10:00:00Z")
        end = self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")

        def change(schedule_id, date):
            """Build a transaction operation that replaces the chosen endpoint date."""

            def operation(connection):
                """Update one endpoint through the production projection-maintenance trigger."""
                connection.execute(
                    """
                    UPDATE sequent_backend.scheduled_event SET cron_config = %s WHERE id = %s
                """,
                    (Jsonb({"scheduled_date": date}), schedule_id),
                )

            return operation

        results = self.concurrent(
            [
                change(start, "2026-10-02T10:00:00Z"),
                change(end, "2026-10-02T12:00:00Z"),
            ]
        )
        self.assertEqual(results, [None, None])
        self.assertEqual(self.dates(), ("2026-10-02T10:00:00Z", "2026-10-02T12:00:00Z"))

    def test_duplicate_endpoint_fails_without_changing_projection(self):
        """Reject an ambiguous endpoint and retain the last valid projected deadline."""
        self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        with self.assertRaises(psycopg.errors.UniqueViolation):
            self.election.schedule(self.connection, "END", "2026-10-02T12:00:00Z")
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))

    def test_invalid_date_type_is_rejected_at_configuration_write(self):
        """Reject a non-string endpoint date before it can populate the projection."""
        with self.assertRaises(psycopg.Error):
            self.election.schedule(self.connection, "END", 42)
        self.assertEqual(self.dates(), (None, None))

    def test_malformed_configuration_cannot_publish_a_missing_deadline(self):
        """Reject malformed schedule updates atomically while retaining the valid deadline."""
        schedule_id = self.election.schedule(
            self.connection, "END", "2026-10-01T12:00:00Z"
        )
        for configuration in (
            ["invalid"],
            {"cron": 42},
            {"scheduled_date": "invalid-date"},
        ):
            with self.subTest(configuration=configuration):
                with self.assertRaises(psycopg.Error):
                    self.connection.execute(
                        "UPDATE sequent_backend.scheduled_event SET cron_config = %s WHERE id = %s",
                        (Jsonb(configuration), schedule_id),
                    )
                self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))

    def test_move_archive_unarchive_and_delete(self):
        """Keep both election projections correct as a schedule changes scope and active state."""
        target = Election()
        target.create(self.connection)
        schedule_id = self.election.schedule(
            self.connection, "END", "2026-10-01T12:00:00Z"
        )
        self.connection.execute(
            """
            UPDATE sequent_backend.scheduled_event
            SET tenant_id = %s, election_event_id = %s, task_id = %s, event_payload = %s
            WHERE id = %s
        """,
            (
                target.tenant,
                target.event,
                target.task_name("END"),
                Jsonb({"election_id": str(target.election)}),
                schedule_id,
            ),
        )
        self.assertEqual(self.dates(), (None, None))
        self.assertEqual(self.dates(target), (None, "2026-10-01T12:00:00Z"))
        self.connection.execute(
            "UPDATE sequent_backend.scheduled_event SET archived_at = now() WHERE id = %s",
            (schedule_id,),
        )
        self.assertEqual(self.dates(target), (None, None))
        self.connection.execute(
            """
            UPDATE sequent_backend.scheduled_event SET archived_at = NULL, stopped_at = now()
            WHERE id = %s
        """,
            (schedule_id,),
        )
        # Executed/stopped tasks still define a deadline; only archiving removes it.
        self.assertEqual(self.dates(target), (None, "2026-10-01T12:00:00Z"))
        self.connection.execute(
            "DELETE FROM sequent_backend.scheduled_event WHERE id = %s", (schedule_id,)
        )
        self.assertEqual(self.dates(target), (None, None))

    def test_projection_and_source_rollback_together(self):
        """Verify uncommitted dates are transaction-local and disappear with source rollback."""
        schedule_id = self.election.schedule(
            self.connection, "END", "2026-10-01T12:00:00Z"
        )
        with psycopg.connect(self.db.dsn, autocommit=True) as writer:
            with writer.transaction(force_rollback=True):
                writer.execute(
                    "UPDATE sequent_backend.scheduled_event SET cron_config = %s WHERE id = %s",
                    (
                        Jsonb({"scheduled_date": "2026-10-02T12:00:00Z"}),
                        schedule_id,
                    ),
                )
                self.assertEqual(
                    self.dates(connection=writer), (None, "2026-10-02T12:00:00Z")
                )
                self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))

    def test_projection_exists_before_election_import(self):
        """Support schedule-first imports without losing the projected deadline."""
        future = Election()
        future.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        future.create(self.connection)
        self.assertEqual(self.dates(future), (None, "2026-10-01T12:00:00Z"))

    def test_status_changes_remain_authoritative(self):
        """Read administrative pauses immediately and reject a foreign tenant scope."""
        self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        self.connection.execute(
            "UPDATE sequent_backend.election SET status = %s WHERE id = %s",
            (
                Jsonb({"voting_status": "PAUSED"}),
                self.election.election,
            ),
        )
        row = self.connection.execute(
            CONFIGURATION_QUERY, self.election.scope
        ).fetchone()
        self.assertEqual(row[1]["voting_status"], "PAUSED")
        foreign_scope = (Election().tenant, self.election.event, self.election.election)
        self.assertIsNone(
            self.connection.execute(CONFIGURATION_QUERY, foreign_scope).fetchone()
        )

    def test_truncate_clears_projection(self):
        """Remove all materialized windows when the source schedule table is truncated."""
        self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        self.connection.execute("TRUNCATE sequent_backend.scheduled_event")
        self.assertEqual(self.dates(), (None, None))
        self.assertEqual(
            self.db.scalar(
                "SELECT count(*) FROM sequent_backend.election_voting_window"
            ),
            0,
        )

    def test_concurrent_duplicate_endpoints_have_one_winner(self):
        """The unique index protects endpoint identity across concurrent writers."""
        results = self.concurrent([
            lambda connection: self.election.schedule(connection, "END", "2026-10-01T12:00:00Z")
            for _ in range(2)
        ])
        self.assertEqual(results.count(None), 1)
        self.assertTrue(any("scheduled_event_active_voting_task_idx" in (result or "") for result in results))
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))

    def test_optional_dates_and_timezone_offsets(self):
        """Preserve absent endpoints and valid RFC3339 offsets and fractions."""
        self.election.schedule(self.connection, "START", None)
        value = "2026-10-01T12:00:00.123456789-05:00"
        self.election.schedule(self.connection, "END", value)
        self.assertEqual(self.dates(), (None, value))


def run_regressions(database):
    """Apply the schedule migrations, run their database invariants and exit nonzero on failure."""
    for migration in SCHEDULE_MIGRATIONS:
        database.apply(migration)
    VotingFlowTests.database = database
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(VotingFlowTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful():
        raise SystemExit("Voting-flow database regressions failed")
