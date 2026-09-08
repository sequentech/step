# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Verify finite ownership, import hash semantics and global result accounting."""
import base64
import csv
import hashlib
import json
import os
from pathlib import Path
from contextlib import closing
import sqlite3
import tempfile
import unittest
from unittest.mock import patch

from runner import census, protocol, shard_bounds, shard_count
from aggregate import percentile


class ScaleTests(unittest.TestCase):
    """Boundary tests focus on failures that otherwise corrupt a large run."""

    def test_million_voters_have_disjoint_finite_ranges(self):
        config = dict(start=123, count=1_000_003, shard_size=10000)
        previous = config["start"]
        total = 0
        for shard in range(shard_count(config)):
            first, count = shard_bounds(config, shard)
            self.assertEqual(first, previous)
            self.assertLessEqual(count, 10000)
            previous = first + count
            total += count
        self.assertEqual(total, config["count"])
        for invalid in (-1, shard_count(config)):
            with self.assertRaises(ValueError):
                shard_bounds(config, invalid)

    def test_shared_password_is_hashed_once_and_plaintext_column_is_absent(self):
        config = dict(
            start=23,
            count=10001,
            shard_size=10000,
            username_prefix="load-",
            tenant_id="t",
            election_event_id="event",
            election_id="election",
            area_name="area",
        )
        with tempfile.TemporaryDirectory() as tmp, patch.dict(
            os.environ, LOAD_PASSWORD="synthetic"
        ), patch("runner.hashlib.pbkdf2_hmac", wraps=hashlib.pbkdf2_hmac) as derive:
            directory = Path(tmp) / "census"
            census(config, directory)
            self.assertEqual(derive.call_count, 1)
            hashes = set()
            total = 0
            for path in sorted(directory.glob("*.csv")):
                with path.open() as stream:
                    reader = csv.DictReader(stream)
                    self.assertNotIn("password", reader.fieldnames)
                    for row in reader:
                        self.assertEqual(row["username"], f"load-{23 + total}")
                        hashes.add((row["hashed_password"], row["password_salt"]))
                        total += 1
            self.assertEqual(total, 10001)
            self.assertEqual(len(hashes), 1)
            value, salt = hashes.pop()
            self.assertEqual(
                base64.b64decode(value),
                hashlib.pbkdf2_hmac(
                    "sha256", b"synthetic", base64.b64decode(salt), 27500, 32
                ),
            )

    def test_protocol_requires_no_browser_artifact_and_bootstrap_never_casts(self):
        config = dict(
            keycloak_url="https://auth.example",
            realm="load",
            login_url="https://vote.example/login",
            graphql_url="https://api.example/v1/graphql",
            election_event_id="event",
        )
        self.assertNotIn(
            "cast", [s["kind"] for s in protocol(config, cast=False)["steps"]]
        )
        kinds = [s["kind"] for s in protocol(config, cast=True)["steps"]]
        self.assertEqual(
            kinds,
            [
                "auth",
                "login",
                "token",
                "status",
                "publication",
                "publication",
                "publication",
                "publication",
                "cast",
            ],
        )
        config["mode"] = "status"
        self.assertEqual(
            [s["kind"] for s in protocol(config, cast=True)["steps"]],
            ["auth", "login", "token", "status"],
        )

    def test_global_percentile_uses_all_samples_and_duplicate_ownership_fails(self):
        with closing(sqlite3.connect(":memory:")) as db:
            db.execute("CREATE TABLE samples (voter INTEGER PRIMARY KEY, cast_ms REAL)")
            db.executemany(
                "INSERT INTO samples VALUES (?, ?)", enumerate([1, 2, 3, 1000])
            )
            self.assertEqual(percentile(db, "cast_ms", 0.5), 2.5)
            self.assertAlmostEqual(percentile(db, "cast_ms", 0.99), 970.09)
            with self.assertRaises(sqlite3.IntegrityError):
                db.execute("INSERT INTO samples VALUES (0, 20)")


