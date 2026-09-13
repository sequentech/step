# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Protect the coverage gate against reports that look healthier than they are."""

import copy
import re
import tempfile
import unittest
from pathlib import Path

from report import CoverageError, exclusion_arguments, summarize, validate_exclusions


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

    def test_zero_line_count_cannot_hide_measured_functions_or_regions(self) -> None:
        for metric in ("functions", "regions"):
            record = llvm_file(self.source, 0, 0)
            record["summary"][metric] = {"count": 1, "covered": 0}
            with (
                self.subTest(metric=metric),
                self.assertRaisesRegex(CoverageError, "cannot exclude measured code"),
            ):
                summarize(
                    export(record),
                    self.package,
                    95,
                    {"src/codec.rs": "No executable lines"},
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


class CoverageExclusionTests(unittest.TestCase):
    """Reviewed scaffolding can be omitted without masking runtime scope gaps."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.package = Path(self.directory.name) / "core.with-regex+[characters]"
        self.source = self.package / "src/codec.rs"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("pub fn encode() {}\n")
        self.fixture = self.package / "src/fixtures.rs"
        self.fixture.write_text("pub fn fixture() {}\n")
        self.excluded = {"src/fixtures.rs": "Synthetic test data builder."}

    def test_excluded_counters_are_disclosed_but_do_not_pad_the_score(self):
        # Removing a fully covered fixture should LOWER this example's score.
        # This guards against keeping its covered lines in the numerator.
        result = summarize(
            export(llvm_file(self.source, 50), llvm_file(self.fixture, 100)),
            self.package,
            95,
            {},
            self.excluded,
        )
        self.assertEqual(result["metrics"]["lines"]["percent"], 50)
        self.assertEqual(result["excluded_files"], self.excluded)
        self.assertEqual(
            result["excluded_measurements"]["src/fixtures.rs"]["lines"],
            {"covered": 100, "count": 100},
        )
        self.assertNotIn("src/fixtures.rs", result["files"])
        self.assertIn("src/fixtures.rs", result["source_files"])
        self.assertEqual(result["unaccounted_files"], [])

    def test_absent_excluded_measurement_does_not_hide_another_unmeasured_file(self):
        # LLVM may omit a test-only module. Its exclusion must not cover a
        # similarly named runtime file added beside it later.
        runtime = self.package / "src/fixtures_runtime.rs"
        runtime.write_text("pub fn validate() {}\n")
        result = summarize(
            export(llvm_file(self.source)), self.package, 95, {}, self.excluded
        )
        self.assertEqual(result["unaccounted_files"], ["src/fixtures_runtime.rs"])
        self.assertFalse(result["passes"])

    def test_exclusions_need_exact_existing_files_and_nonempty_reasons(self):
        for invalid in (
            [],
            {"src/*.rs": "Wildcard"},
            {"src/deleted.rs": "Stale"},
            {"../outside.rs": "Outside"},
            {"src/fixtures.rs": " "},
            {"src/fixtures.rs": None},
        ):
            with self.subTest(invalid=invalid), self.assertRaises(CoverageError):
                validate_exclusions(self.package, invalid, {})
        with self.assertRaisesRegex(CoverageError, "both excluded"):
            validate_exclusions(self.package, self.excluded, self.excluded)

    def test_excluded_entries_still_require_valid_unique_counters(self):
        for records in (
            [llvm_file(self.fixture, 101)],
            [llvm_file(self.fixture), llvm_file(self.fixture)],
        ):
            with self.subTest(records=records), self.assertRaises(CoverageError):
                summarize(
                    export(llvm_file(self.source), *records),
                    self.package,
                    95,
                    {},
                    self.excluded,
                )

    def test_excluding_every_measured_line_is_not_a_passing_empty_report(self):
        with self.assertRaisesRegex(CoverageError, "No measured"):
            summarize(
                export(llvm_file(self.fixture)), self.package, 95, {}, self.excluded
            )

    def test_filter_uses_literal_full_paths_including_regex_metacharacters(self):
        self.assertEqual(exclusion_arguments(self.package, {}), [])
        flag, pattern = exclusion_arguments(self.package, self.excluded)
        self.assertEqual(flag, "--ignore-filename-regex")
        self.assertIsNotNone(re.search(pattern, str(self.fixture)))
        for other in (
            str(self.fixture) + ".bak",
            str(self.fixture).replace("fixtures", "fixture"),
            str(self.package.parent / "another-package/src/fixtures.rs"),
        ):
            self.assertIsNone(re.search(pattern, other))


if __name__ == "__main__":
    unittest.main()
