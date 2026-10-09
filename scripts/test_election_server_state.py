# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Check which election values Hasura roles can change, in a disposable PostgreSQL cluster.

Run inside devenv: python3 scripts/test_election_server_state.py
"""

import json
from pathlib import Path
import sys
import unittest
from uuid import uuid4

sys.path.insert(0, str(Path(__file__).parent / "voting_flow"))
import psycopg
from psycopg.types.json import Jsonb
from database import MIGRATIONS, local_database
from fixtures import Election

GUARD_MIGRATION = MIGRATIONS / "1791530000000_guard_election_server_state"
EDITOR_ROLES = [
    "admin-user",
    "election-edit",
    "election-event-write",
    "election-write",
    "permission-label-write",
]


class ElectionServerStateTests(unittest.TestCase):
    database = None

    def setUp(self):
        self.connection = self.database.connection
        # Created by the server: online voting open, online enabled, three revotes.
        self.election = Election()
        self.election.create(self.connection)

    def update(self, assignments, parameters=(), role=None):
        """Update the fixture election, as a Hasura role when one is given."""
        with self.connection.transaction():
            if role is not None:
                self.connection.execute(
                    "SELECT set_config('hasura.user', %s, true)",
                    (json.dumps({"x-hasura-role": role, "x-hasura-user-id": str(uuid4())}),),
                )
            self.connection.execute(
                f"UPDATE sequent_backend.election SET {assignments} WHERE id = %s",
                (*parameters, self.election.election),
            )

    def insert(self, role, status=None, keys_ceremony_id=None):
        """Insert a new election as a Hasura role."""
        with self.connection.transaction():
            self.connection.execute(
                "SELECT set_config('hasura.user', %s, true)",
                (json.dumps({"x-hasura-role": role}),),
            )
            self.connection.execute(
                """
                INSERT INTO sequent_backend.election
                    (id, tenant_id, election_event_id, status, keys_ceremony_id)
                VALUES (%s, %s, %s, %s, %s)
                """,
                (uuid4(), self.election.tenant, self.election.event, status, keys_ceremony_id),
            )

    def column(self, name):
        return self.connection.execute(
            f"SELECT {name} FROM sequent_backend.election WHERE id = %s",
            (self.election.election,),
        ).fetchone()[0]

    def test_editor_roles_cannot_change_voting_status(self):
        for role in EDITOR_ROLES:
            for status in [{"voting_status": "CLOSED"}, {"voting_status": "OPEN", "is_published": True}, None]:
                with self.subTest(role=role, status=status), self.assertRaises(psycopg.errors.InsufficientPrivilege):
                    self.update("status = %s", (Jsonb(status) if status is not None else None,), role)
        self.assertEqual(self.column("status"), {"voting_status": "OPEN"})

    def test_server_writes_change_voting_status(self):
        self.update("status = %s", (Jsonb({"voting_status": "PAUSED"}),))
        self.update("status = %s", (Jsonb({"voting_status": "CLOSED"}),), "service-account")
        self.update("status = %s", (Jsonb({"voting_status": "OPEN"}),), "admin")
        self.assertEqual(self.column("status"), {"voting_status": "OPEN"})

    def test_editor_roles_change_allow_tally(self):
        self.update("status = status || %s", (Jsonb({"allow_tally": "DISALLOWED"}),), "election-write")
        self.assertEqual(self.column("status"), {"voting_status": "OPEN", "allow_tally": "DISALLOWED"})
        self.update("status = NULL")
        self.update("status = %s", (Jsonb({"allow_tally": "ALLOWED"}),), "election-write")

    def test_editor_roles_resave_unchanged_values(self):
        self.update("num_allowed_revotes = NULL")
        for role in EDITOR_ROLES:
            with self.subTest(role=role):
                self.update(
                    "status = status, voting_channels = %s, num_allowed_revotes = 1, "
                    "keys_ceremony_id = keys_ceremony_id",
                    (Jsonb({"online": True, "kiosk": False, "telephone": False}),),
                    role,
                )

    def test_editor_roles_cannot_change_key_ceremony(self):
        self.update("status = NULL")
        for role in EDITOR_ROLES:
            with self.subTest(role=role), self.assertRaises(psycopg.errors.InsufficientPrivilege):
                self.update("keys_ceremony_id = %s", (uuid4(),), role)
        ceremony = uuid4()
        self.update("keys_ceremony_id = %s", (ceremony,))
        self.assertEqual(self.column("keys_ceremony_id"), ceremony)

    def test_started_channel_and_revote_limit_are_fixed(self):
        for assignment, value in [
            ("voting_channels = %s", Jsonb({"online": False})),
            ("voting_channels = %s", None),
            ("num_allowed_revotes = %s", 5),
        ]:
            with self.subTest(assignment=assignment, value=value), self.assertRaises(psycopg.errors.InsufficientPrivilege):
                self.update(assignment, (value,), "election-write")
        self.update("voting_channels = %s, num_allowed_revotes = 5", (Jsonb({"online": False}),))

    def test_channels_and_revote_limit_change_before_voting_starts(self):
        self.update("voting_channels = %s", (Jsonb({"online": True, "kiosk": True}),), "election-write")
        self.update("status = %s", (Jsonb({"voting_status": "NOT_STARTED", "kiosk_voting_status": "NOT_STARTED"}),))
        self.update("voting_channels = %s, num_allowed_revotes = 5", (Jsonb({"online": False}),), "election-write")
        self.assertEqual(self.column("num_allowed_revotes"), 5)

    def test_human_inserts_start_without_voting_or_key_ceremony(self):
        self.insert("admin-user")
        self.insert("election-event-write", Jsonb({"allow_tally": "ALLOWED"}))
        for status, ceremony in [
            (Jsonb({"voting_status": "OPEN"}), None),
            (Jsonb({"voting_status": "NOT_STARTED"}), None),
            (None, uuid4()),
        ]:
            with self.subTest(status=status, ceremony=ceremony), self.assertRaises(psycopg.errors.InsufficientPrivilege):
                self.insert("admin-user", status, ceremony)


if __name__ == "__main__":
    with local_database() as database:
        database.connection.execute("ALTER TABLE sequent_backend.election ADD COLUMN keys_ceremony_id uuid")
        database.apply(GUARD_MIGRATION)
        ElectionServerStateTests.database = database
        result = unittest.TextTestRunner(verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(ElectionServerStateTests)
        )
        database.apply(GUARD_MIGRATION, "down")
        election = Election()
        election.create(database.connection)
        with database.connection.transaction():
            database.connection.execute(
                "SELECT set_config('hasura.user', %s, true)", (json.dumps({"x-hasura-role": "admin-user"}),)
            )
            database.connection.execute(
                "UPDATE sequent_backend.election SET status = NULL WHERE id = %s", (election.election,)
            )
        if not result.wasSuccessful():
            raise SystemExit(1)
