# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Verify profile privacy, truthful reports and actual PostgreSQL JSON collection."""

import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import unittest

import psycopg

from capture import collect_logs, log_positions
from resources import extract
from report import render


def entry(
    method: str = "GET", mime: str = "text/css", body: dict | None = None
) -> dict:
    """Build a HAR request with secrets that must never become replay credentials."""
    return {
        "startedDateTime": "2026-09-07T12:00:00Z",
        "time": 12,
        "request": {
            "method": method,
            "url": "https://example.test/style.css?token=SECRET",
            "headers": [{"name": "Cookie", "value": "SECRET"}],
            "postData": {"text": json.dumps(body or {})},
        },
        "response": {"status": 200, "bodySize": 42, "content": {"mimeType": mime}},
    }


class ResourceTests(unittest.TestCase):
    """Prevent protocol mutations or session secrets leaking into optional resources."""

    def test_standard_har_mime_and_secret_removal(self) -> None:
        """Extract real Playwright HAR without requiring a Chromium-only extension."""
        result = extract({"log": {"entries": [entry()]}})
        self.assertEqual(result["resources"][0]["kind"], "stylesheet")
        self.assertEqual(result["resources"][0]["query_keys"], ["token"])
        self.assertNotIn("SECRET", json.dumps(result))

    def test_mutation_and_login_post_are_excluded(self) -> None:
        """A load recipe must not cast again or replay an old authentication form."""
        cast = entry(
            "POST",
            body={
                "operationName": "InsertCastVote",
                "query": "mutation InsertCastVote { insert_cast_vote }",
            },
        )
        self.assertEqual(
            extract({"log": {"entries": [cast, entry("POST")]}})["resources"], []
        )

    def test_graphql_query_requires_fresh_bindings(self) -> None:
        """Retain ballot-list operations while discarding captured voter variables."""
        query = entry(
            "POST",
            body={
                "operationName": "BallotList",
                "query": "query BallotList { election }",
                "variables": {"voter": "SECRET"},
            },
        )
        result = extract({"log": {"entries": [query]}})
        self.assertEqual(result["resources"][0]["kind"], "graphql_query")
        self.assertTrue(result["resources"][0]["requires_session_binding"])
        self.assertNotIn("SECRET", json.dumps(result))

    def test_unknown_transfer_size_remains_unknown(self) -> None:
        """Do not turn unsupported CDP transfer measurements into zero-byte traffic."""
        request = entry()
        request["response"]["bodySize"] = -42
        result = extract({"log": {"entries": [request]}})
        self.assertIsNone(result["resources"][0]["response_body_bytes"])

    def test_repeated_assets_preserve_load(self) -> None:
        """Do not deduplicate requests that the browser actually made twice."""
        result = extract({"log": {"entries": [entry(), entry()]}})
        self.assertEqual(len(result["resources"]), 2)

    def test_empty_capture_cannot_claim_voter_success(self) -> None:
        """Documentation must distinguish missing evidence from a successful cast."""
        with tempfile.TemporaryDirectory() as directory:
            self.assertIn("No real login-to-cast", render(Path(directory)))

    def test_api_success_without_persistence_is_not_verified(self) -> None:
        """A UI/API success alone must not become a verified E2E report."""
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "capture.json").write_text(json.dumps({"completed": True}))
            self.assertIn("**not verified**", render(Path(directory)))


class PostgresLogTests(unittest.TestCase):
    """Exercise the log collector against two real local PostgreSQL databases."""

    def test_both_databases_and_observer_exclusion(self) -> None:
        """Capture SELECTs from both databases while excluding observer statements."""
        with tempfile.TemporaryDirectory(prefix="voting-e2e-pg-") as directory:
            root = Path(directory)
            data = root / "data"
            with socket.socket() as sock:
                sock.bind(("127.0.0.1", 0))
                port = sock.getsockname()[1]
            subprocess.run(
                ["initdb", "-D", str(data), "-A", "trust", "-U", "postgres"],
                check=True,
                capture_output=True,
            )
            options = f"-p {port} -k {root} -c listen_addresses=127.0.0.1 -c logging_collector=on -c log_destination=jsonlog -c log_statement=all -c log_directory={root}/logs -c log_filename=capture.log -c session_preload_libraries=auto_explain -c auto_explain.log_min_duration=0 -c auto_explain.log_nested_statements=on"
            subprocess.run(
                [
                    "pg_ctl",
                    "-D",
                    str(data),
                    "-l",
                    str(root / "server.log"),
                    "-o",
                    options,
                    "-w",
                    "start",
                ],
                check=True,
                capture_output=True,
            )
            try:
                dsn = f"host=127.0.0.1 port={port} user=postgres"
                with psycopg.connect(dsn, autocommit=True) as connection:
                    connection.execute("CREATE DATABASE backend")
                    connection.execute("CREATE DATABASE keycloak")
                pattern = str(root / "logs" / "*.json")
                positions = log_positions(pattern)
                for database in ("backend", "keycloak"):
                    with psycopg.connect(
                        f"{dsn} dbname={database}",
                        autocommit=True,
                        application_name="voter_fixture",
                    ) as connection:
                        connection.execute(
                            "CREATE FUNCTION nested_probe() RETURNS int LANGUAGE plpgsql AS $$ BEGIN RETURN (SELECT 12767); END $$"
                        )
                        connection.execute("SELECT nested_probe()")
                    with psycopg.connect(
                        f"{dsn} dbname={database}",
                        autocommit=True,
                        application_name="voting_e2e_observer",
                    ) as connection:
                        connection.execute("SELECT 99999")
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    records = {
                        db: collect_logs(pattern, positions, db)
                        for db in ("backend", "keycloak")
                    }
                    if all(
                        any("12767" in record.get("message", "") for record in rows)
                        for rows in records.values()
                    ):
                        break
                    time.sleep(0.05)
                for rows in records.values():
                    self.assertTrue(
                        any(
                            "12767" in row["message"]
                            and "Query Text:" in row["message"]
                            for row in rows
                        )
                    )
                    self.assertFalse(any("99999" in row["message"] for row in rows))
            finally:
                subprocess.run(
                    ["pg_ctl", "-D", str(data), "-m", "immediate", "-w", "stop"],
                    check=True,
                    capture_output=True,
                )


if __name__ == "__main__":
    unittest.main()
