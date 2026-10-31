# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Compatible frontend outputs survive leaf edits and partial job retries."""

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from scripts.dev import frontend_cache as cache


class FrontendCacheTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "--quiet", str(self.root)], check=True)
        self.model = SimpleNamespace(
            units={
                "ui-core": SimpleNamespace(path="packages/ui-core", depends=set()),
                "ui-essentials": SimpleNamespace(
                    path="packages/ui-essentials", depends={"ui-core"}
                ),
            }
        )
        self.addCleanup(mock.patch.stopall)
        mock.patch.object(cache, "load_model", return_value=self.model).start()
        self.real_run = cache.run
        self.node_version = "v22.22.0"
        self.yarn_version = "1.22.22"
        mock.patch.object(cache, "run", side_effect=self.command).start()
        for path, contents in {
            "packages/ui-core/src/index.tsx": "export const value = 1",
            "packages/ui-core/webpack.config.cjs": "module.exports = {}",
            "packages/ui-core/rust/sequent-core-0.1.0.tgz": "wasm archive",
            "packages/ui-core/package.json": "{}",
            "packages/ui-essentials/src/index.tsx": "export const component = 1",
            "packages/ui-essentials/package.json": "{}",
            "packages/results-portal/src/App.tsx": "export const app = 1",
            "packages/results-portal/package.json": "{}",
            "packages/package.json": "{}",
            "packages/yarn.lock": "lockfile",
            "scripts/dev/frontend_cache.py": "recipe",
        }.items():
            self.write(path, contents)
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)

    def command(self, root, *arguments):
        if arguments == ("node", "--version"):
            return self.node_version
        if arguments == ("yarn", "--version"):
            return self.yarn_version
        return self.real_run(root, *arguments)

    def write(self, path, contents):
        destination = self.root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(contents)

    def key(self):
        return cache.fingerprint(cache.identity(self.root))

    def test_unchanged_and_portal_leaf_edits_reuse_shared_output_identity(self):
        first = self.key()
        self.assertEqual(self.key(), first)
        self.write("packages/results-portal/src/App.tsx", "export const app = 2")
        self.assertEqual(self.key(), first)

    def test_source_config_lock_workspace_manifest_wasm_and_recipe_invalidate(self):
        for path in (
            "packages/ui-core/src/index.tsx",
            "packages/ui-core/webpack.config.cjs",
            "packages/ui-essentials/src/index.tsx",
            "packages/yarn.lock",
            "packages/results-portal/package.json",
            "packages/ui-core/rust/sequent-core-0.1.0.tgz",
            "scripts/dev/frontend_cache.py",
        ):
            with self.subTest(path=path):
                first = self.key()
                self.write(path, (self.root / path).read_text() + " changed")
                self.assertNotEqual(self.key(), first)

    def test_new_source_file_invalidates(self):
        first = self.key()
        self.write("packages/ui-core/src/new.ts", "export const added = 1")
        self.assertNotEqual(self.key(), first)

    def test_node_yarn_architecture_and_build_environment_invalidate(self):
        first = self.key()
        self.node_version = "v22.23.0"
        self.assertNotEqual(self.key(), first)
        first = self.key()
        self.yarn_version = "1.22.23"
        self.assertNotEqual(self.key(), first)
        first = self.key()
        with mock.patch.object(cache.platform, "machine", return_value="other-arch"):
            self.assertNotEqual(self.key(), first)
        with mock.patch.dict(os.environ, {"NODE_OPTIONS": "--max-old-space-size=4096"}):
            self.assertNotEqual(self.key(), first)

    def built_outputs(self):
        self.write("packages/ui-core/dist/index.js", "built core")
        self.write("packages/ui-essentials/dist/index.js", "built components")

    def test_missing_outputs_are_not_reused(self):
        self.assertFalse(cache.verify(self.root, "shared-ui", "inputs"))

    def test_valid_output_reused_but_stale_missing_corrupt_or_extra_output_rejected(
        self,
    ):
        self.built_outputs()
        cache.stamp(self.root, "shared-ui", "inputs")
        self.assertTrue(cache.verify(self.root, "shared-ui", "inputs"))
        self.assertFalse(cache.verify(self.root, "shared-ui", "other-inputs"))
        self.write("packages/ui-core/dist/index.js", "corrupted")
        self.assertFalse(cache.verify(self.root, "shared-ui", "inputs"))
        self.built_outputs()
        (self.root / "packages/ui-core/dist/index.js").unlink()
        self.assertFalse(cache.verify(self.root, "shared-ui", "inputs"))
        self.built_outputs()
        self.write("packages/ui-core/dist/stale.js", "unexpected")
        self.assertFalse(cache.verify(self.root, "shared-ui", "inputs"))

    def test_partial_retry_finds_previous_successful_producer_and_validates_content(
        self,
    ):
        self.built_outputs()
        key = self.key()
        cache.stamp(self.root, "shared-ui", key)
        with mock.patch.dict(os.environ, {"GITHUB_RUN_ATTEMPT": "1"}):
            published = {cache.artifact_name("shared-ui", "123"): key}
        with mock.patch.dict(os.environ, {"GITHUB_RUN_ATTEMPT": "2"}):
            consumed = published[cache.artifact_name("shared-ui", "123")]
            self.assertTrue(cache.verify(self.root, "shared-ui", consumed))
            self.assertNotIn(cache.artifact_name("shared-ui", "124"), published)

    def test_portal_artifact_from_another_revision_is_rejected(self):
        self.write("packages/results-portal/dist/index.html", "portal")
        cache.stamp(self.root, "results-portal", "commit1")
        self.assertTrue(cache.verify(self.root, "results-portal", "commit1"))
        self.assertFalse(cache.verify(self.root, "results-portal", "commit2"))

    def test_unknown_manifest_schema_is_rejected(self):
        self.built_outputs()
        cache.stamp(self.root, "shared-ui", "inputs")
        path = self.root / cache.MANIFEST_DIRECTORY / "shared-ui.json"
        manifest = json.loads(path.read_text())
        manifest["schema"] = 999
        path.write_text(json.dumps(manifest))
        self.assertFalse(cache.verify(self.root, "shared-ui", "inputs"))


if __name__ == "__main__":
    unittest.main()
