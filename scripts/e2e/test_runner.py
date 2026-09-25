# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Driver outcome contracts; no containers or external services are required."""

import contextlib
import importlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

with patch.dict(
    os.environ,
    {
        "SUPER_ADMIN_TENANT_ID": "11111111-1111-4111-8111-111111111111",
        "HASURA_ENDPOINT": "https://graphql.invalid",
        "KEYCLOAK_URL": "https://identity.invalid",
        "AWS_S3_PRIVATE_URI": "https://storage.invalid",
        "B4_URL": "https://board.invalid",
    },
):
    driver = importlib.import_module("scripts.e2e.journeys.__main__")


class RunnerContracts(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.output = Path(temporary.name)
        self.enterContext(patch.object(driver, "OUTPUT", self.output))
        self.enterContext(
            patch("socket.socket", side_effect=AssertionError("Network forbidden"))
        )
        self.enterContext(contextlib.redirect_stdout(io.StringIO()))
        self.enterContext(contextlib.redirect_stderr(io.StringIO()))

    def run_case(self, case, pattern=None):
        with patch.object(driver.test_journeys, "BackendJourneys", case):
            return driver.run_tests(pattern)

    def test_filter_runs_prerequisites_and_records_only_the_selected_sequence(self):
        observed = []

        class Journeys(unittest.TestCase):
            def test_01_bootstrap(self):
                observed.append("bootstrap")

            def test_02_cast(self):
                observed.append("cast")

            def test_03_tally(self):
                observed.append("tally")

        self.assertEqual(self.run_case(Journeys, "cast"), 0)
        self.assertEqual(observed, ["bootstrap", "cast"])
        records = json.loads((self.output / "journeys.json").read_text())
        self.assertEqual(
            [record["test"] for record in records],
            ["test_01_bootstrap", "test_02_cast"],
        )
        self.assertEqual([record["outcome"] for record in records], ["pass", "pass"])
        self.assertEqual(self.run_case(Journeys, "not-a-journey"), 2)
        self.assertEqual(observed, ["bootstrap", "cast"])

    def test_empty_discovery_cannot_report_a_successful_run(self):
        class Valid(unittest.TestCase):
            def test_journey(self):
                pass

        class Empty(unittest.TestCase):
            pass

        self.assertEqual(self.run_case(Valid), 0)
        (self.output / "journeys.json").unlink()
        self.assertEqual(self.run_case(Empty), 2)
        self.assertFalse((self.output / "journeys.json").exists())

    def test_skipped_and_expected_failures_are_not_green_journeys(self):
        class Valid(unittest.TestCase):
            def test_journey(self):
                pass

        class Skipped(unittest.TestCase):
            @unittest.skip("missing prerequisite")
            def test_journey(self):
                pass

        class ExpectedFailure(unittest.TestCase):
            @unittest.expectedFailure
            def test_journey(self):
                self.fail("known defect")

        self.assertEqual(self.run_case(Valid), 0)
        for case, outcome in [(Skipped, "skip"), (ExpectedFailure, "expected failure")]:
            with self.subTest(outcome=outcome):
                self.assertEqual(self.run_case(case), 1)
                records = json.loads((self.output / "journeys.json").read_text())
                self.assertEqual([record["outcome"] for record in records], [outcome])


if __name__ == "__main__":
    unittest.main()
