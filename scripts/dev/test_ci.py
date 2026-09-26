# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""CI must preserve affected checks and reject absent or cancelled results."""

import unittest
from pathlib import Path

from scripts.dev.affected.changes import from_files
from scripts.dev.affected.model import load_model
from scripts.dev.ci import Validation, check_results, matrices, selection_for

ROOT = Path(__file__).resolve().parents[2]


class PlanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.model = load_model(ROOT)

    def plan(self, *paths):
        return matrices(self.model.select(from_files(ROOT, paths, ROOT)))

    def test_unchanged_tree_skips_every_managed_job(self):
        plan = self.plan()
        self.assertFalse(any(plan["jobs"].values()))
        self.assertFalse(any(plan["ui_jobs"].values()))

    def test_docs_change_does_not_compile_application_code(self):
        plan = self.plan("docs/docusaurus/docs/07-developers/example.md")
        self.assertTrue(plan["jobs"]["docs"])
        self.assertFalse(plan["jobs"]["run-tests"])
        self.assertFalse(plan["jobs"]["run-frontend-tests"])
        self.assertFalse(plan["jobs"]["frontend-ui"])

    def test_results_leaf_keeps_production_validation_without_rust(self):
        plan = self.plan("packages/results-portal/src/App.tsx")
        self.assertEqual(plan["builds"], ["results-portal"])
        self.assertEqual(plan["stories"], ["results-portal"])
        self.assertEqual(
            plan["journeys"], [{"package": "results-portal", "shard": 1, "shards": 1}]
        )
        self.assertFalse(plan["jobs"]["run-tests"])

    def test_shared_ui_selects_every_portal_and_workbench(self):
        plan = self.plan("packages/ui-core/src/index.ts")
        self.assertEqual(len(plan["builds"]), 4)
        self.assertEqual(len(plan["stories"]), 6)
        self.assertEqual(len(plan["journeys"]), 7)
        self.assertTrue(plan["ui_jobs"]["workbench"])
        self.assertFalse(plan["jobs"]["run-tests"])

    def test_compiler_cache_changes_select_all_its_rust_callers(self):
        for path in (
            ".github/actions/setup-rust-cache/action.yml",
            "scripts/dev/rust_cache.py",
        ):
            with self.subTest(path=path):
                plan = self.plan(path)
                self.assertEqual(
                    {row["service"] for row in plan["rust"]},
                    {
                        "electoral-log",
                        "harvest",
                        "strand",
                        "immu-board",
                        "immudb-rs",
                        "sequent-core",
                        "step-cli",
                        "velvet",
                        "wrap-map-err",
                    },
                )
                self.assertTrue(plan["jobs"]["run-windmill-tests"])
                self.assertFalse(plan["jobs"]["frontend-ui"])

    def test_keycloak_page_selects_its_browser_and_type_checks(self):
        plan = self.plan("packages/keycloak-ui/src/login/pages/Login.tsx")
        self.assertEqual(plan["stories"], ["keycloak-ui"])
        self.assertIn(
            {"package": "keycloak-ui", "command": "yarn typecheck"}, plan["node"]
        )
        self.assertFalse(plan["builds"])
        self.assertFalse(plan["jobs"]["run-tests"])

    def test_verifier_builds_voting_once_for_cross_portal_journey(self):
        plan = self.plan("packages/ballot-verifier/src/App.tsx")
        self.assertEqual(plan["builds"], ["ballot-verifier", "voting-portal"])
        self.assertEqual(
            [row["package"] for row in plan["journeys"]], ["ballot-verifier"]
        )

    def test_wasm_source_requires_native_tests_and_package_freshness(self):
        plan = self.plan("packages/sequent-core/src/lib.rs")
        self.assertTrue(plan["jobs"]["wasm-freshness"])
        self.assertIn("sequent-core", [row["service"] for row in plan["rust"]])
        self.assertEqual(plan["builds"], [])

    def test_packaged_wasm_change_selects_consumers(self):
        plan = self.plan("packages/ui-core/rust/sequent-core-0.1.0.tgz")
        self.assertTrue(plan["jobs"]["wasm-freshness"])
        self.assertEqual(len(plan["builds"]), 4)

    def test_fixture_change_selects_consumers(self):
        plan = self.plan("packages/ui-test-kit/src/fixtures/index.ts")
        self.assertTrue(plan["ui_jobs"]["fixtures"])
        self.assertEqual(len(plan["journeys"]), 7)

    def test_graphql_schema_selects_its_portal(self):
        plan = self.plan("packages/voting-portal/graphql.schema.json")
        self.assertIn("voting-portal", plan["builds"])
        self.assertTrue(plan["ui_jobs"]["fixtures"])

    def test_toolchain_or_model_change_selects_every_managed_group(self):
        for path in ("devenv.lock", "scripts/dev/affected.toml", "scripts/dev/ci.py"):
            with self.subTest(path=path):
                self.assertTrue(all(self.plan(path)["jobs"].values()))

    def test_missing_base_selects_full_validation(self):
        _, selection = selection_for(
            ROOT, "refs/heads/absent-ci-test-base", Validation.AFFECTED
        )
        self.assertTrue(selection.fallback)
        self.assertTrue(all(matrices(selection)["jobs"].values()))

    def test_protected_branch_event_selects_full_validation(self):
        _, selection = selection_for(ROOT, "HEAD", Validation.FULL)
        self.assertTrue(all(matrices(selection)["jobs"].values()))

    def test_yarn_lock_selects_ui_not_unrelated_rust(self):
        plan = self.plan("packages/yarn.lock")
        self.assertTrue(plan["jobs"]["run-frontend-tests"])
        self.assertFalse(plan["jobs"]["run-tests"])


class RequiredChecksTests(unittest.TestCase):
    def test_selected_success_and_explicit_skip_pass(self):
        self.assertEqual(
            check_results(
                {"test": True, "docs": False},
                {
                    "test": {"result": "success"},
                    "docs": {"result": "skipped"},
                },
            ),
            [],
        )

    def test_selected_failure_cancellation_skip_or_missing_fails(self):
        for state in ("failure", "cancelled", "skipped", "missing"):
            with self.subTest(state=state):
                results = {} if state == "missing" else {"test": {"result": state}}
                self.assertEqual(len(check_results({"test": True}, results)), 1)

    def test_unselected_execution_is_not_reported_as_a_skip(self):
        self.assertEqual(
            len(check_results({"test": False}, {"test": {"result": "success"}})), 1
        )


if __name__ == "__main__":
    unittest.main()
