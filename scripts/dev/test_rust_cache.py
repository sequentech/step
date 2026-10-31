# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.dev.rust_cache import cache_identity


class RustCacheTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.write("packages/Cargo.toml", '[workspace]\nmembers = ["leaf"]\n')
        self.write("packages/Cargo.lock", "version = 4\n")
        self.write(
            "packages/leaf/Cargo.toml", '[package]\nname = "leaf"\nversion = "0.1.0"\n'
        )
        self.write("packages/leaf/src/lib.rs", "pub fn value() -> u8 { 1 }\n")
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)

    def write(self, path, content):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)

    def identity(self, **overrides):
        arguments = dict(
            name="leaf",
            lockfile="packages/Cargo.lock",
            target_dir="packages/target",
            compiler="rustc 1.96.0\nhost: x86_64-unknown-linux-gnu",
            system="Linux",
            architecture="X64",
            profile="dev",
            features="default",
            targets="host",
            environment={},
        )
        arguments.update(overrides)
        return cache_identity(self.root, **arguments)

    def test_source_edits_reuse_compatible_units_but_never_change_target(self):
        before = self.identity()
        self.write("packages/leaf/src/lib.rs", "pub fn value() -> u8 { 2 }\n")
        self.assertEqual(before, self.identity())
        self.assertEqual(before["target-dir"], str(self.root / "packages/target"))

    def test_lock_edit_changes_snapshot_but_preserves_compatible_restore(self):
        before = self.identity()
        self.write("packages/Cargo.lock", "version = 4\n# updated dependency\n")
        after = self.identity()
        self.assertNotEqual(before["key"], after["key"])
        self.assertEqual(before["restore-prefix"], after["restore-prefix"])
        self.assertNotEqual(before["registry-key"], after["registry-key"])

    def test_build_dimensions_cannot_restore_incompatible_snapshot(self):
        before = self.identity()["restore-prefix"]
        for change in [
            {"compiler": "rustc 1.97.0"},
            {"system": "macOS"},
            {"architecture": "ARM64"},
            {"profile": "release"},
            {"features": "default,keycloak"},
            {"targets": "wasm32-unknown-unknown"},
            {"epoch": "v2"},
            {"environment": {"RUSTFLAGS": "-C target-cpu=native"}},
            {"environment": {"CARGO_PROFILE_DEV_DEBUG": "0"}},
            {"environment": {"CARGO_ENCODED_RUSTFLAGS": "-C\x1fdebuginfo=0"}},
        ]:
            with self.subTest(change=change):
                self.assertNotEqual(before, self.identity(**change)["restore-prefix"])

    def test_manifest_and_cargo_config_invalidate_compatible_restore(self):
        before = self.identity()["restore-prefix"]
        self.write(
            "packages/leaf/Cargo.toml", '[package]\nname = "leaf"\nversion = "0.2.0"\n'
        )
        self.assertNotEqual(before, self.identity()["restore-prefix"])
        before = self.identity()["restore-prefix"]
        self.write(
            "packages/.cargo/config.toml",
            '[build]\ntarget = "wasm32-unknown-unknown"\n',
        )
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        self.assertNotEqual(before, self.identity()["restore-prefix"])

    def test_dead_lock_path_is_rejected(self):
        with self.assertRaises(FileNotFoundError):
            self.identity(lockfile="packages/leaf/Cargo.lock")

    def test_checkout_root_and_workspace_cannot_be_target_directories(self):
        for target in [".", "packages", "../elsewhere"]:
            with self.subTest(target=target), self.assertRaises(ValueError):
                self.identity(target_dir=target)

    def test_runtime_only_variables_do_not_flush_the_cache(self):
        self.assertEqual(
            self.identity(), self.identity(environment={"RUST_BACKTRACE": "full"})
        )


if __name__ == "__main__":
    unittest.main()
