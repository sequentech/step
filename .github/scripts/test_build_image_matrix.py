# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Publication scopes preserve the complete release and isolate authoring builds."""

import json
from pathlib import Path
import subprocess
import sys
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / ".github/scripts/build_image_matrix.py"


class BuildImageMatrixTests(unittest.TestCase):
    def select(self, scope):
        return subprocess.run(
            [sys.executable, str(SCRIPT), scope], capture_output=True, text=True
        )

    def test_authoring_contains_only_two_apps_and_their_contexts(self):
        result = self.select("authoring")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = json.loads(result.stdout)["include"]
        self.assertEqual(
            [row["service"] for row in rows],
            ["election-architect", "delivery-orchestrator"],
        )
        self.assertIn("election_config_wasm=", rows[0]["build_contexts"])
        self.assertIn("guide_source=", rows[1]["build_contexts"])

    def test_all_preserves_release_services(self):
        result = self.select("all")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = json.loads(result.stdout)["include"]
        self.assertEqual(
            {row["service"] for row in rows},
            {"keycloak", "voting-portal", "admin-portal", "ballot-verifier",
             "results-portal", "harvest", "b4v6", "trustee", "windmill",
             "hasura", "loadtesting", "election-architect", "delivery-orchestrator",
             "monitoring-renderer", "messaging-webhooks"},
        )
        self.assertEqual(len(rows), 15)

    def test_monitoring_and_webhooks_use_their_production_build_contexts(self):
        result = self.select("all")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = {row["service"]: row for row in json.loads(result.stdout)["include"]}
        for service, context, dockerfile in (
            ("monitoring-renderer", "packages/monitoring-renderer",
             "packages/monitoring-renderer/Dockerfile"),
            ("messaging-webhooks", "packages",
             "packages/messaging-webhooks/Dockerfile.prod"),
        ):
            with self.subTest(service=service):
                self.assertIn(service, rows)
                self.assertEqual(rows[service]["context"], context)
                self.assertEqual(rows[service]["file"], dockerfile)
                self.assertTrue((ROOT / context).is_dir())
                self.assertTrue((ROOT / dockerfile).is_file())

    def test_architect_scope_excludes_delivery(self):
        result = self.select("architect")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = json.loads(result.stdout)["include"]
        self.assertEqual([row["service"] for row in rows], ["election-architect"])
        self.assertIn("election_config_wasm=", rows[0]["build_contexts"])

    def test_unknown_scope_fails_before_a_build_matrix_is_emitted(self):
        for scope in ("", "delivery", "all,authoring", "$(touch /tmp/untrusted)"):
            with self.subTest(scope=scope):
                result = self.select(scope)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "")

    def test_manual_publication_forwards_inputs_without_deploying(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/pr_build.yml").read_text())
        self.assertEqual(set(workflow["jobs"]), {"build-and-push"})
        job = workflow["jobs"]["build-and-push"]
        self.assertEqual(job["uses"], "./.github/workflows/reusable_build_push.yml")
        self.assertEqual(job["with"], {
            "push": "${{ inputs.push }}",
            "tag": "${{ inputs.tag }}",
            "scope": "${{ inputs.scope }}",
        })

    def test_planner_has_no_release_credentials_and_build_keeps_environment(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/reusable_build_push.yml").read_text())
        planner = workflow["jobs"]["plan"]
        self.assertNotIn("secrets.", json.dumps(planner))
        build = workflow["jobs"]["build-and-push"]
        self.assertEqual(build["environment"], "release")
        self.assertEqual(build["needs"], "plan")
        self.assertEqual(build["strategy"]["matrix"], "${{ fromJSON(needs.plan.outputs.matrix) }}")


if __name__ == "__main__":
    unittest.main()
