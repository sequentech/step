# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import tempfile
import hashlib
import json
import time
from types import SimpleNamespace
import unittest
from pathlib import Path
from load import partition, claim, assess, dispatch, worker


class LoadTests(unittest.TestCase):
    def test_partition_is_disjoint_and_exhaustive(self):
        rows = list(range(12))
        shards = partition(rows, 3, 4)
        self.assertEqual(sorted(sum(shards, [])), rows)
        self.assertEqual([len(s) for s in shards], [4, 4, 4])
        with self.assertRaises(ValueError):
            partition(rows[:11], 3, 4)

    def test_claim_survives_restart(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "claim"
            claim(path)
            with self.assertRaises(FileExistsError):
                claim(path)

    def test_global_percentiles_and_rate_use_raw_samples_and_wall_time(self):
        samples = [
            dict(
                kind="cast",
                accepted=True,
                duration_ms=x,
                started_at_ms=1000,
                ended_at_ms=1000 + x,
            )
            for x in (10, 20, 30, 40)
        ]
        result = assess(
            samples, 4, 1000, 2, {"p50_ms": 25, "p99_ms": 40, "min_casts_per_second": 2}
        )
        self.assertTrue(result["goals_passed"])
        self.assertEqual(result["p50_ms"], 25)
        self.assertEqual(result["accepted_casts_per_second"], 2)
        self.assertFalse(
            assess(
                samples[:3],
                4,
                1000,
                2,
                {"p50_ms": 25, "p99_ms": 40, "min_casts_per_second": 2},
            )["goals_passed"]
        )

    def test_slow_tail_and_failed_cast_fail_goals(self):
        rows = [
            dict(
                kind="cast",
                accepted=False,
                duration_ms=5000,
                started_at_ms=1000,
                ended_at_ms=6000,
            )
        ]
        self.assertFalse(
            assess(
                rows,
                1,
                1000,
                1,
                {"p50_ms": 100, "p99_ms": 500, "min_casts_per_second": 1},
            )["goals_passed"]
        )

    def test_dispatch_rejects_duplicate_assignment_and_expired_tokens(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = dict(
                url="http://graphql:8080/v1/graphql",
                token_expires_at=time.time() + 900,
                tenant_id="tenant",
                election_event_id="event",
                credentials={"username": "voter"},
                payload={
                    "operationName": "InsertCastVote",
                    "variables": {"ballotId": "ballot"},
                },
            )
            fixture = root / "ballots.json"
            fixture.write_text(json.dumps([row]))
            (root / "prepared.json").write_text(
                json.dumps(
                    dict(
                        ballots_sha256=hashlib.sha256(fixture.read_bytes()).hexdigest(),
                        source_commit="test",
                    )
                )
            )
            target = root / "target.json"
            target.write_text(
                json.dumps(
                    dict(
                        tenant_id="tenant",
                        election_event_id="event",
                        login_url="http://portal/login",
                        allowed_origins=["http://graphql:8080"],
                    )
                )
            )
            args = SimpleNamespace(
                output=root / "first",
                target=target,
                prepared=fixture,
                nodes=1,
                rate=1,
                duration=1,
                offset=0,
                start_delay=1,
                ledger=root / "claims",
                p50=100,
                p99=200,
                min_cps=1,
                engine="k6",
                vus=1,
            )
            dispatch(args)
            args.output = root / "duplicate"
            with self.assertRaises(FileExistsError):
                dispatch(args)
            row["token_expires_at"] = time.time() - 1
            fixture.write_text(json.dumps([row]))
            (root / "prepared.json").write_text(
                json.dumps(
                    dict(
                        ballots_sha256=hashlib.sha256(fixture.read_bytes()).hexdigest(),
                        source_commit="test",
                    )
                )
            )
            args.output = root / "expired"
            args.ledger = root / "fresh-claims"
            with self.assertRaisesRegex(ValueError, "Refresh"):
                dispatch(args)
            self.assertFalse(args.ledger.exists())

    def test_corrupted_shard_is_rejected_before_execution(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            (directory / "config.json").write_text("{}")
            (directory / "ballots.json").write_text("[]")
            (directory / "assignment.json").write_text(
                json.dumps({"config.json": "wrong", "ballots.json": "wrong"})
            )
            with self.assertRaisesRegex(ValueError, "digest mismatch"):
                worker(directory)
            self.assertTrue((directory / "attempted").exists())
            self.assertFalse((directory / "samples.jsonl").exists())
