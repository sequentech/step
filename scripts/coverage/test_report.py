# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Protect the coverage gate against reports that look healthier than they are."""

import copy
import tempfile
import unittest
from pathlib import Path

from report import CoverageError, summarize


def llvm_file(path: Path, covered: int = 95, count: int = 100) -> dict:
    """Small LLVM export fixture; percentages are deliberately untrustworthy."""
    return {
        "filename": str(path),
        "summary": {
            metric: {"count": count, "covered": covered, "percent": 100.0}
            for metric in ("lines", "functions", "regions")
        },
    }


def export(*files: dict) -> dict:
    return {
        "type": "llvm.coverage.json.export",
        "version": "3.1.0",
        "data": [{"files": list(files)}],
    }


class CoverageReportTests(unittest.TestCase):
    """Use real paths so another package cannot accidentally satisfy this gate."""

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.package = Path(self.directory.name) / "sequent-core"
        self.source = self.package / "src" / "codec.rs"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("pub fn encode() {}\n")

    def summarize(self, payload: dict, **kwargs: object) -> dict:
        return summarize(payload, self.package, minimum=95, exceptions={}, **kwargs)

    def test_exact_boundary_passes_using_counts_not_reported_percent(self) -> None:
        result = self.summarize(export(llvm_file(self.source)))
        self.assertTrue(result["passes"])
        self.assertEqual(result["metrics"]["lines"]["percent"], 95.0)

    def test_rounding_cannot_turn_a_shortfall_into_a_pass(self) -> None:
        # 94.999% is displayed close to 95%, but still falls below the target.
        result = self.summarize(export(llvm_file(self.source, 94999, 100000)))
        self.assertFalse(result["passes"])
        self.assertIn("below 95%", " ".join(result["failures"]))

    def test_files_are_weighted_by_lines_not_averaged_percentages(self) -> None:
        small_file = self.package / "src" / "small.rs"
        small_file.write_text("pub fn small() {}\n")
        result = self.summarize(
            export(llvm_file(self.source, 0, 99), llvm_file(small_file, 1, 1))
        )
        self.assertEqual(result["metrics"]["lines"]["percent"], 1.0)

    def test_a_new_unmeasured_source_file_blocks_the_gate(self) -> None:
        # A file can disappear from LLVM when its module is not compiled.
        missing = self.package / "src" / "authorization.rs"
        missing.write_text("pub fn authorize() {}\n")
        result = self.summarize(export(llvm_file(self.source, 100)))
        self.assertFalse(result["passes"])
        self.assertEqual(result["unaccounted_files"], ["src/authorization.rs"])

    def test_scope_exception_is_visible_and_requires_an_exact_existing_file(
        self,
    ) -> None:
        declaration = self.package / "src" / "lib.rs"
        declaration.write_text("mod codec;\n")
        result = summarize(
            export(llvm_file(self.source)),
            self.package,
            95,
            {"src/lib.rs": "Module declarations only; no executable lines."},
        )
        self.assertTrue(result["passes"])
        self.assertEqual(
            result["scope_exceptions"]["src/lib.rs"],
            "Module declarations only; no executable lines.",
        )

        for invalid in (
            {"src/*.rs": "Wildcard"},
            {"src/absent.rs": "Stale"},
            {"src/lib.rs": ""},
        ):
            with self.subTest(invalid=invalid), self.assertRaises(CoverageError):
                summarize(export(llvm_file(self.source)), self.package, 95, invalid)

    def test_measured_code_cannot_be_removed_with_a_scope_exception(self) -> None:
        with self.assertRaises(CoverageError):
            summarize(
                export(llvm_file(self.source, 0)),
                self.package,
                95,
                {"src/codec.rs": "Do not count this uncovered code"},
            )

    def test_dependencies_and_integration_test_files_do_not_pad_package_totals(
        self,
    ) -> None:
        other = self.package.parent / "strand" / "src" / "lib.rs"
        test_file = self.package / "tests" / "smoke.rs"
        result = self.summarize(
            export(
                llvm_file(self.source, 20),
                llvm_file(other, 1000, 1000),
                llvm_file(test_file, 1000, 1000),
            )
        )
        self.assertEqual(result["metrics"]["lines"]["count"], 100)
        self.assertEqual(result["metrics"]["lines"]["covered"], 20)

    def test_zero_lines_is_an_error_even_if_the_export_claims_full_coverage(
        self,
    ) -> None:
        with self.assertRaises(CoverageError):
            self.summarize(export(llvm_file(self.source, 0, 0)))

    def test_empty_missing_and_wrong_schema_reports_are_errors(self) -> None:
        for payload in (
            {},
            export(),
            {"data": []},
            {"type": "other", "data": [{"files": []}]},
        ):
            with self.subTest(payload=payload), self.assertRaises(CoverageError):
                self.summarize(payload)

    def test_invalid_counts_are_errors_in_every_metric(self) -> None:
        for metric in ("lines", "functions", "regions"):
            for covered, count in (
                (-1, 100),
                (101, 100),
                (True, 100),
                (1.5, 100),
                (1, "100"),
                (0, -1),
            ):
                payload = export(llvm_file(self.source))
                payload["data"][0]["files"][0]["summary"][metric] = {
                    "covered": covered,
                    "count": count,
                }
                with (
                    self.subTest(metric=metric, covered=covered, count=count),
                    self.assertRaises(CoverageError),
                ):
                    self.summarize(payload)

    def test_missing_metrics_duplicate_files_and_stale_paths_are_errors(self) -> None:
        payload = export(llvm_file(self.source))
        incomplete = copy.deepcopy(payload)
        del incomplete["data"][0]["files"][0]["summary"]["regions"]
        for invalid in (
            incomplete,
            export(llvm_file(self.source), llvm_file(self.source)),
            export(llvm_file(self.package / "src" / "deleted.rs")),
        ):
            with self.subTest(invalid=invalid), self.assertRaises(CoverageError):
                self.summarize(invalid)

    def test_report_must_contain_one_combined_export(self) -> None:
        payload = export(llvm_file(self.source))
        payload["data"].append(copy.deepcopy(payload["data"][0]))
        with self.assertRaises(CoverageError):
            self.summarize(payload)

    def test_invalid_thresholds_and_schema_fields_are_rejected(self) -> None:
        for minimum in (-1, 101, True, 95.5):
            with self.subTest(minimum=minimum), self.assertRaises(CoverageError):
                summarize(export(llvm_file(self.source)), self.package, minimum, {})

        for field, value in (
            ("version", "4.0.0"),
            ("data", [{"files": None}]),
            ("data", [{"files": [None]}]),
        ):
            payload = export(llvm_file(self.source))
            payload[field] = value
            with (
                self.subTest(field=field, value=value),
                self.assertRaises(CoverageError),
            ):
                self.summarize(payload)

        for field, value in (("filename", "src/codec.rs"), ("summary", None)):
            record = llvm_file(self.source)
            record[field] = value
            with self.subTest(field=field), self.assertRaises(CoverageError):
                self.summarize(export(record))

    def test_zero_function_count_is_unknown_instead_of_division_by_zero(self) -> None:
        record = llvm_file(self.source)
        record["summary"]["functions"] = {"count": 0, "covered": 0}
        self.assertIsNone(
            self.summarize(export(record))["metrics"]["functions"]["percent"]
        )


if __name__ == "__main__":
    unittest.main()
