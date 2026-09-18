# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Test real publication uploads against disposable PostgreSQL and local private S3."""
import os
import getpass
import subprocess
from voting_flow.database import local_database, ROOT

if __name__ == "__main__":
    with local_database() as database:
        result = subprocess.run(
            [
                "cargo",
                "test",
                "--jobs",
                "1",
                "-p",
                "windmill",
                "--lib",
                "services::ballot_styles::publication_files::tests::publication_objects_and_authorized_references",
                "--",
                "--ignored",
                "--exact",
            ],
            cwd=ROOT / "packages",
            env={
                **{key: value for key, value in os.environ.items() if not key.startswith("HASURA_DB__")},
                "BALLOT_FILES_TEST_DSN": database.dsn,
                "HASURA_DB__HOST": str(database.directory),
                "HASURA_DB__PORT": "55432",
                "HASURA_DB__DBNAME": "postgres",
                "HASURA_DB__USER": getpass.getuser(),
                "HASURA_DB__SSL_MODE": "Disable",
                "CARGO_TARGET_DIR": os.environ.get("CARGO_TARGET_DIR", str(ROOT / "packages/windmill/rust-local-target")),
            },
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )

        print(result.stdout)
        if "1 passed; 0 failed" not in result.stdout:
            raise RuntimeError("The publication object regression did not execute")

        migration = (
            ROOT
            / "hasura/migrations/backend-db/1788808561206_ballot_style_voter_reference_index"
        )
        database.connection.execute((migration / "up.sql").read_text())
        valid = database.connection.execute(
            "SELECT indisvalid FROM pg_index WHERE indexrelid='sequent_backend.ballot_style_voter_reference_idx'::regclass"
        ).fetchone()
        assert valid == (True,)
        database.connection.execute((migration / "down.sql").read_text())
        assert database.connection.execute(
            "SELECT to_regclass('sequent_backend.ballot_style_voter_reference_idx')"
        ).fetchone() == (None,)
        print("Voter reference index migration and rollback passed")
