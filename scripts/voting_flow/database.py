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
RESULTS = ROOT / ".cache/voting-flow"
MIGRATIONS = ROOT / "hasura/migrations/backend-db"
AREA_MIGRATION = MIGRATIONS / "1788765000000_serialize_cast_vote_area_checks"
STORAGE_MIGRATION = MIGRATIONS / "1788765000001_cast_vote_external_storage"
SCHEDULE_MIGRATION = MIGRATIONS / "1788765000002_validate_voting_schedules"
SCHEDULE_INDEX = "sequent_backend.scheduled_event_active_scope_task_idx"
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
        """Configure a disposable cluster and private socket without starting PostgreSQL."""
        self.directory = Path(directory)
        self.data = self.directory / "data"
        self.connection = None
        self.started = False
        # A private Unix socket allows independent runs to reuse this port.
        self.dsn = f"host={directory} port=55432 dbname=postgres"
        self.env = dict(
            os.environ, PGHOST=directory, PGPORT="55432", PGDATABASE="postgres"
        )

    def command(self, *arguments):
        """Run a PostgreSQL utility, suppressing normal output and raising on failure."""
        subprocess.run(arguments, check=True, stdout=subprocess.DEVNULL)

    def start(self):
        """Initialize the fixture schema and instrumented PostgreSQL cluster on a private socket."""
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
            # The largest baseline uses 64 writer + 64 identity connections.
            "-c pg_stat_statements.track=all -c max_connections=160"
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
        self.started = True
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
        """Close the client and stop a started cluster, including after partial initialization."""
        if self.connection is not None:
            self.connection.close()
        if self.started:
            self.command(
                "pg_ctl", "-D", str(self.data), "-m", "immediate", "-w", "stop"
            )
            self.started = False

    def apply(self, migration, direction="up"):
        # Production Hasura migrations are transactional. Keep that contract in
        # the harness, including constraint and index installation.
        """Apply the selected up/down migration atomically, matching Hasura transaction semantics."""
        with self.connection.transaction():
            self.connection.execute((migration / f"{direction}.sql").read_text())

    def run_index_script(self):
        """Run the concurrent index script outside a transaction and return its captured result."""
        return subprocess.run(
            ["psql", "-X", "-f", str(INDEX_SCRIPT)],
            env=self.env,
            text=True,
            capture_output=True,
        )

    def scalar(self, query, parameters=None):
        """Execute parameterized SQL and return the first column of its first result row."""
        return self.connection.execute(query, parameters).fetchone()[0]


@contextmanager
def local_database():
    """Yield an isolated database and always stop it before removing its temporary files."""
    with tempfile.TemporaryDirectory(prefix="voting-flow-") as directory:
        database = LocalDatabase(directory)
        try:
            # Startup can fail after PostgreSQL launches (for example, a bad
            # migration). Stop it before removing its temporary data directory.
            database.start()
            yield database
        finally:
            database.stop()
