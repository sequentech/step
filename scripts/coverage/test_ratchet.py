# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""CI protects the previous measurement, even when the 95% goal is unfinished."""

import copy
import unittest

from ratchet import (
    compare,
    compare_rust,
    markdown,
    python_metrics,
    rust_metrics,
)
from report import CoverageError


def metrics(covered=80, count=100):
    """Keep the test's fractions visible instead of manufacturing percentages."""
    return {
        name: {"covered": covered, "count": count} for name in ("lines", "branches")
    }


class CoverageRatchetTests(unittest.TestCase):
    def test_equality_and_improvement_below_95_percent_pass(self):
        # Teams can merge incremental tests without first completing the whole
        # package. The fixed target must never override the relative decision.
        for head in (metrics(80), metrics(81), metrics(160, 200)):
            with self.subTest(head=head):
                self.assertTrue(compare(metrics(80), head)["passes"])

    def test_a_decrease_above_95_percent_fails(self):
        result = compare(metrics(99), metrics(98))
        self.assertFalse(result["passes"])
        self.assertEqual(len(result["failures"]), 2)

    def test_rounding_and_another_metric_cannot_hide_a_decrease(self):
        head = metrics(95000, 100000)
        head["branches"] = {"covered": 94999, "count": 100000}
        result = compare(metrics(95, 100), head)
        self.assertFalse(result["passes"])
        self.assertFalse(result["metrics"]["lines"]["decreased"])
        self.assertTrue(result["metrics"]["branches"]["decreased"])

    def test_added_uncovered_source_lowers_coverage(self):
        self.assertFalse(compare(metrics(80), metrics(80, 101))["passes"])

    def test_empty_branch_inventory_is_vacuously_covered(self):
        base = metrics()
        base["branches"] = {"covered": 0, "count": 0}
        self.assertTrue(compare(base, base)["passes"])
        self.assertFalse(compare(base, metrics())["passes"])
        self.assertTrue(compare(metrics(), base)["passes"])

    def test_invalid_or_incompatible_counters_never_pass(self):
        for head in (
            {},
            metrics(0, 0),
            metrics(True),
            metrics(-1),
            metrics(101),
            metrics(1.5),
        ):
            with self.subTest(head=head), self.assertRaises(CoverageError):
                compare(metrics(), head)

    def test_python_reports_lines_and_branches_separately(self):
        report = {
            "meta": {"branch_coverage": True},
            "files": {"source.py": {}},
            "totals": {
                "covered_lines": 80,
                "num_statements": 100,
                "covered_branches": 20,
                "num_branches": 40,
            },
        }
        result = python_metrics(report)
        self.assertEqual(result["branches"], {"covered": 20, "count": 40})
        for bad in ({}, {**report, "files": {}}, {**report, "meta": {}}):
            with self.assertRaises(CoverageError):
                python_metrics(bad)

    def test_rust_target_shortfall_is_not_a_regression(self):
        report = {
            "status": "measured",
            "tests_passed": 10,
            "passes": False,
            "profile": "core",
            "features": [],
            "tools": {"rust": "pinned"},
            "config_sha256": "same-policy",
            "unaccounted_files": ["src/wasm.rs"],
            "metrics": {
                name: {"covered": 80, "count": 100}
                for name in ("lines", "functions", "regions")
            },
        }
        self.assertTrue(compare_rust(report, report)["passes"])

        # Dropping a compiled source file cannot manufacture an improvement.
        head = copy.deepcopy(report)
        head["unaccounted_files"].append("src/authorization.rs")
        self.assertFalse(compare_rust(report, head)["passes"])

        for field in (
            "profile",
            "features",
            "tools",
            "config_sha256",
            "unaccounted_files",
        ):
            bad = copy.deepcopy(report)
            del bad[field]
            with self.subTest(field=field), self.assertRaises(CoverageError):
                compare_rust(report, bad)

        for field, value in (("status", "error"), ("tests_passed", 0), ("metrics", {})):
            bad = {**report, field: value}
            with self.assertRaises(CoverageError):
                rust_metrics(bad)

    def test_summary_names_the_comparison_and_exact_commit_pair(self):
        result = {
            **compare(metrics(80), metrics(81)),
            "status": "pass",
            "scope": "package",
            "base_revision": "base-sha",
            "head_revision": "head-sha",
        }
        output = markdown(result)
        self.assertIn("base-sha", output)
        self.assertIn("head-sha", output)
        self.assertIn("80/100", output)
        self.assertIn("no coverage decrease", output)


if __name__ == "__main__":
    unittest.main()
