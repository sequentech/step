# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import unittest

from .ci import (
    DEFAULT_EXCLUDED_WORKFLOWS,
    Job,
    StepKind,
    classify_step,
    parse_time,
    push_metrics,
)

PUSH = "2026-09-26T10:00:00Z"


def job(
    workflow,
    name,
    completed=None,
    conclusion="success",
    started="2026-09-26T10:01:00Z",
    status="completed",
    steps=(),
):
    return Job(
        workflow=workflow,
        name=name,
        status=status,
        conclusion=conclusion if status == "completed" else None,
        created=parse_time(PUSH),
        started=parse_time(started),
        completed=parse_time(completed),
        steps=list(steps),
        runner="GitHub Actions 1",
    )


def runs(*statuses):
    return [{"created_at": PUSH, "status": status} for status in statuses]


class StepTest(unittest.TestCase):
    def test_classifies_steps_by_name(self):
        cases = {
            "Set up job": StepKind.SETUP,
            "Set up Rust tests": StepKind.SETUP,
            "Check out code": StepKind.SETUP,
            "Restore cargo cache": StepKind.SETUP,
            "Build Sequent Core WASM": StepKind.BUILD,
            "Run Windmill tests": StepKind.TEST,
            "Check frontend linting": StepKind.TEST,
            "Post Set up Rust tests": StepKind.TEARDOWN,
            "Complete job": StepKind.TEARDOWN,
            "Notify maintainers": StepKind.OTHER,
        }
        for name, kind in cases.items():
            with self.subTest(name=name):
                self.assertEqual(classify_step(name), kind)

    def test_job_breakdown_sums_step_durations(self):
        steps = [
            {
                "name": "Set up job",
                "started_at": "2026-09-26T10:01:00Z",
                "completed_at": "2026-09-26T10:01:10Z",
            },
            {
                "name": "Run tests",
                "started_at": "2026-09-26T10:01:10Z",
                "completed_at": "2026-09-26T10:02:00Z",
            },
            {"name": "Run more tests", "started_at": None, "completed_at": None},
        ]
        totals = job("Tests", "unit", "2026-09-26T10:02:00Z", steps=steps).breakdown()
        self.assertEqual(
            (totals["setup"], totals["test"], totals["build"]), (10.0, 50.0, 0.0)
        )


class PushTest(unittest.TestCase):
    def test_toolchain_prebuild_is_a_check_but_not_a_product_result(self):
        prebuild = job(
            "Prebuild development tools",
            "build-only (ubuntu-24.04-arm, arm64)",
            "2026-09-26T10:00:10Z",
            started="2026-09-26T10:00:01Z",
        )
        metrics = push_metrics(
            runs("completed"), [prebuild], DEFAULT_EXCLUDED_WORKFLOWS
        )
        self.assertEqual(metrics["phases"]["first_check"], 10.0)
        self.assertNotIn("first_actionable", metrics["phases"])
        self.assertIsNone(metrics["first_actionable_job"])

        product = job(
            "Tests",
            "Build Windmill",
            "2026-09-26T10:01:00Z",
            started="2026-09-26T10:00:01Z",
        )
        metrics = push_metrics(
            runs("completed"), [prebuild, product], DEFAULT_EXCLUDED_WORKFLOWS
        )
        self.assertEqual(metrics["phases"]["first_check"], 10.0)
        self.assertEqual(metrics["phases"]["first_actionable"], 60.0)
        self.assertEqual(metrics["first_actionable_job"], "Tests / Build Windmill")

    def test_selection_and_result_gates_do_not_claim_a_product_test_result(self):
        jobs = [
            job("Tests", "Select affected feedback checks", "2026-09-26T10:00:02Z"),
            job("Tests", "docs-build", "2026-09-26T10:00:10Z"),
            job("Tests", "Selected frontend checks", "2026-09-26T10:00:12Z"),
            job("Tests", "Required feedback checks", "2026-09-26T10:00:15Z"),
            job(
                "Tests",
                "voting-portal focused tests",
                "2026-09-26T10:00:03Z",
                conclusion="skipped",
            ),
        ]
        metrics = push_metrics(runs("completed"), jobs, DEFAULT_EXCLUDED_WORKFLOWS)
        self.assertEqual(metrics["phases"]["first_check"], 2.0)
        self.assertNotIn("first_actionable", metrics["phases"])
        self.assertIsNone(metrics["first_actionable_job"])

    def test_separates_first_check_from_first_build_or_test_result(self):
        jobs = [
            job("REUSE licensing check", "reuse", "2026-09-26T10:00:10Z"),
            job("Lint & Prettify", "Check Hasura prettify", "2026-09-26T10:00:17Z"),
            job(
                "Package coverage",
                "Coverage tooling — no decrease",
                "2026-09-26T10:00:20Z",
            ),
            job("Tests", "Run Rust tests (wrap-map-err)", "2026-09-26T10:00:52Z"),
            job("Tests", "Run Windmill tests", "2026-09-26T10:15:00Z"),
        ]
        metrics = push_metrics(runs("completed"), jobs, DEFAULT_EXCLUDED_WORKFLOWS)
        self.assertEqual(metrics["phases"]["first_check"], 17.0)
        self.assertEqual(metrics["phases"]["first_actionable"], 52.0)
        self.assertEqual(metrics["phases"]["all_done"], 900.0)
        self.assertEqual(
            metrics["first_actionable_job"], "Tests / Run Rust tests (wrap-map-err)"
        )

    def test_skipped_and_cancelled_jobs_are_not_results(self):
        jobs = [
            job("Tests", "skipped", "2026-09-26T10:00:05Z", conclusion="skipped"),
            job("Tests", "cancelled", "2026-09-26T10:00:06Z", conclusion="cancelled"),
            job("Tests", "failing", "2026-09-26T10:03:00Z", conclusion="failure"),
        ]
        metrics = push_metrics(runs("completed"), jobs, DEFAULT_EXCLUDED_WORKFLOWS)
        self.assertEqual(metrics["phases"]["first_actionable"], 180.0)

    def test_incomplete_push_has_no_completion_time(self):
        jobs = [
            job("Tests", "done", "2026-09-26T10:05:00Z"),
            job("Tests", "queued", status="queued", started=None),
        ]
        metrics = push_metrics(
            runs("completed", "in_progress"), jobs, DEFAULT_EXCLUDED_WORKFLOWS
        )
        self.assertFalse(metrics["complete"])
        self.assertNotIn("all_done", metrics["phases"])
        self.assertEqual(metrics["phases"]["first_actionable"], 300.0)

    def test_no_result_when_only_excluded_workflows_finished(self):
        jobs = [job("CLA Assistant", "CLA", "2026-09-26T10:00:04Z")]
        metrics = push_metrics(runs("completed"), jobs, DEFAULT_EXCLUDED_WORKFLOWS)
        self.assertNotIn("first_check", metrics["phases"])
        self.assertIsNone(metrics["first_actionable_job"])

    def test_queue_statistics_use_job_creation_to_start(self):
        jobs = [
            job("Tests", "a", "2026-09-26T10:05:00Z", started="2026-09-26T10:00:02Z"),
            job("Tests", "b", "2026-09-26T10:30:00Z", started="2026-09-26T10:25:00Z"),
        ]
        phases = push_metrics(runs("completed"), jobs, DEFAULT_EXCLUDED_WORKFLOWS)[
            "phases"
        ]
        self.assertEqual((phases["max_queue"], phases["median_queue"]), (1500.0, 751.0))
        self.assertEqual(phases["first_job_started"], 2.0)


if __name__ == "__main__":
    unittest.main()
