# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Check fixture ownership and cleanup without contacting a Docker daemon."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
FAKE_DOCKER = r"""#!/usr/bin/env python3
import json, os, sys
args = sys.argv[1:]
with open(os.environ["FAKE_DOCKER_LOG"], "a") as output:
    output.write(json.dumps(args) + "\n")
mode = os.environ["FAKE_DOCKER_MODE"]
if args[0] == "create" and mode == "collision":
    sys.exit(17)
if args[0] == "exec" and "psql" in args:
    sys.stdin.read()
    if mode == "fixture-failure":
        sys.exit(23)
if args[0] == "port":
    print("127.0.0.1:15432")
"""


class CastVoteFixture(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="step-castvote-test-")
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.log = self.directory / "docker.jsonl"
        docker = self.directory / "docker"
        docker.write_text(FAKE_DOCKER)
        docker.chmod(0o755)
        self.name = "fixture-owned-by-this-test"

    def run_fixture(self, mode):
        return subprocess.run(
            ["bash", str(ROOT / "scripts/voting_flow/castvote_fixture.sh"), self.name],
            env={
                **os.environ,
                "PATH": f"{self.directory}{os.pathsep}{os.environ['PATH']}",
                "FAKE_DOCKER_LOG": str(self.log),
                "FAKE_DOCKER_MODE": mode,
            },
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )

    def calls(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_success_returns_the_url_and_leaves_cleanup_to_the_caller(self):
        result = self.run_fixture("success")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "postgres://postgres@127.0.0.1:15432/postgres\n")
        calls = self.calls()
        self.assertEqual(calls[0][0], "create")
        self.assertIn(["start", self.name], calls)
        self.assertEqual(len([call for call in calls if "psql" in call]), 5)
        self.assertFalse(any(call[0] == "rm" for call in calls))

    def test_fixture_failure_removes_only_its_container_and_anonymous_volumes(self):
        result = self.run_fixture("fixture-failure")
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(
            [call for call in self.calls() if call[0] == "rm"],
            [["rm", "--force", "--volumes", self.name]],
        )

    def test_existing_name_failure_never_starts_or_removes_that_container(self):
        result = self.run_fixture("collision")
        self.assertEqual(result.returncode, 17, result.stderr)
        self.assertEqual(result.stdout, "")
        calls = self.calls()
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][:3], ["create", "--name", self.name])


if __name__ == "__main__":
    unittest.main()
