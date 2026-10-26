# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""pr_build.yml's preflight: publishing only from main or ovcs, one Docker tag,
inputs through env. Runs the workflow's own script in bash."""

from pathlib import Path
import subprocess
import unittest

import yaml

WORKFLOW = Path(__file__).resolve().parents[2] / ".github" / "workflows" / "pr_build.yml"


def load():
    return yaml.safe_load(WORKFLOW.read_text())


def preflight(push, tag, ref):
    step = load()["jobs"]["preflight"]["steps"][0]
    env = {"PATH": "/usr/bin:/bin", "PUSH": push, "TAG": tag, "REF": ref}
    return subprocess.run(["bash", "-c", step["run"]], env=env, capture_output=True, text=True)


class PrBuildPreflight(unittest.TestCase):
    def test_the_credentialed_job_waits_for_the_preflight(self):
        jobs = load()["jobs"]
        self.assertEqual(jobs["build-and-push"]["needs"], "preflight")
        self.assertEqual(jobs["preflight"].get("permissions"), {})
        self.assertNotIn("secrets", jobs["preflight"])

    def test_inputs_are_forwarded_and_reach_the_script_only_through_env(self):
        jobs = load()["jobs"]
        self.assertEqual(jobs["build-and-push"]["with"], {"tag": "${{ inputs.tag }}", "push": "${{ inputs.push }}"})
        step = jobs["preflight"]["steps"][0]
        self.assertNotIn("${{", step["run"])
        self.assertEqual(set(step["env"]), {"PUSH", "TAG", "REF"})

    def test_publishing_is_refused_outside_main_and_ovcs(self):
        for ref in ("refs/heads/main", "refs/heads/ovcs"):
            self.assertEqual(preflight("true", "", ref).returncode, 0, ref)
        for ref in ("refs/heads/feature", "refs/heads/main-copy", "refs/tags/v1", "refs/pull/1/merge"):
            result = preflight("true", "", ref)
            self.assertNotEqual(result.returncode, 0, ref)
            self.assertIn("allowed only from main or ovcs", result.stdout)

    def test_a_build_without_publishing_runs_from_any_branch(self):
        self.assertEqual(preflight("false", "", "refs/heads/feature").returncode, 0)

    def test_the_tag_is_one_docker_tag(self):
        for tag in ("", "1.2.3", "v10.1-rc_2", "_x", "a" * 128):
            self.assertEqual(preflight("false", tag, "refs/heads/main").returncode, 0, tag)
        for tag in ("-x", ".x", "a" * 129, "a b", "a:b", "x\nregistry.example/evil:tag", "a/b", "tag\n", "ü"):
            self.assertNotEqual(preflight("false", tag, "refs/heads/main").returncode, 0, repr(tag))


if __name__ == "__main__":
    unittest.main()
