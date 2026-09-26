# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.dev import prebuild_smoke as smoke
from scripts.dev.bench.common import Run
from scripts.dev.bench.process import CommandResult
from scripts.dev.bench.results import Result, result_path
from scripts.dev.prebuild import IMAGE_LABEL

IMAGE = {"Id": "sha256:local", "Architecture": "arm64", "Os": "linux", "Size": 123}
VERSIONS = {
    "node": "v20.20.0",
    "yarn": "1.22.22",
    "rustc": "rustc 1.96.0",
    "cargo": "cargo 1.96.0",
    "wasm-pack": "wasm-pack 0.13.1",
    "wasm-bindgen": "wasm-bindgen 0.2.128",
}
READY = {"tools": VERSIONS, "python": "3.13.1", "wasm_target_ready": True}


class FakeDocker:
    """Models volume contents and ownership at the Docker boundary."""

    environment = {}

    def __init__(self):
        self.objects = {"container": {}, "volume": {}}
        self.used = set()
        self.starts = []
        self.peak_volumes = 0
        self.failure = None
        self.clock = 0
        self.duration = 2

    def run(self, kind, action, *arguments, **_kwargs):
        objects = self.objects[kind]
        name = arguments[-1]
        if action == "inspect":
            if name not in objects:
                return subprocess.CompletedProcess([], 1, "", f"No such {kind}")
            owner = {smoke.OWNER_LABEL: objects[name]}
            value = (
                {"Config": {"Labels": owner}}
                if kind == "container"
                else {"Labels": owner}
            )
            return subprocess.CompletedProcess([], 0, json.dumps([value]), "")
        if action == "create":
            self.assert_not_existing(name)
            objects[name] = arguments[1].split("=", 1)[1]
            self.peak_volumes = max(self.peak_volumes, len(objects))
        elif action == "rm":
            del objects[name]
        else:
            raise AssertionError((kind, action, arguments))
        return subprocess.CompletedProcess([], 0, name, "")

    def assert_not_existing(self, name):
        if name in self.objects["volume"]:
            raise AssertionError("reused a fresh volume")

    def start(self, invocation, *, log, **_kwargs):
        container = invocation[invocation.index("--name") + 1]
        volume = invocation[invocation.index("--mount") + 1].split(",")[1].split("=")[1]
        owner = invocation[invocation.index("--label") + 1].split("=", 1)[1]
        self.objects["container"][container] = owner
        self.starts.append(
            {
                "container": container,
                "volume": volume,
                "warm": volume in self.used,
                "command": invocation,
            }
        )
        self.used.add(volume)
        self.clock += self.duration
        if self.failure:
            log.write_text("actual container failure details\n")
            return CommandResult("docker run", self.failure, self.duration, "failure")
        log.write_text(
            "ordinary startup output\n" + smoke.READY_PREFIX + json.dumps(READY) + "\n"
        )
        return CommandResult("docker run", 0, self.duration, "ready")


class SmokeTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.output = self.root / "results"
        self.docker = FakeDocker()

    def series(self, **options):
        def run(**kwargs):
            output = kwargs.pop("output_dir")
            kwargs.pop("checkout")
            result = Result(
                **kwargs,
                checkout={"commit": "tested-head"},
                harness={},
                host={"machine": "aarch64"},
                tools={},
            )
            return Run(result, result_path(output, result))

        with (
            patch.object(smoke, "start_run", side_effect=run),
            patch.object(smoke, "run_command", side_effect=self.docker.start),
            patch.object(
                smoke.time, "monotonic", side_effect=lambda: self.docker.clock
            ),
        ):
            return smoke.measure(self.root, self.output, self.docker, IMAGE, **options)

    def test_fresh_stores_are_distinct_and_warmup_does_not_enter_summary(self):
        runs = self.series()
        starts = self.docker.starts
        self.assertEqual([item["warm"] for item in starts], [False] * 3 + [True] * 11)
        self.assertEqual(len({item["volume"] for item in starts[:3]}), 3)
        self.assertEqual({item["volume"] for item in starts[3:]}, {starts[2]["volume"]})
        self.assertEqual(len({item["container"] for item in starts}), 14)
        self.assertEqual(self.docker.peak_volumes, 1)
        self.assertEqual(self.docker.objects, {"container": {}, "volume": {}})
        cold, warm = [json.loads(run.path.read_text()) for run in runs]
        self.assertEqual(cold["summary"]["n"], 3)
        self.assertEqual(warm["summary"]["n"], 10)
        self.assertEqual(warm["samples"][0]["role"], "warmup")
        self.assertEqual(warm["samples"][1]["detail"]["tools"], VERSIONS)
        for item in starts:
            command = item["command"]
            self.assertEqual(command[command.index("--pull") + 1], "never")
            self.assertEqual(command[command.index("--network") + 1], "none")
            self.assertEqual(
                command[command.index("--tmpfs") + 1],
                "/nix/var/nix/daemon-socket:mode=0755",
            )
            self.assertIn("sha256:local", command)
            self.assertFalse(
                any("type=bind" in arg or "docker.sock" in arg for arg in command)
            )

    def test_failed_or_timed_out_readiness_records_failure_and_cleans_resources(self):
        for code in (1, -9):
            with self.subTest(code=code):
                self.docker = FakeDocker()
                self.docker.failure = code
                with self.assertRaisesRegex(smoke.SmokeError, "readiness failed"):
                    self.series()
                files = list(self.output.glob("prebuild-startup/*--cold--*.json"))
                report = json.loads(files[0].read_text())
                self.assertEqual(report["summary"]["failed"], 1)
                self.assertIsNone(report["samples"][0]["seconds"])
                self.assertIn(
                    "actual container failure details",
                    next(self.output.rglob("measured-1.log")).read_text(),
                )
                self.assertEqual(self.docker.objects, {"container": {}, "volume": {}})

    def test_time_budget_keeps_one_real_fresh_sample_and_labels_the_limitation(self):
        self.docker.duration = 30
        runs = self.series(warm=2, budget=180)
        cold, warm = [json.loads(run.path.read_text()) for run in runs]
        self.assertEqual(cold["summary"]["n"], 1)
        self.assertIn("limited to 1/3", cold["notes"][0])
        self.assertEqual(warm["summary"]["n"], 2)

    def test_zero_exit_without_the_tools_and_wasm_target_is_not_ready(self):
        log = self.root / "log"
        for result in (
            {},
            [],
            {**READY, "tools": []},
            {**READY, "wasm_target_ready": False},
            {**READY, "tools": {**VERSIONS, "wasm-bindgen": ""}},
        ):
            log.write_text(smoke.READY_PREFIX + json.dumps(result))
            with self.assertRaisesRegex(smoke.SmokeError, "complete toolchain"):
                smoke.readiness(log)
        log.write_text(smoke.READY_PREFIX + json.dumps(READY))
        self.assertEqual(smoke.readiness(log), READY)

    def test_cleanup_refuses_a_foreign_resource_even_if_its_name_is_tracked(self):
        owned = smoke.OwnedResources(self.docker, self.root / "cleanup.log")
        name = owned.volume()
        self.docker.objects["volume"][name] = "someone-else"
        with self.assertRaisesRegex(smoke.SmokeError, "cleanup failed"):
            owned.cleanup()
        self.assertEqual(self.docker.objects["volume"][name], "someone-else")
        self.assertIn("another owner", (self.root / "cleanup.log").read_text())
        with self.assertRaisesRegex(smoke.SmokeError, "untracked"):
            owned.remove("volume", "unrelated-volume")

    def test_stale_or_non_native_images_are_rejected_without_running_them(self):
        from unittest.mock import Mock

        entry = {**IMAGE, "Config": {"Labels": {IMAGE_LABEL: "matching"}}}
        docker = Mock()
        with patch.object(smoke.platform, "machine", return_value="aarch64"):
            for change in (
                {"Architecture": "amd64"},
                {"Os": "windows"},
                {"Config": {"Labels": {IMAGE_LABEL: "stale"}}},
            ):
                docker.run.return_value = subprocess.CompletedProcess(
                    [], 0, json.dumps([{**entry, **change}])
                )
                with self.assertRaisesRegex(smoke.SmokeError, "fingerprint and native"):
                    smoke.local_image(docker, "candidate", "matching")
            docker.run.return_value = subprocess.CompletedProcess(
                [], 0, json.dumps([entry])
            )
            self.assertEqual(smoke.local_image(docker, "candidate", "matching"), IMAGE)
        self.assertTrue(
            all(
                call.args[:2] == ("image", "inspect")
                for call in docker.run.call_args_list
            )
        )

    def test_unreachable_daemon_preserves_owned_names_for_cleanup(self):
        owned = smoke.OwnedResources(self.docker, self.root / "cleanup.log")
        name = owned.volume()
        with patch.object(
            self.docker,
            "run",
            return_value=subprocess.CompletedProcess(
                [], 1, "", "Cannot connect to the Docker daemon"
            ),
        ):
            with self.assertRaisesRegex(smoke.SmokeError, "cleanup failed"):
                owned.cleanup()
        self.assertEqual(
            json.loads((self.root / "resources.json").read_text())["volumes"], [name]
        )
        self.assertIn("Cannot connect", (self.root / "cleanup.log").read_text())


if __name__ == "__main__":
    unittest.main()
