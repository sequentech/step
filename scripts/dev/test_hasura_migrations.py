# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Check the migration catalog constraints enforced by Hasura CLI at startup."""

from collections import defaultdict
import os
from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[2]


class HasuraMigrationsTest(unittest.TestCase):
    def test_migration_versions_are_unique_within_each_source(self):
        sources = list((ROOT / "hasura/migrations").iterdir())
        self.assertTrue(sources, "the deployment must include database migrations")
        for source in sources:
            if not source.is_dir():
                continue
            versions = defaultdict(list)
            for migration in source.iterdir():
                if migration.is_dir() and (migration / "up.sql").is_file():
                    versions[int(migration.name.split("_", 1)[0])].append(
                        migration.name
                    )
            duplicates = {
                version: names for version, names in versions.items() if len(names) > 1
            }
            with self.subTest(source=source.name):
                self.assertFalse(
                    duplicates, f"Hasura refuses duplicate migrations: {duplicates}"
                )

    @unittest.skipUnless(
        os.environ.get("ELECTORAL_LOG_TEST_DATABASE_URL"),
        "requires a disposable PostgreSQL test database and psql",
    )
    def test_transmission_upgrade_preserves_existing_readiness_audit_records(self):
        migration = (
            ROOT
            / "hasura/migrations/backend-db"
            / "1791700000000_audit_log_outbox_transmission_downloads/up.sql"
        ).read_text()
        sql = """
BEGIN;
CREATE TEMP TABLE audit_upgrade_fixture (
    statement_kind text CONSTRAINT audit_log_outbox_statement_kind_known CHECK (
        statement_kind IN ('VotersImported', 'ReadinessVerified')
    )
);
INSERT INTO audit_upgrade_fixture VALUES ('VotersImported'), ('ReadinessVerified');
"""
        sql += migration.replace(
            "sequent_backend.audit_log_outbox", "pg_temp.audit_upgrade_fixture"
        )
        sql += """
INSERT INTO audit_upgrade_fixture VALUES
    ('TransmissionFileDownloaded'), ('VotersImported'), ('ReadinessVerified');
SELECT count(*) FROM audit_upgrade_fixture;
ROLLBACK;
"""
        result = subprocess.run(
            [
                "psql",
                "--dbname",
                os.environ["ELECTORAL_LOG_TEST_DATABASE_URL"],
                "-X",
                "-At",
                "-v",
                "ON_ERROR_STOP=1",
            ],
            input=sql,
            text=True,
            capture_output=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("5", result.stdout.splitlines())


if __name__ == "__main__":
    unittest.main()
