# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise metadata-check resource ownership without contacting Docker."""

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FAKE_DOCKER = r"""#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
state_path = Path(os.environ["FAKE_DOCKER_STATE"])
state = json.loads(state_path.read_text())
mode = os.environ["FAKE_DOCKER_MODE"]
with open(os.environ["FAKE_DOCKER_LOG"], "a") as output:
    output.write(json.dumps(args) + "\n")

def finish(status=0, output=None):
    state_path.write_text(json.dumps(state))
    if output is not None:
        print(output)
    sys.exit(status)

def add(identifier, kind, name, owner):
    state[identifier] = {"kind": kind, "name": name, "owner": owner}

if args[:2] == ["network", "create"]:
    name = args[2]
    if mode == "network-collision":
        add("sentinel-network", "network", name, "other")
        add("sentinel-postgres", "container", name + "-postgres", "other")
        add("sentinel-hasura", "container", name + "-hasura", "other")
        finish(17)
    add("owned-network", "network", name, "this-run")
    finish(output="owned-network")
if args[0] in ("create", "run"):
    name = args[args.index("--name") + 1]
    service = "postgres" if name.endswith("-postgres") else "hasura"
    if mode == service + "-collision":
        add("sentinel-" + service, "container", name, "other")
        finish(18 if service == "postgres" else 19)
    identifier = "owned-" + service
    add(identifier, "container", name, "this-run")
    if args[0] == "run" and mode == service + "-start-failure":
        finish(20 if service == "postgres" else 21)
    finish(output=identifier)
if args[0] == "start":
    if mode == args[1].replace("owned-", "") + "-start-failure":
        finish(20 if args[1] == "owned-postgres" else 21)
    finish(output=args[1])
if args[0] == "inspect":
    finish(output="true")
if args[0] == "exec":
    if "pg_isready" in args or any(arg.endswith("/healthz") for arg in args):
        finish()
    if "psql" in args:
        finish(output="1")
    if any(arg.endswith("/v1/metadata") for arg in args):
        if mode == "name-reused":
            original = state.pop("owned-hasura")
            add("sentinel-hasura", "container", original["name"], "other")
        finish(output=json.dumps({
            "is_consistent": mode != "inconsistent-metadata", "inconsistent_objects": []
        }))
if args[0] == "rm" or args[:2] == ["network", "rm"]:
    targets = args[1:] if args[0] == "rm" else args[2:]
    for target in targets:
        for identifier, resource in list(state.items()):
            if target in (identifier, resource["name"]):
                del state[identifier]
    finish()
sys.exit("Unexpected fake Docker request: " + repr(args))
"""


class MetadataCleanup(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="step-metadata-cleanup-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.script = self.root / "scripts/postgres/check_hasura_metadata.sh"
        self.script.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "scripts/postgres/check_hasura_metadata.sh", self.script)
        (self.root / "hasura/migrations/backend-db/one-migration").mkdir(parents=True)
        (self.root / "hasura/metadata").mkdir()
        docker = self.root / "docker"
        docker.write_text(FAKE_DOCKER)
        docker.chmod(0o755)
        self.state = self.root / "resources.json"
        self.log = self.root / "docker.jsonl"

    def run_check(self, mode="success"):
        self.state.write_text("{}")
        self.log.write_text("")
        return subprocess.run(
            ["bash", str(self.script)],
            env={
                **os.environ,
                "PATH": f"{self.root}{os.pathsep}{os.environ['PATH']}",
                "FAKE_DOCKER_STATE": str(self.state),
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

    def assert_owned_resources_removed(self):
        self.assertFalse(
            any(
                row["owner"] == "this-run"
                for row in json.loads(self.state.read_text()).values()
            )
        )

    def assert_valid_control(self):
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            "Hasura applied 1 migrations and reports consistent metadata", result.stdout
        )
        self.assertEqual(json.loads(self.state.read_text()), {})

    def test_consistent_metadata_cleans_up_created_containers_and_volumes(self):
        self.assert_valid_control()
        removals = [call for call in self.calls() if call[0] == "rm"]
        targets = []
        for call in removals:
            self.assertIn("--volumes", call)
            targets.extend(value for value in call[1:] if not value.startswith("-"))
        self.assertCountEqual(targets, ["owned-hasura", "owned-postgres"])
        self.assertIn(["network", "rm", "owned-network"], self.calls())

    def test_collisions_preserve_every_existing_resource(self):
        self.assert_valid_control()
        for service, code, count in (
            ("network", 17, 3),
            ("postgres", 18, 1),
            ("hasura", 19, 1),
        ):
            with self.subTest(service=service):
                result = self.run_check(service + "-collision")
                self.assertEqual(result.returncode, code, result.stderr)
                self.assertNotIn("reports consistent metadata", result.stdout)
                remaining = json.loads(self.state.read_text())
                self.assertEqual(len(remaining), count)
                self.assertTrue(
                    all(row["owner"] == "other" for row in remaining.values())
                )
                self.assert_owned_resources_removed()

    def test_start_failure_cleans_up_resources_created_before_start(self):
        self.assert_valid_control()
        for service, code in (("postgres", 20), ("hasura", 21)):
            with self.subTest(service=service):
                result = self.run_check(service + "-start-failure")
                self.assertEqual(result.returncode, code, result.stderr)
                self.assert_owned_resources_removed()

    def test_inconsistent_metadata_fails_and_cleans_up_its_resources(self):
        self.assert_valid_control()
        result = self.run_check("inconsistent-metadata")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("Hasura metadata is inconsistent", result.stderr)
        self.assert_owned_resources_removed()

    def test_reused_container_name_does_not_transfer_cleanup_ownership(self):
        self.assert_valid_control()
        result = self.run_check("name-reused")
        self.assertEqual(result.returncode, 0, result.stderr)
        remaining = json.loads(self.state.read_text())
        self.assertEqual(list(remaining), ["sentinel-hasura"])
        self.assertEqual(remaining["sentinel-hasura"]["owner"], "other")


if __name__ == "__main__":
    unittest.main()
