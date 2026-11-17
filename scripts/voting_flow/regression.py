# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Database invariants tested through the real migration and production query."""

from concurrent.futures import ThreadPoolExecutor
import threading
from pathlib import Path
import subprocess
import unittest
from uuid import uuid4

import psycopg
from psycopg.types.json import Jsonb

from database import (
    AREA_MIGRATION,
    CONFIGURATION_QUERY,
    RESENT_BALLOT_MIGRATION,
    STORAGE_MIGRATION,
    SCHEDULE_MIGRATION,
    VOTER_BALLOT_KEY,
    VOTER_BALLOT_KEY_MIGRATION,
)
from fixtures import INSERT_VOTE, Election

KEY_COLUMNS = ["ballot_id", "election_id", "voter_id_string"]


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
        # Reapplying the area migration restores its older check_revote_limit.
        self.db.apply(RESENT_BALLOT_MIGRATION)

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

    def test_concurrent_resends_store_one_vote(self):
        """Store a ballot resent by twelve racing requests once, even with unlimited revotes."""
        self.connection.execute(
            "UPDATE sequent_backend.election SET num_allowed_revotes = 0 WHERE id = %s",
            (self.election.election,),
        )
        results = self.concurrent(
            [
                lambda connection: self.election.vote(
                    connection, "resent", ballot_id="resent-ballot"
                )
                for _ in range(12)
            ]
        )
        self.assertEqual(results.count(None), 1)
        self.assertEqual(results.count("ballot_already_cast"), 11)

    def test_resent_ballot_is_refused_in_any_status(self):
        """Refuse a resent ballot even after Datafix discarded the stored one."""
        self.election.vote(
            self.connection, "discarded-resend", status="discarded", ballot_id="kept"
        )
        with self.assertRaises(psycopg.errors.RaiseException) as refused:
            self.election.vote(self.connection, "discarded-resend", ballot_id="kept")
        self.assertEqual(refused.exception.diag.message_primary, "ballot_already_cast")
        self.election.vote(self.connection, "discarded-resend", ballot_id="new")

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

    def key_shape(self):
        """Return validity, uniqueness and key/total column counts of the voter ballot key."""
        return self.connection.execute(
            """
            SELECT indisvalid, indisunique, indnkeyatts, indnatts
            FROM pg_index WHERE indexrelid = to_regclass(%s)
        """,
            (VOTER_BALLOT_KEY,),
        ).fetchone()

    def nullable_key_columns(self):
        """Return the voter ballot key columns that still accept NULL."""
        return self.db.scalar(
            """
            SELECT coalesce(array_agg(attname::text ORDER BY attname), '{}')
            FROM pg_attribute
            WHERE attrelid = 'sequent_backend.cast_vote'::regclass
              AND attname = ANY(%s) AND NOT attnotnull
        """,
            (KEY_COLUMNS,),
        )

    def not_null_check(self):
        """Return whether the script's NULL check is validated, or None when it is absent."""
        row = self.connection.execute(
            """
            SELECT convalidated FROM pg_constraint
            WHERE conrelid = 'sequent_backend.cast_vote'::regclass
              AND conname = 'cast_vote_voter_ballot_not_null'
        """
        ).fetchone()
        return row and row[0]

    def insert_null_key(self, column):
        """Insert a vote with one NULL key column, bypassing check_revote_limit."""
        values = {
            "election_id": self.election.election,
            "voter_id_string": "null-key",
            "ballot_id": str(uuid4()),
        }
        values[column] = None
        with self.connection.transaction():
            self.connection.execute("SET LOCAL session_replication_role = replica")
            self.connection.execute(
                INSERT_VOTE,
                (
                    self.election.tenant,
                    self.election.event,
                    values["election_id"],
                    values["voter_id_string"],
                    self.election.area,
                    "valid",
                    "ballot",
                    values["ballot_id"],
                    bytes(64),
                ),
            )

    def delete_null_keys(self):
        """Delete this test's votes with a NULL key column."""
        self.connection.execute(
            """
            DELETE FROM sequent_backend.cast_vote
            WHERE tenant_id = %s AND election_event_id = %s
              AND (election_id IS NULL OR voter_id_string IS NULL OR ballot_id IS NULL)
        """,
            (self.election.tenant, self.election.event),
        )

    def restore_voter_ballot_key(self):
        """Remove this test's NULL-key rows and any invalid key, then reapply the migration."""
        self.delete_null_keys()
        shape = self.key_shape()
        if shape is not None and not shape[0]:
            self.connection.execute(f"DROP INDEX {VOTER_BALLOT_KEY}")
        self.db.apply(VOTER_BALLOT_KEY_MIGRATION)

    def test_index_script_proves_no_nulls_before_the_migration(self):
        """Validate the NULL check without blocking votes so the migration's SET NOT NULL skips its scan."""
        messages = []

        def collect(notice):
            messages.append(notice.message_primary)

        self.db.apply(VOTER_BALLOT_KEY_MIGRATION, "down")
        try:
            result = self.db.run_index_script()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIs(self.not_null_check(), True)
            self.connection.add_notice_handler(collect)
            self.connection.execute("SET client_min_messages = debug1")
            try:
                self.db.apply(VOTER_BALLOT_KEY_MIGRATION)
            finally:
                self.connection.execute("RESET client_min_messages")
                self.connection.remove_notice_handler(collect)
        finally:
            self.restore_voter_ballot_key()
        for column in KEY_COLUMNS:
            self.assertIn(
                f'existing constraints on column "cast_vote.{column}" are sufficient '
                "to prove that it does not contain nulls",
                messages,
            )
        self.assertIsNone(self.not_null_check())
        self.assertEqual(self.nullable_key_columns(), [])

    def test_index_script_refuses_null_key_columns(self):
        """Stop on an existing NULL key column while refusing new ones, then finish once it is gone."""
        self.db.apply(VOTER_BALLOT_KEY_MIGRATION, "down")
        try:
            self.insert_null_key("ballot_id")
            failed = self.db.run_index_script()
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("is violated by some row", failed.stderr)
            self.assertIs(self.not_null_check(), False)
            self.assertIsNone(self.key_shape())
            with self.assertRaises(psycopg.errors.CheckViolation):
                self.insert_null_key("voter_id_string")
            self.delete_null_keys()
            result = self.db.run_index_script()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIs(self.not_null_check(), True)
        finally:
            self.restore_voter_ballot_key()
        self.assertIsNone(self.not_null_check())
        self.assertEqual(self.key_shape(), (True, True, 5, 7))

    def test_index_script_recovers_from_a_failed_build_before_the_migration(self):
        """Keep the old index while a failed build is invalid, then build the key the migration reuses."""
        participation = "sequent_backend.cast_vote_participation_election_idx"
        exists = "SELECT to_regclass(%s) IS NOT NULL"
        key_oid = "SELECT to_regclass(%s)::oid"
        resent = """
            DELETE FROM sequent_backend.cast_vote WHERE id = (
                SELECT id FROM sequent_backend.cast_vote
                WHERE election_id = %s AND voter_id_string = 'index-test' LIMIT 1
            )
        """
        self.db.apply(VOTER_BALLOT_KEY_MIGRATION, "down")
        try:
            self.election.vote(self.connection, "index-test", ballot_id="resent")
            with self.connection.transaction():
                self.connection.execute("SET LOCAL session_replication_role = replica")
                self.election.vote(self.connection, "index-test", ballot_id="resent")
            failed = self.db.run_index_script()
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("could not create unique index", failed.stderr)
            self.assertEqual(self.key_shape(), (False, True, 5, 7))
            self.assertTrue(self.db.scalar(exists, (participation,)))
            rerun = self.db.run_index_script()
            self.assertNotEqual(rerun.returncode, 0)
            self.assertIn("cast_vote_voter_ballot_key is invalid", rerun.stderr)
            self.connection.execute(resent, (self.election.election,))
            self.connection.execute(f"DROP INDEX CONCURRENTLY {VOTER_BALLOT_KEY}")
            result = self.db.run_index_script()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(self.db.scalar(exists, (participation,)))
            built = self.db.scalar(key_oid, (VOTER_BALLOT_KEY,))
        finally:
            if self.db.scalar(
                "SELECT count(*) FROM sequent_backend.cast_vote "
                "WHERE election_id = %s AND voter_id_string = 'index-test'",
                (self.election.election,),
            ) > 1:
                self.connection.execute(resent, (self.election.election,))
            self.restore_voter_ballot_key()
        self.assertEqual(self.db.scalar(key_oid, (VOTER_BALLOT_KEY,)), built)
        self.assertEqual(self.key_shape(), (True, True, 5, 7))

    def test_index_script_keeps_the_key_the_migration_built(self):
        """Running the script after the migration, or again, neither rebuilds nor drops the key."""
        key_oid = "SELECT to_regclass(%s)::oid"
        built = self.db.scalar(key_oid, (VOTER_BALLOT_KEY,))
        for _ in range(2):
            result = self.db.run_index_script()
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.db.scalar(key_oid, (VOTER_BALLOT_KEY,)), built)
        self.assertEqual(self.key_shape(), (True, True, 5, 7))
        self.assertIsNone(self.not_null_check())

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

    def test_resent_ballot_migration_is_reversible(self):
        """After rollback the trigger lets a resent ballot through to the unique key; reapplied, it refuses it."""
        self.election.vote(self.connection, "rollback", ballot_id="kept")
        self.db.apply(RESENT_BALLOT_MIGRATION, "down")
        try:
            with self.assertRaises(psycopg.errors.UniqueViolation):
                self.election.vote(self.connection, "rollback", ballot_id="kept")
        finally:
            self.db.apply(RESENT_BALLOT_MIGRATION)
        with self.assertRaises(psycopg.errors.RaiseException):
            self.election.vote(self.connection, "rollback", ballot_id="kept")

    def test_voter_ballot_key_refuses_a_resend_on_any_write_path(self):
        """Store a voter's ballot once even when a write bypasses the revote trigger."""

        def bypassing_trigger(voter, ballot_id):
            with self.connection.transaction():
                self.connection.execute("SET LOCAL session_replication_role = replica")
                self.election.vote(self.connection, voter, ballot_id=ballot_id)

        self.election.vote(self.connection, "key", ballot_id="kept")
        self.election.vote(self.connection, "another-voter", ballot_id="kept")
        with self.assertRaises(psycopg.errors.UniqueViolation) as refused:
            bypassing_trigger("key", "kept")
        self.assertEqual(
            refused.exception.diag.constraint_name, "cast_vote_voter_ballot_key"
        )
        bypassing_trigger("key", "revote")
        with self.assertRaises(psycopg.errors.UniqueViolation):
            self.connection.execute(
                """
                UPDATE sequent_backend.cast_vote SET ballot_id = 'kept'
                WHERE election_id = %s AND voter_id_string = 'key' AND ballot_id = 'revote'
            """,
                (self.election.election,),
            )
        self.assertEqual(
            self.db.scalar(
                """
                SELECT array_agg(ballot_id ORDER BY ballot_id) FROM sequent_backend.cast_vote
                WHERE election_id = %s AND voter_id_string = 'key'
            """,
                (self.election.election,),
            ),
            ["kept", "revote"],
        )
        for column in KEY_COLUMNS:
            with self.subTest(column=column):
                with self.assertRaises(psycopg.errors.NotNullViolation):
                    self.insert_null_key(column)

    def test_voter_ballot_key_migration_refuses_null_key_columns(self):
        """Stop the migration on a NULL key column and apply it once the row is gone."""
        self.db.apply(VOTER_BALLOT_KEY_MIGRATION, "down")
        try:
            for column in KEY_COLUMNS:
                with self.subTest(column=column):
                    self.insert_null_key(column)
                    with self.assertRaises(psycopg.errors.NotNullViolation):
                        self.db.apply(VOTER_BALLOT_KEY_MIGRATION)
                    self.assertEqual(self.nullable_key_columns(), KEY_COLUMNS)
                    self.delete_null_keys()
        finally:
            self.restore_voter_ballot_key()
        self.assertEqual(self.nullable_key_columns(), [])
        self.assertEqual(self.key_shape(), (True, True, 5, 7))

    def test_voter_ballot_key_migration_is_reversible(self):
        """Restore the participation index on rollback and refuse to accept an invalid key on reapply."""
        participation = "sequent_backend.cast_vote_participation_election_idx"
        exists = "SELECT to_regclass(%s) IS NOT NULL"
        self.election.vote(self.connection, "invalid-key")
        self.election.vote(self.connection, "invalid-key")
        self.db.apply(VOTER_BALLOT_KEY_MIGRATION, "down")
        try:
            self.assertFalse(self.db.scalar(exists, (VOTER_BALLOT_KEY,)))
            self.assertTrue(self.db.scalar(exists, (participation,)))
            self.assertEqual(self.nullable_key_columns(), KEY_COLUMNS)
            with self.assertRaises(psycopg.errors.UniqueViolation):
                self.connection.execute(
                    """
                    CREATE UNIQUE INDEX CONCURRENTLY cast_vote_voter_ballot_key
                    ON sequent_backend.cast_vote
                        (tenant_id, election_event_id, election_id, voter_id_string)
                """
                )
            with self.assertRaises(psycopg.errors.RaiseException):
                self.db.apply(VOTER_BALLOT_KEY_MIGRATION)
        finally:
            self.restore_voter_ballot_key()
        self.assertEqual(self.key_shape(), (True, True, 5, 7))
        self.assertFalse(self.db.scalar(exists, (participation,)))
        self.assertEqual(self.nullable_key_columns(), [])

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
