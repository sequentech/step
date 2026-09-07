# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""An isolated PostgreSQL cluster shared by regression tests and benchmarks."""

from contextlib import contextmanager
from pathlib import Path
import os
import subprocess
import tempfile

import psycopg

ROOT = Path(__file__).resolve().parents[2]
MIGRATIONS = ROOT / "hasura/migrations/backend-db"
AREA_MIGRATION = MIGRATIONS / "1788765000000_serialize_cast_vote_area_checks"
STORAGE_MIGRATION = MIGRATIONS / "1788765000001_cast_vote_external_storage"
WINDOW_MIGRATION = MIGRATIONS / "1788765000002_materialize_voting_windows"
INDEX_SCRIPT = ROOT / "scripts/postgres/cast_vote_covering_index.sql"
CONFIGURATION_QUERY = (
    (ROOT / "packages/windmill/src/postgres/sql/cast_vote_configuration.sql")
    .read_text()
    .replace("$1", "%s")
    .replace("$2", "%s")
    .replace("$3", "%s")
)


class LocalDatabase:
    def __init__(self, directory):
        self.directory = Path(directory)
        self.data = self.directory / "data"
        # A private Unix socket allows independent runs to reuse this port.
        self.dsn = f"host={directory} port=55432 dbname=postgres"
        self.env = dict(
            os.environ, PGHOST=directory, PGPORT="55432", PGDATABASE="postgres"
        )

    def command(self, *arguments):
        subprocess.run(arguments, check=True, stdout=subprocess.DEVNULL)

    def start(self):
        self.command(
            "initdb",
            "-D",
            str(self.data),
            "-A",
            "trust",
            "--no-locale",
            "--encoding=UTF8",
        )
        options = (
            f"-k {self.directory} -p 55432 -c listen_addresses='' "
            "-c shared_preload_libraries=pg_stat_statements "
            "-c pg_stat_statements.track=all"
        )
        self.command(
            "pg_ctl",
            "-D",
            str(self.data),
            "-l",
            str(self.directory / "postgres.log"),
            "-o",
            options,
            "-w",
            "start",
        )
        self.connection = psycopg.connect(self.dsn, autocommit=True)
        self.connection.execute((Path(__file__).parent / "schema.sql").read_text())
        self.apply(AREA_MIGRATION)
        self.connection.execute(
            """
            CREATE TRIGGER check_revote_limit_trigger
            BEFORE INSERT ON sequent_backend.cast_vote
            FOR EACH ROW EXECUTE FUNCTION check_revote_limit()
        """
        )

    def stop(self):
        self.connection.close()
        self.command("pg_ctl", "-D", str(self.data), "-m", "immediate", "-w", "stop")

    def apply(self, migration, direction="up"):
        # Production Hasura migrations are transactional. Keep that contract in
        # the harness, especially for projection backfill and trigger installation.
        with self.connection.transaction():
            self.connection.execute((migration / f"{direction}.sql").read_text())

    def run_index_script(self):
        return subprocess.run(
            ["psql", "-X", "-f", str(INDEX_SCRIPT)],
            env=self.env,
            text=True,
            capture_output=True,
        )

    def scalar(self, query, parameters=None):
        return self.connection.execute(query, parameters).fetchone()[0]


@contextmanager
def local_database():
    with tempfile.TemporaryDirectory(prefix="voting-flow-") as directory:
        database = LocalDatabase(directory)
        database.start()
        try:
            yield database
        finally:
            database.stop()