class SetupTests(unittest.TestCase):
    """Protect realm settings that make shared-password census imports runner."""

    def test_fixture_uses_unique_lookup_and_matching_hash_policy(self):
        from provision import fixture
        from runner import ROOT

        template = json.loads(
            (ROOT / "packages/voting-load/fixtures/election.json").read_text()
        )
        result = fixture(
            template, dict(portal_url="https://vote.example", hash_iterations=27500)
        )
        realm = result["keycloak_event_realm"]
        self.assertEqual(
            realm["passwordPolicy"],
            "hashAlgorithm(pbkdf2-sha256) and hashIterations(27500)",
        )
        for entry in realm["authenticatorConfig"]:
            if "matchAttributes" in entry.get("config", {}):
                self.assertEqual(entry["config"]["matchAttributes"], "username")
        self.assertEqual(len(result["elections"]), 1)
        self.assertGreater(len(result["candidates"]), 0)


class WorkerTests(unittest.TestCase):
    """A restarted node must never acquire a previously attempted voter range."""

    def test_claim_is_exclusive_and_configuration_is_immutable(self):
        from types import SimpleNamespace
        from runner import digest, save, worker

        with tempfile.TemporaryDirectory() as tmp, patch.dict(
            os.environ, LOAD_PASSWORD="synthetic"
        ):
            directory = Path(tmp)
            config = dict(start=0, count=1, shard_size=1, engine="k6", mode="status")
            save(directory / "config.json", config)
            save(
                directory / "ready.json",
                dict(config_sha256=digest(directory / "config.json")),
            )
            with patch(
                "runner.subprocess.run", return_value=SimpleNamespace(returncode=0)
            ) as execute:
                worker(directory, 0)
                with self.assertRaises(FileExistsError):
                    worker(directory, 0)
                self.assertEqual(execute.call_count, 1)
            config["start"] = 1
            save(directory / "config.json", config)
            with self.assertRaisesRegex(ValueError, "configuration changed"):
                worker(directory, 0)


class ReportTests(unittest.TestCase):
    """Reports must reveal missing work and keep protocol internals out of HTML."""

    def test_partial_run_still_writes_failure_report_without_queries(self):
        from runner import save
        from aggregate import report

        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            save(
                directory / "config.json",
                dict(
                    start=0,
                    count=2,
                    shard_size=2,
                    engine="k6",
                    mode="vote",
                    vus=1,
                    goals={"cast_ms": {"p99": 500}},
                    profile={"private_query": "SENSITIVE_QUERY"},
                ),
            )
            with self.assertRaisesRegex(RuntimeError, "Load goals failed"):
                report(directory)
            text = (directory / "report.html").read_text()
            self.assertIn("Failed", text)
            self.assertIn("Missing or failed voter journeys", text)
            self.assertNotIn("SENSITIVE_QUERY", text)
            self.assertNotIn("GraphQL operations", text)
            self.assertEqual(
                json.loads((directory / "results.json").read_text())["accepted_casts"],
                0,
            )

    def test_duplicate_receipts_fail_instead_of_inflating_throughput(self):
        from runner import save
        from aggregate import report

        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            save(
                directory / "config.json",
                dict(start=0, count=2, shard_size=2, engine="k6", mode="vote", vus=1),
            )
            shard = directory / "results/000000"
            shard.mkdir(parents=True)
            save(shard / "exit.json", {"code": 0})
            rows = [
                dict(index=i, passed=True, start=0, end=100, receipt="same-receipt")
                for i in range(2)
            ]
            (shard / "samples.jsonl").write_text(
                "\n".join(json.dumps(row) for row in rows)
            )
            with self.assertRaisesRegex(RuntimeError, "Load goals failed"):
                report(directory)
            self.assertIn(
                "Duplicate voter or API receipt",
                (directory / "report.html").read_text(),
            )


if __name__ == "__main__":
    unittest.main()
