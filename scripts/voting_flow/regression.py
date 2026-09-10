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
    AREA_MIGRATION,
    CONFIGURATION_QUERY,
    STORAGE_MIGRATION,
    SCHEDULE_MIGRATION,
)
from fixtures import Election


class VotingFlowTests(unittest.TestCase):
    database = None

    def setUp(self):
        """Create an independent open election for each regression in the shared test database."""
        self.db = self.database
        self.connection = self.db.connection
        self.election = Election()
        self.election.create(self.connection)

    def dates(self, election=None, connection=None):
        """Read current start/end dates for the chosen fixture and optional transaction."""
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

    def test_schedule_migration_rejects_duplicates_atomically_and_recovers(self):
        """Roll back index creation on duplicate schedules and migrate after correction."""
        self.db.apply(SCHEDULE_MIGRATION, "down")
        duplicates = [self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z") for _ in range(2)]
        with self.assertRaises(psycopg.errors.UniqueViolation):
            self.db.apply(SCHEDULE_MIGRATION)
        for name in ("scheduled_event_active_scope_task_idx", "scheduled_event_active_voting_task_idx"):
            self.assertEqual(self.connection.execute(
                "SELECT to_regclass(%s)", (f"sequent_backend.{name}",)
            ).fetchone(), (None,))
        self.connection.execute("DELETE FROM sequent_backend.scheduled_event WHERE id=%s", (duplicates[1],))
        self.db.apply(SCHEDULE_MIGRATION)
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))
        for name in ("scheduled_event_active_scope_task_idx", "scheduled_event_active_voting_task_idx"):
            self.assertEqual(self.connection.execute(
                "SELECT indisvalid FROM pg_index WHERE indexrelid=to_regclass(%s)",
                (f"sequent_backend.{name}",),
            ).fetchone(), (True,))

    def test_schedule_migration_and_rollback(self):
        """Verify constraint and eligibility migration reapplication preserve valid schedules."""
        self.db.apply(SCHEDULE_MIGRATION, "down")
        self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        self.db.apply(SCHEDULE_MIGRATION)
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))
        self.db.apply(AREA_MIGRATION, "down")
        self.db.apply(AREA_MIGRATION)

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

    def test_bounded_concurrent_revotes(self):
        """Allow exactly the configured number of ballots when twelve requests race for one voter."""
        results = self.concurrent(
            [
                lambda connection: self.election.vote(
                    connection, "limited", status="in-progress"
                )
                for _ in range(12)
            ]
        )
        self.assertEqual(results.count(None), 3)
        self.assertEqual(results.count("insert_failed_exceeds_allowed_revotes"), 9)

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
        """Independent endpoints must not deadlock across inverse election order."""
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
        """Two bulk inserts with opposing row orders preserve every endpoint."""
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
                """Update one endpoint through the production database constraints."""
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

    def test_cross_area_rule_applies_to_unlimited_revotes(self):
        """Permit one winning area and reject the other even when the revote limit is unlimited."""
        self.connection.execute(
            "UPDATE sequent_backend.election SET num_allowed_revotes = 0 WHERE id = %s",
            (self.election.election,),
        )
        operations = [
            lambda connection, area=area: self.election.vote(connection, "areas", area)
            for area in [self.election.area, self.election.other_area] * 6
        ]
        results = self.concurrent(operations)
        self.assertEqual(results.count(None), 6)
        self.assertEqual(results.count("check_votes_in_other_areas_failed"), 6)

    def test_discarded_votes_do_not_consume_eligibility(self):
        """Allow a valid ballot after a discarded ballot in another area."""
        self.election.vote(
            self.connection, "discarded", self.election.other_area, "discarded"
        )
        self.election.vote(self.connection, "discarded", self.election.area)

    def test_duplicate_endpoint_fails_without_changing_deadline(self):
        """Reject an ambiguous endpoint and retain the last valid deadline."""
        self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        with self.assertRaises(psycopg.errors.UniqueViolation):
            self.election.schedule(self.connection, "END", "2026-10-02T12:00:00Z")
        self.assertEqual(self.dates(), (None, "2026-10-01T12:00:00Z"))

    def test_index_replacement_stops_on_an_invalid_previous_build(self):
        """Retain the valid old index after an interrupted build and verify the documented recovery."""
        self.election.vote(self.connection, "index-test")
        self.election.vote(self.connection, "index-test")
        with self.assertRaises(psycopg.errors.UniqueViolation):
            self.connection.execute(
                """
                CREATE UNIQUE INDEX CONCURRENTLY cast_vote_participation_election_covering_idx
                ON sequent_backend.cast_vote
                    (tenant_id, election_event_id, election_id, voter_id_string)
            """
            )
        validity = "SELECT indisvalid FROM pg_index WHERE indexrelid = %s::regclass"
        old_index = "sequent_backend.cast_vote_participation_election_idx"
        temporary_index = (
            "sequent_backend.cast_vote_participation_election_covering_idx"
        )
        self.assertFalse(self.db.scalar(validity, (temporary_index,)))
        self.assertNotEqual(self.db.run_index_script().returncode, 0)
        self.assertTrue(self.db.scalar(validity, (old_index,)))
        self.connection.execute(
            "DROP INDEX CONCURRENTLY sequent_backend.cast_vote_participation_election_covering_idx"
        )
        result = self.db.run_index_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.db.scalar(validity, (old_index,)))

    def test_invalid_date_type_is_rejected_at_configuration_write(self):
        """Reject a non-string endpoint date when writing configuration."""
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
        """Keep both election reads correct as a schedule changes scope and active state."""
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

    def test_schedule_reads_obey_transaction_boundaries(self):
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

    def test_schedules_can_precede_election_import(self):
        """Support schedule-first imports without losing the deadline."""
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

    def test_storage_migration_is_reversible(self):
        """Verify EXTERNAL storage can be rolled back and reapplied for future writes."""
        query = """
            SELECT attstorage FROM pg_attribute
            WHERE attrelid = 'sequent_backend.cast_vote'::regclass AND attname = 'content'
        """
        self.db.apply(STORAGE_MIGRATION)
        self.assertEqual(self.db.scalar(query), "e")
        self.db.apply(STORAGE_MIGRATION, "down")
        self.assertEqual(self.db.scalar(query), "x")
        self.db.apply(STORAGE_MIGRATION)

    def test_truncate_removes_deadlines(self):
        """Truncating schedules immediately removes deadlines without maintaining derived data."""
        self.election.schedule(self.connection, "END", "2026-10-01T12:00:00Z")
        self.connection.execute("TRUNCATE sequent_backend.scheduled_event")
        self.assertEqual(self.dates(), (None, None))

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
    """Apply the schedule migration, run its database invariants and exit nonzero on failure."""
    database.apply(SCHEDULE_MIGRATION)
    VotingFlowTests.database = database
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(VotingFlowTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful():
        raise SystemExit("Voting-flow database regressions failed")
