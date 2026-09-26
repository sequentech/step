# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Service feature isolation and the bundled linker's compiler-driver contract."""

import enum
import json
import os
import shlex
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEVCONTAINER = ROOT / ".devcontainer"
SERVICES = ("harvest", "windmill", "beat")
LINKER = "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER"
LINKER_SCRIPT = DEVCONTAINER / "scripts" / "rust-lld-cc.sh"
WORKSPACE = "/workspaces/rust-linker-contract"


def available(*command):
    if shutil.which(command[0]) is None:
        return False
    return subprocess.run(command, capture_output=True, check=False).returncode == 0


def compose_services():
    """Resolve paths and service arguments without contacting a Docker daemon."""
    completed = subprocess.run(
        [
            "docker",
            "compose",
            "--project-directory",
            str(DEVCONTAINER),
            "--env-file",
            str(DEVCONTAINER / ".env.development"),
            "--file",
            str(DEVCONTAINER / "docker-compose.yml"),
            "--profile",
            "*",
            "config",
            "--no-env-resolution",
            "--format",
            "json",
        ],
        env={**os.environ, "DEVCONTAINER_WORKSPACE_FOLDER": WORKSPACE},
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(completed.stdout)["services"]


@unittest.skipUnless(available("docker", "compose", "version"), "needs compose")
class ServiceConfigurationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.services = compose_services()

    def test_each_watcher_runs_its_service_with_its_own_feature_graph(self):
        for service, package, prefix in (
            ("harvest", "harvest", ["run"]),
            ("windmill", "windmill", ["run", "--bin", "main", "consume"]),
            ("beat", "windmill", ["run", "--bin", "beat"]),
        ):
            with self.subTest(service=service):
                configuration = self.services[service]
                self.assertEqual(
                    configuration["working_dir"], f"{WORKSPACE}/packages/{package}"
                )
                command = configuration["command"]
                self.assertEqual(command[:2], ["cargo", "watch"])
                # A cross-package prebuild unifies features that cargo run may
                # not enable, causing a second compilation instead of reuse.
                self.assertNotIn("-s", command)
                self.assertEqual(command.count("-x"), 1)
                cargo = shlex.split(command[command.index("-x") + 1])
                self.assertEqual(cargo[: len(prefix)], prefix)
                self.assertNotIn("--package", cargo)
                self.assertNotIn("-p", cargo)
                # Keep cargo-watch's metadata-derived local dependency scope.
                self.assertNotIn("--skip-local-deps", command)
                self.assertNotIn("--watch", command)
                self.assertNotIn("-w", command)

    def test_shared_target_uses_one_checkout_linker_and_compatible_flags(self):
        flags = set()
        for service in SERVICES:
            with self.subTest(service=service):
                environment = self.services[service]["environment"]
                self.assertEqual(
                    environment[LINKER],
                    f"{WORKSPACE}/.devcontainer/scripts/rust-lld-cc.sh",
                )
                flags.add(environment.get("RUSTFLAGS"))
        self.assertEqual(len(flags), 1)
        self.assertTrue(os.access(LINKER_SCRIPT, os.X_OK))


class BundledLinker(enum.Enum):
    AVAILABLE = "available"
    ABSENT = "absent"
    MISSING_DRIVER = "missing-driver"
    NON_EXECUTABLE_DRIVER = "non-executable-driver"


class LinkerScriptTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.tools = Path(self.temporary.name) / "compiler tools"
        self.tools.mkdir()
        self.calls = self.tools / "calls"
        self.cc = self.tools / "cc"
        self.cc.write_text(
            '#!/bin/sh\nprintf "called\\n" >> "$RUST_LINK_TEST_CALLS"\n'
            'printf "%s\\0" "$@"\nexit "$RUST_LINK_TEST_STATUS"\n'
        )
        self.cc.chmod(0o755)

    def link(self, linker, arguments, status=0):
        if linker is not BundledLinker.ABSENT:
            rust_lld = self.tools / "rust-lld"
            rust_lld.write_text("#!/bin/sh\nexit 99\n")
            rust_lld.chmod(0o755)
            if linker is not BundledLinker.MISSING_DRIVER:
                driver = self.tools / "gcc-ld" / "ld.lld"
                driver.parent.mkdir()
                driver.write_text("#!/bin/sh\nexit 99\n")
                if linker is BundledLinker.AVAILABLE:
                    driver.chmod(0o755)
        return subprocess.run(
            [str(LINKER_SCRIPT), *arguments],
            env={
                "PATH": str(self.tools),
                "RUST_LINK_TEST_CALLS": str(self.calls),
                "RUST_LINK_TEST_STATUS": str(status),
            },
            capture_output=True,
            check=False,
        )

    def arguments(self, completed):
        return completed.stdout.decode().removesuffix("\0").split("\0")

    def test_bundled_driver_preserves_debug_flags_and_argument_boundaries(self):
        arguments = [
            "-g",
            "input with spaces.o",
            "@linker response.rsp",
            "",
            "-o",
            "out",
        ]
        completed = self.link(BundledLinker.AVAILABLE, arguments)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(
            self.arguments(completed),
            ["-fuse-ld=lld", f"-B{self.tools}/gcc-ld", *arguments],
        )
        self.assertEqual(self.calls.read_text(), "called\n")

    def test_absent_bundled_driver_uses_the_default_compiler(self):
        for linker in (
            BundledLinker.ABSENT,
            BundledLinker.MISSING_DRIVER,
            BundledLinker.NON_EXECUTABLE_DRIVER,
        ):
            with (
                self.subTest(linker=linker),
                tempfile.TemporaryDirectory() as directory,
            ):
                # Separate tool directories prevent one case supplying another's LLD.
                original = self.tools
                self.tools = Path(directory)
                shutil.copy(self.cc, self.tools / "cc")
                try:
                    completed = self.link(linker, ["-g", "main.o", "-o", "out"])
                finally:
                    self.tools = original
                self.assertEqual(completed.returncode, 0, completed.stderr)
                self.assertEqual(
                    self.arguments(completed), ["-g", "main.o", "-o", "out"]
                )

    def test_unrelated_lld_on_path_does_not_replace_the_compiler_default(self):
        unrelated = self.tools / "ld.lld"
        unrelated.write_text("#!/bin/sh\nexit 99\n")
        unrelated.chmod(0o755)
        completed = self.link(BundledLinker.ABSENT, ["main.o"])
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(self.arguments(completed), ["main.o"])

    def test_link_failure_is_returned_without_retrying_a_different_linker(self):
        completed = self.link(BundledLinker.AVAILABLE, ["main.o"], status=37)
        self.assertEqual(completed.returncode, 37)
        self.assertEqual(self.calls.read_text(), "called\n")

    def test_default_compiler_failure_is_returned(self):
        completed = self.link(BundledLinker.ABSENT, ["main.o"], status=23)
        self.assertEqual(completed.returncode, 23)
        self.assertEqual(self.calls.read_text(), "called\n")

    def test_missing_compiler_fails_instead_of_reporting_a_successful_link(self):
        self.cc.unlink()
        completed = self.link(BundledLinker.AVAILABLE, ["main.o"])
        self.assertEqual(completed.returncode, 127)
        self.assertIn(b"cc", completed.stderr)
        self.assertFalse(self.calls.exists())


if __name__ == "__main__":
    unittest.main()
