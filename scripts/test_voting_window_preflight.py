# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Check voting-window migration diagnostics in a disposable PostgreSQL cluster."""

import json
import sys
import unittest
from pathlib import Path

import psycopg
from psycopg.types.json import Jsonb

sys.path.insert(0, str(Path(__file__).parent / "voting_flow"))
from database import WINDOW_MIGRATION, local_database
from fixtures import Election


class VotingWindowPreflightTests(unittest.TestCase):
    def setUp(self):
        """Prove a valid source migrates before adding each case's invalid rows."""
        self.cluster = local_database()
        self.database = self.cluster.__enter__()
        self.addCleanup(self.cluster.__exit__, None, None, None)
        self.connection = self.database.connection
        self.election = Election()
        self.election.create(self.connection)
        self.end = self.election.schedule(
            self.connection, "END", "2026-10-01T12:00:00Z"
        )
        # Every rejection starts with a successful migration over the valid source.
        self.database.apply(WINDOW_MIGRATION)
        self.assertEqual(
            self.connection.execute(
                "SELECT start_date, end_date FROM sequent_backend.election_voting_window"
            ).fetchall(),
            [(None, "2026-10-01T12:00:00Z")],
        )
        self.database.apply(WINDOW_MIGRATION, "down")

    def snapshot(self):
        """Read every source field the projection migration could affect."""
        return self.connection.execute(
            "SELECT id, tenant_id, election_event_id, task_id, event_payload, cron_config, archived_at "
            "FROM sequent_backend.scheduled_event ORDER BY id"
        ).fetchall()

    def test_reports_every_invalid_scope_without_changing_source_rows(self):
        """Report all invalid rows, retain the sources, and accept explicit repairs."""
        duplicate = self.election.schedule(
            self.connection, "END", "2026-10-02T12:00:00Z"
        )
        malformed = Election()
        malformed.create(self.connection)
        malformed_id = malformed.schedule(
            self.connection, "START", "2026-10-01T10:00:00Z"
        )
        self.connection.execute(
            "UPDATE sequent_backend.scheduled_event SET cron_config = %s WHERE id = %s",
            (
                Jsonb({"cron": 7, "scheduled_date": "2026-10-01T10:00:00Z"}),
                malformed_id,
            ),
        )
        invalid_date = Election()
        invalid_date.create(self.connection)
        invalid_id = invalid_date.schedule(
            self.connection, "END", "2026-02-30T12:00:00Z"
        )
        before = self.snapshot()
        with self.assertRaises(psycopg.errors.RaiseException) as failure:
            self.database.apply(WINDOW_MIGRATION)
        self.assertEqual(
            failure.exception.diag.message_primary, "invalid_voting_window_backfill"
        )
        details = json.loads(failure.exception.diag.message_detail)
        actual = {
            (
                row["tenant_id"],
                row["election_event_id"],
                row["election_id"],
                row["schedule_id"],
            ): row["problems"]
            for row in details
        }
        expected = {
            (*map(str, self.election.scope), str(self.end)): ["duplicate_active_task"],
            (*map(str, self.election.scope), str(duplicate)): ["duplicate_active_task"],
            (*map(str, malformed.scope), str(malformed_id)): ["malformed_cron_config"],
            (*map(str, invalid_date.scope), str(invalid_id)): [
                "invalid_scheduled_date"
            ],
        }
        self.assertEqual(actual, expected)
        self.assertEqual(len(details), len(expected))
        self.assertIn("Archive or correct", failure.exception.diag.message_hint)
        self.assertEqual(self.snapshot(), before)
        self.assertIsNone(
            self.database.scalar(
                "SELECT to_regclass('sequent_backend.election_voting_window')"
            )
        )

        # Archiving the ambiguous row and correcting the malformed fields is sufficient.
        self.connection.execute(
            "UPDATE sequent_backend.scheduled_event SET archived_at = now() WHERE id = %s",
            (duplicate,),
        )
        self.connection.execute(
            "UPDATE sequent_backend.scheduled_event SET cron_config = %s WHERE id IN (%s, %s)",
            (
                Jsonb({"scheduled_date": "2026-10-03T12:00:00Z"}),
                malformed_id,
                invalid_id,
            ),
        )
        self.database.apply(WINDOW_MIGRATION)
        self.assertEqual(
            self.database.scalar(
                "SELECT count(*) FROM sequent_backend.election_voting_window"
            ),
            3,
        )

    def test_archived_and_unrelated_invalid_schedules_are_not_backfilled(self):
        """Ignore malformed rows outside the migration's active task contract."""
        archived = self.election.schedule(self.connection, "END", "not-a-date")
        self.connection.execute(
            "UPDATE sequent_backend.scheduled_event SET archived_at = now() WHERE id = %s",
            (archived,),
        )
        unrelated = self.election.schedule(self.connection, "START", "not-a-date")
        self.connection.execute(
            "UPDATE sequent_backend.scheduled_event SET event_payload = %s WHERE id = %s",
            (
                Jsonb({"election_id": str(self.election.election), "unrelated": True}),
                unrelated,
            ),
        )
        before = self.snapshot()
        self.database.apply(WINDOW_MIGRATION)
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(
            self.connection.execute(
                "SELECT start_date, end_date FROM sequent_backend.election_voting_window"
            ).fetchall(),
            [(None, "2026-10-01T12:00:00Z")],
        )


if __name__ == "__main__":
    unittest.main()
