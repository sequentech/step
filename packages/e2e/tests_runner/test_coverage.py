# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import unittest
from runner.coverage import union, totals, lcov
from runner.process import ROOT


class CoverageTests(unittest.TestCase):
    def test_union_deduplicates_lines_and_preserves_uncovered_denominator(self):
        unit = {"file.rs": {"1": 8, "2": 0, "3": 0}}
        e2e = {"file.rs": {"1": 2, "2": 1}, "file.ts": {"4": 0}}
        self.assertEqual(totals(union(unit, e2e)), (2, 4))
        self.assertEqual(unit["file.rs"]["2"], 0)

    def test_lcov_excludes_external_code_and_merges_duplicate_instantiations(self):
        data = f"SF:/dependency/lib.rs\nDA:1,99\nend_of_record\nSF:{ROOT}/packages/core/src/lib.rs\nDA:1,0\nDA:2,5\nend_of_record\nSF:{ROOT}/packages/core/src/lib.rs\nDA:1,3\n"
        self.assertEqual(lcov(data), {"packages/core/src/lib.rs": {"1": 3, "2": 5}})
