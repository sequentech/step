# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""How the Rust services build: together, once, and linked with LLD on aarch64."""

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
PACKAGES = ROOT / "packages"
# Service: the binary its watcher runs, of package harvest or windmill.
SHARED_BUILD = {"harvest": "harvest", "windmill": "main", "beat": "beat"}
LINKER = "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER"
LINKER_SCRIPT = DEVCONTAINER / "scripts" / "rust-lld-cc.sh"


def compose_services():
    """Services of every profile, without contacting a Docker daemon."""
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
            "--format",
            "json",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(completed.stdout)["services"]


def available(*command):
    if shutil.which(command[0]) is None:
        return False
    completed = subprocess.run(command, capture_output=True, check=False)
    return completed.returncode == 0


def watched_commands(command):
    """The shell commands cargo watch runs, split into their Cargo invocations."""
    script = command[command.index("-s") + 1]
    return [shlex.split(part) for part in script.split(";")]


class SharedBuildTest(unittest.TestCase):
    @unittest.skipUnless(available("docker", "compose", "version"), "needs compose")
    def test_services_sharing_the_target_dir_build_all_of_it_first(self):
        services = compose_services()
        builds = {}
        for name, binary in SHARED_BUILD.items():
            with self.subTest(service=name):
                command = services[name]["command"]
                self.assertIn("-s", command)
                build, run = watched_commands(command)
                builds[name] = build
                self.assertEqual(build[:2], ["cargo", "build"])
                # A failed build of another service's binary must not keep this
                # one down: its own cargo run still builds and runs what it can.
                self.assertIn("--keep-going", build)
                self.assertEqual(run[:2], ["cargo", "run"])
                if "--bin" in run:
                    self.assertEqual(run[run.index("--bin") + 1], binary)
        # One Cargo invocation compiles every binary, so the first watcher to
        # take the build directory lock builds them all in parallel.
        self.assertEqual(sorted(builds), sorted(SHARED_BUILD))
        self.assertEqual(len({tuple(build) for build in builds.values()}), 1)
        build = builds["harvest"]
        selected = {build[i + 1] for i, part in enumerate(build) if part == "--bin"}
        self.assertEqual(selected, set(SHARED_BUILD.values()))

    @unittest.skipUnless(available("docker", "compose", "version"), "needs compose")
    def test_services_sharing_the_target_dir_agree_on_fingerprinted_settings(self):
        # Cargo fingerprints every unit with RUSTFLAGS and the linker, so a
        # difference would rebuild the other services' units on each build.
        services = compose_services()
        for key in ("RUSTFLAGS", LINKER):
            with self.subTest(key=key):
                values = {
                    services[name]["environment"].get(key) for name in SHARED_BUILD
                }
                self.assertEqual(len(values), 1, values)
        linker = services["harvest"]["environment"][LINKER]
        self.assertEqual(Path(linker).name, LINKER_SCRIPT.name)
        self.assertTrue(os.access(LINKER_SCRIPT, os.X_OK))


class LinkerScriptTest(unittest.TestCase):
    """rust-lld-cc.sh, run with fake cc and rust-lld on PATH."""

    def link(self, with_lld):
        with tempfile.TemporaryDirectory() as directory:
            tools = Path(directory) / "bin"
            tools.mkdir()
            cc = tools / "cc"
            cc.write_text('#!/bin/sh\nprintf "%s\\n" "$@"\n')
            cc.chmod(0o755)
            if with_lld:
                gcc_ld = tools / "gcc-ld"
                gcc_ld.mkdir()
                for tool in (tools / "rust-lld", gcc_ld / "ld.lld"):
                    tool.write_text("#!/bin/sh\n")
                    tool.chmod(0o755)
            completed = subprocess.run(
                [str(LINKER_SCRIPT), "-o", "out", "main.o"],
                env={"PATH": f"{tools}:/usr/bin:/bin"},
                capture_output=True,
                text=True,
                check=True,
            )
            return completed.stdout.splitlines(), tools

    def test_links_with_the_toolchain_lld_next_to_rust_lld(self):
        arguments, tools = self.link(with_lld=True)
        self.assertEqual(
            arguments, ["-fuse-ld=lld", f"-B{tools}/gcc-ld", "-o", "out", "main.o"]
        )

    def test_falls_back_to_the_default_linker_without_rust_lld(self):
        arguments, _ = self.link(with_lld=False)
        self.assertEqual(arguments, ["-o", "out", "main.o"])


def resolved_features(package):
    """Each dependency of ``package`` with the features Cargo enables for it."""
    completed = subprocess.run(
        [
            "cargo",
            "tree",
            "--package",
            package,
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--no-dedupe",
            "--format",
            "{p}|{f}",
        ],
        cwd=PACKAGES,
        capture_output=True,
        text=True,
        check=True,
    )
    features = {}
    for line in completed.stdout.splitlines():
        name, _, enabled = line.partition("|")
        name = name.removesuffix(" (*)").strip()
        features.setdefault(name, set()).update(filter(None, enabled.split(",")))
    return features


class FeatureUnificationTest(unittest.TestCase):
    @unittest.skipUnless(available("cargo", "--version"), "needs cargo")
    def test_harvest_adds_no_features_to_windmill_dependencies(self):
        # Otherwise every crate between the differing dependency and windmill
        # compiles twice into packages/target: once for harvest's cargo run and
        # once for windmill's and beat's.
        harvest = resolved_features("harvest")
        windmill = resolved_features("windmill")
        differing = {
            name: sorted(harvest[name] ^ windmill[name])
            for name in windmill
            if harvest.get(name, windmill[name]) != windmill[name]
        }
        self.assertEqual(differing, {})


if __name__ == "__main__":
    unittest.main()
