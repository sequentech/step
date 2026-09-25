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
summary = importlib.import_module("scripts.e2e.summary")
ROOT = Path(__file__).resolve().parents[2]


class DriverOutput(unittest.TestCase):
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


class RunnerContracts(DriverOutput):
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


class SummaryContracts(DriverOutput):
    def test_summary_tables_every_recorded_journey_and_check_with_totals(self):
        class Journeys(unittest.TestCase):
            def test_1_import(self):
                pass

            def test_2_tally(self):
                self.fail("expected 3 | got 2\nsecond line")

        self.assertEqual(self.run_case(Journeys), 1)
        (self.output / "bootstrap.json").write_text(
            json.dumps({"seconds": {"tenant row": 2.0, "trustees registered": 9.5}})
        )
        (self.output / "coverage").mkdir()
        (self.output / "coverage/summary.md").write_text("### Coverage table\n")
        text = summary.render(self.output)
        self.assertIn("| bootstrap | pass | 9.5 s | |", text)
        self.assertRegex(text, r"\| `test_1_import` \| pass \| \d+\.\d s \|")
        self.assertRegex(
            text, r"\| `test_2_tally` \| \*\*fail\*\* \| .* expected 3 \\\| got 2 \|"
        )
        self.assertNotIn("second line", text)
        self.assertIn("**Failed:** 2 journeys (1 pass, 1 fail)", text)
        self.assertTrue(text.rstrip().endswith("### Coverage table"))

        self.assertEqual(self.run_case(Journeys, "import"), 0)
        self.assertIn("**Passed:** 1 journeys (1 pass)", summary.render(self.output))

    def test_summary_explains_a_run_without_results(self):
        for directory in (self.output, self.output / "never-created"):
            with self.subTest(directory=directory.name):
                text = summary.render(directory)
                self.assertIn("No journey results", text)
                self.assertNotIn("| Journey", text)
        with patch("sys.argv", ["summary.py", str(self.output)]):
            summary.main()
        self.assertIn("No journey results", (self.output / "summary.md").read_text())

    def test_ci_summarizes_and_uploads_results_after_every_outcome(self):
        workflow = (ROOT / ".github/workflows/backend-e2e.yml").read_text()
        step = workflow[workflow.index("name: Summarize the journey results") :]
        step = step[: step.index("- name:")]
        self.assertIn("if: always()", step)
        command = "python3 scripts/e2e/summary.py .cache/backend-e2e/run"
        self.assertIn(f'{command} >> "$GITHUB_STEP_SUMMARY"', step)
        upload = workflow[workflow.index("name: Upload the results summary") :]
        self.assertIn("hashFiles('.cache/backend-e2e/run/summary.md') != ''", upload)
        for name in ("summary.md", "journeys.json", "coverage/summary.json"):
            self.assertIn(f".cache/backend-e2e/run/{name}", upload)


if __name__ == "__main__":
    unittest.main()
