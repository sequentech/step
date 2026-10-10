# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Check that report rows stay bound to their tenant, event and election.

Run from the devcontainer: devenv shell python3 scripts/test_report_tenant_binding.py
"""
from pathlib import Path
import sys
import unittest
from uuid import uuid4

sys.path.insert(0, str(Path(__file__).parent / "voting_flow"))
import psycopg
from psycopg.types.json import Jsonb
from database import MIGRATIONS, local_database

BINDING_MIGRATION = MIGRATIONS / "1791582556997_report_tenant_binding"

REPORT_TABLE = """
    CREATE TABLE sequent_backend.report (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        election_event_id uuid NOT NULL,
        tenant_id uuid NOT NULL,
        election_id uuid,
        report_type text NOT NULL,
        cron_config jsonb
    )
"""


class Scope:
    """A tenant with one election event holding two elections."""

    def __init__(self, connection):
        self.tenant = uuid4()
        self.event = uuid4()
        self.elections = [uuid4(), uuid4()]
        connection.execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id) VALUES (%s, %s)",
            (self.event, self.tenant),
        )
        for election in self.elections:
            connection.execute(
                "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id) VALUES (%s, %s, %s)",
                (election, self.tenant, self.event),
            )


def insert_report(connection, tenant, event, election=None):
    return connection.execute(
        """
        INSERT INTO sequent_backend.report (tenant_id, election_event_id, election_id, report_type, cron_config)
        VALUES (%s, %s, %s, 'ACTIVITY_LOGS', %s)
        RETURNING id
    """,
        (tenant, event, election, Jsonb({"is_active": True, "cron_expression": "0 * * * *"})),
    ).fetchone()[0]


class ReportTenantBindingTests(unittest.TestCase):
    database = None

    def setUp(self):
        self.connection = self.database.connection
        self.own = Scope(self.connection)
        self.other = Scope(self.connection)
        self.report = insert_report(self.connection, self.own.tenant, self.own.event, self.own.elections[0])

    def update(self, assignments, parameters):
        self.connection.execute(
            f"UPDATE sequent_backend.report SET {assignments} WHERE id = %s",
            (*parameters, self.report),
        )

    def test_insert_rejects_event_of_another_tenant(self):
        with self.assertRaises(psycopg.errors.ForeignKeyViolation):
            insert_report(self.connection, self.own.tenant, self.other.event)

    def test_insert_rejects_unknown_event(self):
        with self.assertRaises(psycopg.errors.ForeignKeyViolation):
            insert_report(self.connection, self.own.tenant, uuid4())

    def test_insert_rejects_election_outside_the_event(self):
        foreign_event_same_tenant = uuid4()
        self.connection.execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id) VALUES (%s, %s)",
            (foreign_event_same_tenant, self.own.tenant),
        )
        for election in [self.other.elections[0], uuid4()]:
            with self.subTest(election=election), self.assertRaises(psycopg.errors.ForeignKeyViolation):
                insert_report(self.connection, self.own.tenant, self.own.event, election)
        with self.assertRaises(psycopg.errors.ForeignKeyViolation):
            insert_report(self.connection, self.own.tenant, foreign_event_same_tenant, self.own.elections[0])

    def test_update_rejects_tenant_change(self):
        for assignments, parameters in [
            ("tenant_id = %s", (self.other.tenant,)),
            ("tenant_id = %s, election_event_id = %s, election_id = NULL", (self.other.tenant, self.other.event)),
        ]:
            with self.subTest(assignments=assignments), self.assertRaises(psycopg.errors.CheckViolation):
                self.update(assignments, parameters)

    def test_update_rejects_event_or_election_of_another_tenant(self):
        for assignments, parameters in [
            ("election_event_id = %s, election_id = NULL", (self.other.event,)),
            ("election_id = %s", (self.other.elections[0],)),
        ]:
            with self.subTest(assignments=assignments), self.assertRaises(psycopg.errors.ForeignKeyViolation):
                self.update(assignments, parameters)

    def test_consistent_reports_keep_working(self):
        insert_report(self.connection, self.own.tenant, self.own.event)
        self.update("election_id = %s", (self.own.elections[1],))
        self.update("election_id = NULL", ())
        self.update(
            "tenant_id = %s, election_event_id = %s, cron_config = %s",
            (self.own.tenant, self.own.event, Jsonb({"is_active": False})),
        )
        self.assertEqual(
            self.connection.execute(
                "SELECT tenant_id, election_event_id, election_id, cron_config FROM sequent_backend.report WHERE id = %s",
                (self.report,),
            ).fetchone(),
            (self.own.tenant, self.own.event, None, {"is_active": False}),
        )


if __name__ == "__main__":
    with local_database() as database:
        database.connection.execute(REPORT_TABLE)
        own, other = Scope(database.connection), Scope(database.connection)
        legacy = insert_report(database.connection, own.tenant, other.event)
        database.apply(BINDING_MIGRATION)
        # Rows saved before the migration keep accepting schedule bookkeeping.
        database.connection.execute(
            "UPDATE sequent_backend.report SET cron_config = jsonb_set(cron_config, '{is_active}', 'false') WHERE id = %s",
            (legacy,),
        )
        ReportTenantBindingTests.database = database
        result = unittest.TextTestRunner(verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(ReportTenantBindingTests)
        )
        database.apply(BINDING_MIGRATION, "down")
        insert_report(database.connection, own.tenant, other.event)
        if not result.wasSuccessful():
            raise SystemExit(1)
        print("Report tenant binding migration and rollback passed")
