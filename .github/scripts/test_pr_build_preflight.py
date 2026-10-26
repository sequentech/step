# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""pr_build.yml's preflight: publishing only from main or ovcs with a
candidate- tag, one Docker tag, inputs through env. Runs the workflow's own
script in bash."""

import subprocess
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "pr_build.yml"


def load():
    return yaml.safe_load(WORKFLOW.read_text())


def preflight(push, tag, ref):
    step = load()["jobs"]["preflight"]["steps"][0]
    env = {"PATH": "/usr/bin:/bin", "PUSH": push, "TAG": tag, "REF": ref}
    return subprocess.run(
        ["bash", "-c", step["run"]], env=env, capture_output=True, text=True
    )


class PrBuildPreflight(unittest.TestCase):
    def test_the_credentialed_job_waits_and_reads_only(self):
        jobs = load()["jobs"]
        self.assertEqual(jobs["build-and-push"]["needs"], "preflight")
        self.assertEqual(jobs["build-and-push"]["permissions"], {"contents": "read"})
        self.assertEqual(jobs["preflight"].get("permissions"), {})
        self.assertNotIn("secrets", jobs["preflight"])

    def test_inputs_are_forwarded_and_reach_the_script_only_through_env(self):
        jobs = load()["jobs"]
        self.assertEqual(
            jobs["build-and-push"]["with"],
            {"tag": "${{ inputs.tag }}", "push": "${{ inputs.push }}"},
        )
        step = jobs["preflight"]["steps"][0]
        self.assertNotIn("${{", step["run"])
        self.assertEqual(set(step["env"]), {"PUSH", "TAG", "REF"})

    def test_publishing_needs_main_or_ovcs(self):
        for ref in ("refs/heads/main", "refs/heads/ovcs"):
            self.assertEqual(preflight("true", "candidate-1", ref).returncode, 0, ref)
        refused = (
            "refs/heads/feature",
            "refs/heads/main-copy",
            "refs/tags/v1",
            "refs/pull/1/merge",
        )
        for ref in refused:
            result = preflight("true", "candidate-1", ref)
            self.assertNotEqual(result.returncode, 0, ref)
            self.assertIn("allowed only from main or ovcs", result.stdout)

    def test_a_published_tag_is_a_candidate_tag(self):
        for tag in ("", "latest", "10.1.0", "v10.1", "candidate-", "x-candidate-1"):
            result = preflight("true", tag, "refs/heads/ovcs")
            self.assertNotEqual(result.returncode, 0, repr(tag))
        self.assertEqual(
            preflight("true", "candidate-meta14594.1", "refs/heads/ovcs").returncode,
            0,
        )

    def test_a_build_without_publishing_passes_the_preflight_on_any_ref(self):
        # The release environment may still refuse the build itself.
        result = preflight("false", "", "refs/heads/feature")
        self.assertEqual(result.returncode, 0)

    def test_the_tag_is_one_docker_tag(self):
        for tag in ("", "1.2.3", "v10.1-rc_2", "_x", "a" * 128):
            result = preflight("false", tag, "refs/heads/main")
            self.assertEqual(result.returncode, 0, tag)
        refused = (
            "-x",
            ".x",
            "a" * 129,
            "a b",
            "a:b",
            "x\nregistry.example/evil:tag",
            "a/b",
            "tag\n",
            "ü",
        )
        for tag in refused:
            result = preflight("false", tag, "refs/heads/main")
            self.assertNotEqual(result.returncode, 0, repr(tag))


if __name__ == "__main__":
    unittest.main()
