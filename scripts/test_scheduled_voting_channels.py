# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Check schedule payload compatibility in a disposable PostgreSQL cluster.

Run from the devcontainer: devenv shell python3 scripts/test_scheduled_voting_channels.py
"""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parent / "voting_flow"))
import psycopg
from psycopg.types.json import Jsonb
from database import CONFIGURATION_QUERY, MIGRATIONS, SCHEDULE_MIGRATION, local_database
from fixtures import Election

CHANNELS_MIGRATION = MIGRATIONS / "1789420000000_scheduled_voting_channels"


class ScheduledChannelTests(unittest.TestCase):
    database = None

    def setUp(self):
        self.connection = self.database.connection
        self.election = Election()
        self.election.create(self.connection)

    def test_missing_null_empty_and_explicit_channel_deadlines(self):
        for endpoint, index in [("START", 3), ("END", 4)]:
            schedule = self.election.schedule(self.connection, endpoint, "2027-01-01T12:00:00Z")
            for channels in [None, [], ["ONLINE", "KIOSK"], ["TELEPHONE"], ["KIOSK", "EARLY_VOTING"]]:
                with self.subTest(endpoint=endpoint, channels=channels):
                    self.connection.execute(
                        "UPDATE sequent_backend.scheduled_event SET event_payload = %s WHERE id = %s",
                        (Jsonb({"election_id": str(self.election.election), "voting_channels": channels}), schedule),
                    )
                    row = self.connection.execute(CONFIGURATION_QUERY, self.election.scope).fetchone()
                    expected = "2027-01-01T12:00:00Z" if not channels or "ONLINE" in channels else None
                    self.assertEqual(row[index], expected)
            # An old client can change only the date without losing the saved selection.
            self.connection.execute(
                "UPDATE sequent_backend.scheduled_event SET cron_config = %s WHERE id = %s",
                (Jsonb({"scheduled_date": "2027-02-01T12:00:00Z"}), schedule),
            )
            self.assertEqual(self.connection.execute(
                "SELECT event_payload -> 'voting_channels' FROM sequent_backend.scheduled_event WHERE id = %s", (schedule,),
            ).fetchone()[0], ["KIOSK", "EARLY_VOTING"])

    def test_invalid_channel_values_and_foreign_election_remain_rejected(self):
        schedule = self.election.schedule(self.connection, "END", "2027-01-01T12:00:00Z")
        for channels in ["ONLINE", ["INVALID"], [None], [1]]:
            with self.subTest(channels=channels), self.assertRaises(psycopg.errors.CheckViolation):
                self.connection.execute(
                    "UPDATE sequent_backend.scheduled_event SET event_payload = %s WHERE id = %s",
                    (Jsonb({"election_id": str(self.election.election), "voting_channels": channels}), schedule),
                )
        with self.assertRaises(psycopg.errors.CheckViolation):
            self.connection.execute(
                "UPDATE sequent_backend.scheduled_event SET event_payload = %s WHERE id = %s",
                (Jsonb({"election_id": "wrong-election", "voting_channels": ["ONLINE"]}), schedule),
            )


if __name__ == "__main__":
    with local_database() as database:
        database.apply(SCHEDULE_MIGRATION)
        election = Election()
        election.create(database.connection)
        legacy = election.schedule(database.connection, "END", "2027-01-01T12:00:00Z")
        original = database.connection.execute(CONFIGURATION_QUERY, election.scope).fetchone()
        database.apply(CHANNELS_MIGRATION)
        assert database.connection.execute(CONFIGURATION_QUERY, election.scope).fetchone() == original
        ScheduledChannelTests.database = database
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(ScheduledChannelTests))
        explicit = Election()
        explicit.create(database.connection)
        explicit_schedule = explicit.schedule(database.connection, "END", "2027-01-01T12:00:00Z")
        database.connection.execute(
            "UPDATE sequent_backend.scheduled_event SET event_payload = %s WHERE id = %s",
            (Jsonb({"election_id": str(explicit.election), "voting_channels": ["KIOSK", "EARLY_VOTING"]}), explicit_schedule),
        )
        database.apply(CHANNELS_MIGRATION, "down")
        assert database.connection.execute(CONFIGURATION_QUERY, election.scope).fetchone() == original
        # Rolled-back schedules return to the legacy payload, so the restored
        # constraint still accepts the updates Windmill makes when they run.
        assert database.connection.execute(
            "SELECT event_payload FROM sequent_backend.scheduled_event WHERE id = %s", (explicit_schedule,),
        ).fetchone()[0] == {"election_id": str(explicit.election)}
        database.connection.execute(
            "UPDATE sequent_backend.scheduled_event SET stopped_at = now() WHERE id = %s", (explicit_schedule,),
        )
        assert database.connection.execute(
            "SELECT convalidated FROM pg_constraint WHERE conname = 'scheduled_event_voting_period_valid'"
        ).fetchone()[0]
        database.apply(CHANNELS_MIGRATION)
        if not result.wasSuccessful():
            raise SystemExit(1)
