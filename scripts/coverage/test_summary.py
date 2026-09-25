# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""One run summary repeats every paired verdict; it never decides the gate.

Fixtures use download-artifact's layout: one directory per artifact, holding the
verdict.json that ci.py wrote. One test runs ci.py itself so formats agree.
"""

import contextlib
import io
import json
import os
import runpy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import ci
import summary
from ratchet import compare
from report import CoverageError
from test_ratchet import metrics

RUST = ("lines", "functions", "regions")
FRONTEND = ("lines", "statements", "functions", "branches")
EIGHTY = "80/100 (80.0000%)"


def counts(names, covered=80, count=100):
    return {name: {"covered": covered, "count": count} for name in names}


def table_rows(output):
    """Map each rendered package label to its cells, in table order."""
    rows = {}
    for line in output.splitlines():
        if line.startswith("| ") and not line.startswith(("| Package |", "| --- |")):
            cells = [cell.strip() for cell in line.strip("|").split(" | ")]
            rows[cells[0]] = cells[1:]
    return rows


class CoverageSummaryTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name) / "artifacts"

    def write(self, artifact, scope, status="pass", base=None, head=None, **fields):
        """Publish a verdict with the fields that ci.paired_run records."""
        verdict = {
            "scope": scope,
            "status": status,
            "passes": status in ("pass", "initialized"),
            "base_revision": "b" * 40,
            "head_revision": "h" * 40,
            **(compare(base, head) if base else {}),
            **fields,
        }
        path = self.root / artifact / f"{scope}-run" / "verdict.json"
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps(verdict))

    def summarize(self, *expected):
        return summary.markdown(summary.collect(self.root, list(expected)))

    def test_a_pass_repeats_the_exact_fractions_of_both_revisions(self):
        self.write(
            "frontend-coverage-ui-core",
            "ui-core",
            base=counts(FRONTEND),
            head=counts(FRONTEND, 81, 101),
        )
        output = self.summarize()
        self.assertIn(
            "| Package | Lines base | Lines head | Statements base | Statements head"
            " | Functions base | Functions head | Branches base | Branches head"
            " | Verdict |",
            output,
        )
        self.assertEqual(
            table_rows(output)["ui-core"],
            [EIGHTY, "81/101 (80.1980%)"] * 4 + ["PASS"],
        )
        self.assertIn(f"- Base `{'b' * 40}` → head `{'h' * 40}`", output)
        # A pass has nothing further to explain below the table.
        self.assertTrue(output.endswith("| PASS |\n"))

    def test_a_decrease_is_marked_and_the_recorded_verdict_is_kept(self):
        head = counts(RUST)
        head["regions"] = {"covered": 79, "count": 100}
        self.write("coverage-windmill", "windmill", "regression", counts(RUST), head)
        # A new unmeasured file fails CI even though every fraction is kept, and
        # a checkout change is an error after a passing comparison. The summary
        # repeats those verdicts instead of deriving new ones from the numbers.
        self.write(
            "coverage-step-cli",
            "step-cli",
            "regression",
            counts(RUST),
            counts(RUST),
            passes=False,
            failures=["New unmeasured source files: src/load/tests.rs"],
        )
        self.write(
            "coverage-braid",
            "braid",
            "error",
            counts(RUST),
            counts(RUST),
            passes=False,
            failures=["Checkout changed while coverage was running"],
        )
        output = self.summarize()
        rows = table_rows(output)
        self.assertEqual(
            rows["windmill"],
            [EIGHTY] * 5 + ["79/100 (79.0000%) decreased", "REGRESSION"],
        )
        self.assertEqual(rows["step-cli"], [EIGHTY] * 6 + ["REGRESSION"])
        self.assertEqual(rows["braid"], [EIGHTY] * 6 + ["ERROR"])
        for line in (
            "- windmill (REGRESSION): regions coverage decreased: 80/100 → 79/100",
            "- step-cli (REGRESSION): New unmeasured source files: src/load/tests.rs",
            "- braid (ERROR): Checkout changed while coverage was running",
        ):
            self.assertIn(line + "\n", output)

    def test_errors_and_initialized_scopes_are_rows_with_their_reasons(self):
        self.write(
            "coverage-tooling", "coverage-tooling", base=metrics(), head=metrics()
        )
        self.write(
            "coverage-step-cli",
            "step-cli",
            "error",
            failures=["Command failed with exit 2;\nsee base/a|b.log"],
        )
        self.write(
            "coverage-velvet",
            "velvet",
            "initialized",
            note="New source scope: no base implementation exists.",
        )
        output = self.summarize()
        rows = table_rows(output)
        self.assertEqual(rows["step-cli"], ["—"] * 4 + ["ERROR"])
        self.assertEqual(rows["velvet"], ["—"] * 4 + ["INITIALIZED"])
        # Recorded text stays within one list item and cannot split a table.
        self.assertIn(
            "- step-cli (ERROR): Command failed with exit 2; see base/a\\|b.log\n",
            output,
        )
        self.assertIn(
            "- velvet (INITIALIZED): New source scope: no base implementation exists.",
            output,
        )

    def test_an_expected_job_without_a_verdict_is_reported_missing(self):
        self.write(
            "coverage-tooling", "coverage-tooling", base=metrics(), head=metrics()
        )
        # A job that stopped before ci.py ran uploads partial files at most.
        (self.root / "coverage-harvest" / "_temp").mkdir(parents=True)
        output = self.summarize("coverage-tooling", "strand", "harvest", "harvest")
        rows = table_rows(output)
        self.assertEqual(list(rows), ["coverage-tooling", "harvest", "strand"])
        self.assertEqual(rows["harvest"], ["—"] * 4 + ["MISSING"])
        self.assertIn(
            "- strand (MISSING): No readable verdict.json was downloaded for this job.",
            output,
        )

        # Without any downloaded artifact, each expected job is still listed.
        nothing = summary.markdown(summary.collect(self.root / "absent", ["harvest"]))
        self.assertEqual(table_rows(nothing), {"harvest": ["MISSING"]})
        self.assertNotIn("- Base", nothing)
        self.assertIn(
            "No paired coverage verdicts were found.",
            summary.markdown(summary.collect(self.root / "absent", [])),
        )

    def test_an_unreadable_verdict_is_reported_instead_of_skipped(self):
        valid = {
            "scope": "windmill",
            "status": "pass",
            "base_revision": "b",
            "head_revision": "h",
        }
        change = {"base": {"covered": 1, "count": 2}, "decreased": False}
        cases = {
            "truncated": "{",
            "not-an-object": "[]",
            "no-status": json.dumps({"scope": "windmill"}),
            "bad-failures": json.dumps({**valid, "failures": "Tests failed"}),
            "bad-note": json.dumps({**valid, "note": ["Tests failed"]}),
            "bad-change": json.dumps({**valid, "metrics": {"lines": {}}}),
            "bad-counters": json.dumps(
                {
                    **valid,
                    "metrics": {
                        "lines": {**change, "head": {"covered": 3, "count": 2}}
                    },
                }
            ),
        }
        for name, content in cases.items():
            path = self.root / name / "verdict.json"
            path.parent.mkdir(parents=True)
            path.write_text(content)
        output = self.summarize("windmill")
        self.assertEqual(
            {label: cells[-1] for label, cells in table_rows(output).items()},
            {**dict.fromkeys(cases, "UNREADABLE"), "windmill": "MISSING"},
        )
        self.assertIn("- bad-counters (UNREADABLE): Invalid head counters: 3/2", output)
        self.assertIn("- bad-change (UNREADABLE): Invalid lines change", output)

    def test_rust_python_and_frontend_metrics_keep_separate_columns(self):
        self.write(
            "coverage-sequent-core",
            "sequent-core",
            base=counts(RUST),
            head=counts(RUST),
        )
        self.write(
            "coverage-tooling", "coverage-tooling", base=metrics(), head=metrics()
        )
        output = self.summarize()
        # LLVM regions and Python branches are different measurements.
        self.assertIn(
            "| Package | Lines base | Lines head | Functions base | Functions head"
            " | Regions base | Regions head | Branches base | Branches head"
            " | Verdict |",
            output,
        )
        rows = table_rows(output)
        self.assertEqual(
            rows["coverage-tooling"],
            [EIGHTY] * 2 + ["—"] * 4 + [EIGHTY] * 2 + ["PASS"],
        )
        self.assertEqual(rows["sequent-core"], [EIGHTY] * 6 + ["—"] * 2 + ["PASS"])

        # Frontend runs add statements. A metric without opportunities keeps
        # its exact 0/0, and an unfamiliar metric is shown after known ones.
        self.root = self.root.parent / "frontend"
        frontend = counts(FRONTEND)
        frontend["branches"] = {"covered": 0, "count": 0}
        self.write(
            "frontend-coverage-admin-portal",
            "admin-portal",
            base=frontend,
            head=frontend,
        )
        future = counts(("lines", "conditions"))
        self.write("future-coverage", "future", base=future, head=future)
        output = self.summarize()
        self.assertIn(
            "| Package | Lines base | Lines head | Statements base | Statements head"
            " | Functions base | Functions head | Branches base | Branches head"
            " | Conditions base | Conditions head | Verdict |",
            output,
        )
        rows = table_rows(output)
        self.assertEqual(
            rows["admin-portal"][6:], ["0/0 (n/a)"] * 2 + ["—"] * 2 + ["PASS"]
        )
        self.assertEqual(
            rows["future"], [EIGHTY] * 2 + ["—"] * 6 + [EIGHTY] * 2 + ["PASS"]
        )

    def test_rows_follow_package_names_whatever_the_download_order(self):
        for artifact, scope, head in (
            ("z-artifact", "alpha", "1"),
            ("a-artifact", "zeta", "2"),
            ("m-second", "beta", "3"),
            ("m-first", "beta", "4"),
        ):
            self.write(
                artifact, scope, base=metrics(), head=metrics(), head_revision=head * 40
            )
        output = self.summarize("gamma")
        # A package measured twice in one tree names each run's directory.
        self.assertEqual(
            list(table_rows(output)),
            [
                "alpha",
                "beta (`m-first/beta-run`)",
                "beta (`m-second/beta-run`)",
                "gamma",
                "zeta",
            ],
        )
        self.assertEqual(
            [line for line in output.splitlines() if line.startswith("- Base")],
            [f"- Base `{'b' * 40}` → head `{digit * 40}`" for digit in "1234"],
        )
        rglob = Path.rglob
        with patch.object(
            Path, "rglob", lambda path, pattern: reversed(list(rglob(path, pattern)))
        ):
            self.assertEqual(self.summarize("gamma"), output)

    def test_verdicts_written_by_ci_are_summarized_unchanged(self):
        base, head = self.root.parent / "base", self.root.parent / "head"
        for checkout in (base, head):
            (checkout / "scripts/coverage").mkdir(parents=True)
        for package in ("admin-portal", "voting-portal"):
            (base / "packages" / package / "src").mkdir(parents=True)
        # Head is measured first. Without base source, ui-core is initialized.
        runs = (
            ("python", "coverage-tooling", [metrics(79), metrics(80)]),
            ("frontend", "admin-portal", [counts(FRONTEND)] * 2),
            ("frontend", "ui-core", [counts(FRONTEND)]),
            ("frontend", "voting-portal", CoverageError("Jest failed")),
        )
        with (
            patch.object(ci, "identity", return_value="a" * 40),
            patch.dict(os.environ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            os.environ.pop("GITHUB_STEP_SUMMARY", None)
            for kind, package, measured in runs:
                with patch.object(ci, "measure", side_effect=measured):
                    ci.paired_run(base, head, kind, package, self.root / package)
        output = self.summarize()
        self.assertEqual(
            {label: cells[-1] for label, cells in table_rows(output).items()},
            {
                "admin-portal": "PASS",
                "coverage-tooling": "REGRESSION",
                "ui-core": "INITIALIZED",
                "voting-portal": "ERROR",
            },
        )
        self.assertIn("79/100 (79.0000%) decreased", output)
        self.assertIn("- ui-core (INITIALIZED): New source scope:", output)
        self.assertIn("- voting-portal (ERROR): Jest failed\n", output)

    def test_the_command_prints_the_table_and_never_fails_the_job(self):
        self.write("coverage-windmill", "windmill", "error", failures=["Tests failed"])
        stdout = io.StringIO()
        arguments = ["summary.py", str(self.root), "--expect", "windmill"]
        with (
            patch.object(sys, "argv", [*arguments, "--expect", "harvest", "velvet"]),
            contextlib.redirect_stdout(stdout),
            self.assertRaises(SystemExit) as result,
        ):
            runpy.run_path(summary.__file__, run_name="__main__")
        self.assertEqual(result.exception.code, 0)
        self.assertEqual(
            stdout.getvalue(), self.summarize("windmill", "harvest", "velvet")
        )


if __name__ == "__main__":
    unittest.main()
