# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import unittest
from runner.monitoring import validate_workload


class WorkloadBudgetTests(unittest.TestCase):
    def setUp(self):
        self.target = {"max_workers": 4, "max_voters": 1000,
                       "max_concurrency": 4, "allow_load": True}

    def check(self, **changes):
        args = {"kind": "load", "engine": "k6", "preset": "smoke",
                "workers": 2, "concurrency": 2}
        args.update(changes)
        return validate_workload(self.target, **args)

    def test_total_concurrency_applies_across_workers_and_presets(self):
        self.assertEqual(self.check(), 8)
        self.assertEqual(self.check(preset="medium"), 1000)
        for changes in ({"workers": 3}, {"concurrency": 3}, {"workers": 0}, {"concurrency": 0}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                self.check(**changes)
        self.target.pop("max_concurrency")
        with self.assertRaises(ValueError): self.check()
        self.assertEqual(self.check(workers=1, concurrency=1), 8)

    def test_browser_runner_memory_budget_and_probe_topology(self):
        self.target["max_concurrency"] = 200
        self.assertEqual(self.check(engine="chromium"), 8)
        with self.assertRaises(ValueError): self.check(engine="chromium", concurrency=3)
        self.assertEqual(self.check(engine="k6", workers=4, concurrency=50), 8)
        with self.assertRaises(ValueError): self.check(engine="k6", concurrency=51)
        self.assertEqual(self.check(kind="probe", engine="chromium", workers=1, concurrency=1), 8)
        for changes in ({"engine": "k6"}, {"workers": 2}, {"concurrency": 2}):
            args = {"kind": "probe", "engine": "chromium", "workers": 1, "concurrency": 1, **changes}
            with self.subTest(changes=changes), self.assertRaises(ValueError): self.check(**args)

    def test_concurrency_does_not_bypass_voter_or_load_permission_limits(self):
        self.target["max_voters"] = 8
        with self.assertRaises(ValueError): self.check(preset="small")
        self.target["allow_load"] = False
        with self.assertRaises(ValueError): self.check(workers=1, concurrency=1)
        self.assertEqual(self.check(kind="probe", engine="chromium", workers=1, concurrency=1), 8)
