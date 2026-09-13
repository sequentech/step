# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Exercise paired decisions, including real coverage against tiny git checkouts.

The integration fixture has its own source and tests on each side. Removing a
single test must produce a failing decision even though the remaining tests pass.
"""

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import ci
from report import CoverageError
from test_ratchet import metrics


class PairedCoverageTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.base, self.head = self.root / "base", self.root / "head"
        self.output = self.root / "reports"
        for checkout in (self.base, self.head):
            (checkout / "scripts/coverage").mkdir(parents=True)

    def run_pair(self, **kwargs):
        with patch.object(ci, "identity", return_value="a" * 40):
            return ci.paired_run(
                self.base,
                self.head,
                "python",
                "coverage-tooling",
                self.output,
                **kwargs,
            )

    def verdict(self):
        reports = list(self.output.glob("*/verdict.json"))
        self.assertEqual(len(reports), 1)
        return json.loads(reports[0].read_text())

    def test_equal_coverage_does_not_require_the_improvement_target(self):
        with patch.object(ci, "measure", return_value=metrics(70)):
            self.assertEqual(self.run_pair(), 0)
        self.assertEqual(self.verdict()["status"], "pass")

    def test_decrease_blocks_the_gate_and_is_visible_in_actions(self):
        summary = self.root / "github-summary.md"
        with (
            patch.object(ci, "measure", side_effect=[metrics(98), metrics(99)]),
            patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(summary)}),
        ):
            self.assertEqual(self.run_pair(), 1)
        self.assertEqual(self.verdict()["status"], "regression")
        self.assertIn("decreased", summary.read_text())

    def test_only_absent_base_source_can_initialize_a_measurement(self):
        (self.base / "scripts/coverage").rmdir()
        with patch.object(ci, "measure", return_value=metrics(40)) as measure:
            self.assertEqual(self.run_pair(), 0)
        measure.assert_called_once()
        self.assertEqual(self.verdict()["status"], "initialized")

    def test_a_broken_baseline_is_not_treated_as_zero_coverage(self):
        with patch.object(
            ci, "measure", side_effect=[metrics(), CoverageError("Tests failed")]
        ):
            self.assertEqual(self.run_pair(), 2)
        self.assertFalse(self.verdict()["passes"])
        self.assertIn("Tests failed", self.verdict()["failures"])

    def test_checkout_changes_invalidate_an_otherwise_passing_comparison(self):
        with (
            patch.object(ci, "measure", return_value=metrics()),
            patch.object(ci, "identity", side_effect=["a" * 40, "b" * 40, "c" * 40]),
        ):
            self.assertEqual(
                ci.paired_run(
                    self.base, self.head, "python", "coverage-tooling", self.output
                ),
                2,
            )
        self.assertEqual(self.verdict()["status"], "error")

    def test_a_previous_success_cannot_be_reused_after_failure(self):
        with patch.object(ci, "measure", return_value=metrics()):
            self.assertEqual(self.run_pair(), 0)
        with patch.object(
            ci, "measure", side_effect=FileNotFoundError("missing report")
        ):
            self.assertEqual(self.run_pair(), 2)
        statuses = sorted(
            json.loads(path.read_text())["status"]
            for path in self.output.glob("*/verdict.json")
        )
        self.assertEqual(statuses, ["error", "pass"])

    def test_command_logs_are_bounded_and_do_not_publish_local_target_verdicts(self):
        with (
            patch.object(ci, "execute", return_value="ok") as execute,
            patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": "unused"}),
        ):
            self.assertEqual(
                ci.command(["tool"], self.head, self.output, "tests"), "ok"
            )
        self.assertNotIn("GITHUB_STEP_SUMMARY", execute.call_args.args[2])
        self.assertEqual(execute.call_args.kwargs["cwd"], self.head)

    def test_zero_python_tests_is_an_error(self):
        with patch.object(ci, "command", return_value="Ran 0 tests in 0.001s"):
            with self.assertRaisesRegex(CoverageError, "No Python tests"):
                ci.measure_python(self.head, self.output)


class RealPythonComparisonTests(unittest.TestCase):
    def fixture(self, checkout, all_paths):
        source = checkout / "scripts/coverage"
        source.mkdir(parents=True)
        (source / "choose.py").write_text(
            "def choose(flag):\n    if flag:\n        return 'yes'\n    return 'no'\n"
        )
        test = (
            "import unittest\nfrom choose import choose\n"
            "class ChoiceTests(unittest.TestCase):\n"
            "    def test_yes(self):\n"
            "        self.assertEqual(choose(True), 'yes')\n"
        )
        if all_paths:
            test += (
                "    def test_no(self):\n"
                "        self.assertEqual(choose(False), 'no')\n"
            )
        (source / "test_choose.py").write_text(test)
        for args in (
            ["init", "-q"],
            ["add", "."],
            [
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        ):
            subprocess.run(["git", "-C", str(checkout), *args], check=True)
        return checkout

    def test_removing_a_test_fails_with_real_branch_and_line_measurements(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base = self.fixture(root / "base", all_paths=True)
            head = self.fixture(root / "head", all_paths=False)
            # Bytecode is generated by Python, never a source input.
            with patch.dict(os.environ, {"PYTHONDONTWRITEBYTECODE": "1"}):
                code = ci.paired_run(base, head, "python", "fixture", root / "reports")
            self.assertEqual(code, 1)
            verdict = json.loads(
                next((root / "reports").glob("*/verdict.json")).read_text()
            )
            self.assertTrue(verdict["metrics"]["branches"]["decreased"])
            self.assertTrue(verdict["metrics"]["lines"]["decreased"])

    def test_dirty_inputs_and_nonobject_reports_fail_clearly(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.fixture(Path(directory) / "checkout", all_paths=True)
            (root / "untracked.py").write_text("unexpected = True")
            with self.assertRaises(CoverageError):
                ci.identity(root)
            (root / "bad.json").write_text("[]")
            with self.assertRaises(CoverageError):
                ci.read_json(root / "bad.json")


if __name__ == "__main__":
    unittest.main()
