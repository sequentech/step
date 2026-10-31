# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import json
import tempfile
import unittest
from pathlib import Path

from .common import roles
from .results import (
    SCHEMA,
    CacheState,
    Result,
    Sample,
    SampleRole,
    read_result,
    result_path,
    summarize_samples,
    validate_label,
    write_result,
)
from .stats import describe
from .summarize import summarize


def sample(index, seconds, role=SampleRole.MEASURED, ok=True, phases=None, load=1.0):
    return Sample(
        index=index,
        role=role,
        ok=ok,
        seconds=seconds,
        started_at="2026-09-26T10:00:00.000+00:00",
        finished_at="2026-09-26T10:00:01.000+00:00",
        load_before=[load, 1.0, 1.0],
        load_after=[1.0, 1.0, 1.0],
        phases=phases or {},
    )


def result(samples, label="before", target="voting"):
    return Result(
        scenario="ui-update",
        target=target,
        label=label,
        cache=CacheState.WARM,
        cache_detail="warm",
        checkout={"commit": "679c3ea18167c7c3a37eab3293b789cb24103acb", "dirty": False},
        harness={},
        host={},
        tools={},
        services=[],
        commands=[],
        parameters={"conditions": ["no rebuild command"]},
        started_at="2026-09-26T11:42:47.123+00:00",
        samples=samples,
    )


class DescribeTest(unittest.TestCase):
    def test_empty_sample_has_no_statistics(self):
        self.assertEqual(
            describe([]),
            {"n": 0, "median": None, "min": None, "max": None, "mean": None},
        )

    def test_even_sample_median_is_the_middle_mean(self):
        summary = describe([4.0, 1.0, 3.0, 2.0])
        self.assertEqual(summary["median"], 2.5)
        self.assertEqual(
            (summary["min"], summary["max"], summary["mean"]), (1.0, 4.0, 2.5)
        )
        self.assertNotIn("p95", summary)

    def test_p95_appears_only_from_twenty_samples(self):
        self.assertNotIn("p95", describe([float(value) for value in range(19)]))
        summary = describe([float(value) for value in range(1, 21)])
        # Inclusive method: position 0.95 * 19 = 18.05 between 19 and 20.
        self.assertAlmostEqual(summary["p95"], 19.05)


class SummaryTest(unittest.TestCase):
    def test_only_successful_measured_samples_are_summarized(self):
        samples = [
            sample(1, 50.0, role=SampleRole.WARMUP),
            sample(2, 2.0, phases={"rebuild": 1.0}),
            sample(3, 4.0, phases={"rebuild": 3.0}),
            sample(4, None, ok=False),
            sample(0, 9.0, role=SampleRole.REVERT),
        ]
        summary = summarize_samples([s.to_dict() for s in samples])
        self.assertEqual(
            (summary["n"], summary["median"], summary["failed"]), (2, 3.0, 1)
        )
        self.assertEqual(summary["phases"]["rebuild"]["median"], 2.0)

    def test_failed_sample_keeps_no_seconds(self):
        self.assertIsNone(sample(1, None, ok=False).to_dict()["seconds"])


class LabelTest(unittest.TestCase):
    def test_accepts_slugs(self):
        self.assertEqual(validate_label("after-hmr.2"), "after-hmr.2")

    def test_rejects_spaces_and_upper_case(self):
        for label in ("After", "before run", "-x", ""):
            with self.subTest(label=label), self.assertRaises(ValueError):
                validate_label(label)


class RolesTest(unittest.TestCase):
    def test_warmups_come_first(self):
        self.assertEqual(
            roles(2, 1), [SampleRole.WARMUP, SampleRole.MEASURED, SampleRole.MEASURED]
        )

    def test_rejects_no_measured_sample(self):
        with self.assertRaises(ValueError):
            roles(0, 1)


class FileTest(unittest.TestCase):
    def test_path_names_the_series_and_start(self):
        path = result_path(Path("/results"), result([]))
        self.assertEqual(
            path, Path("/results/ui-update/voting--warm--before--20260926T114247Z.json")
        )

    def test_written_result_reads_back_with_schema_and_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "ui-update" / "r.json"
            write_result(path, result([sample(1, 1.5)]))
            document = read_result(path)
            self.assertEqual(document["schema"], SCHEMA)
            self.assertEqual(document["summary"]["median"], 1.5)
            self.assertFalse(path.with_suffix(".json.tmp").exists())

    def test_other_json_is_not_a_result(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "timings.json"
            path.write_text(json.dumps({"units": []}))
            with self.assertRaises(ValueError):
                read_result(path)


class SummarizeTest(unittest.TestCase):
    def test_table_row_has_median_range_count_and_conditions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_result(
                root / "ui-update" / "a.json", result([sample(1, 2.0), sample(2, 4.0)])
            )
            (root / "ui-update" / "timings.json").write_text("{}")
            lines = summarize([root], with_phases=False).splitlines()
        self.assertEqual(len(lines), 3)
        cells = [cell.strip() for cell in lines[2].strip("|").split("|")]
        self.assertEqual(
            cells[:8],
            ["ui-update", "voting", "before", "warm", "2", "3.00", "2.00–4.00", "0"],
        )
        self.assertEqual(cells[9], "679c3ea181; no rebuild command")

    def test_phase_rows_follow_their_series(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_result(
                root / "r.json", result([sample(1, 12.0, phases={"rebuild": 10.0})])
            )
            lines = summarize([root], with_phases=True).splitlines()
        self.assertIn("↳ rebuild", lines[3])
        self.assertIn("10.0", lines[3])


if __name__ == "__main__":
    unittest.main()
