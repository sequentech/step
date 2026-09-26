# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.dev.prebuild import (
    BASE_IMAGE,
    IMAGE_LABEL,
    INPUTS,
    compatible_image,
    fingerprint,
    image_reference,
    prepare_context,
    read_jsonc,
    selection,
)

ROOT = Path(__file__).resolve().parents[2]


class PrebuildTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        for name in INPUTS:
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((ROOT / name).read_bytes())

    def test_each_toolchain_and_feature_input_invalidates_tag(self):
        before = fingerprint(self.root)
        for name in INPUTS:
            with self.subTest(name=name):
                path = self.root / name
                original = path.read_bytes()
                path.write_bytes(original + b"\n")
                self.assertNotEqual(before, fingerprint(self.root))
                path.write_bytes(original)

    def test_source_and_local_environment_do_not_invalidate_or_enter_image(self):
        before = fingerprint(self.root)
        (self.root / ".devcontainer/.env").write_text("DO_NOT_PUBLISH=sentinel")
        (self.root / "private-source.ts").write_text("DO_NOT_PUBLISH=sentinel")
        context = self.root / "context"
        prepare_context(self.root, context, before)
        self.assertEqual(before, fingerprint(self.root))
        files = {
            str(path.relative_to(context))
            for path in context.rglob("*")
            if path.is_file()
        }
        self.assertEqual(
            files,
            {
                "devenv.nix",
                "devenv.lock",
                "devenv.yaml",
                ".devcontainer/Dockerfile",
                ".devcontainer/warm-env.sh",
                ".devcontainer/devcontainer.json",
                ".devcontainer/devcontainer-lock.json",
            },
        )
        self.assertFalse(
            any(
                "DO_NOT_PUBLISH" in path.read_text()
                for path in context.rglob("*")
                if path.is_file()
            )
        )
        expected = read_jsonc(self.root / ".devcontainer/devcontainer.json")["features"]
        self.assertEqual(
            json.loads((context / ".devcontainer/devcontainer.json").read_text())[
                "features"
            ],
            expected,
        )

    def test_image_recipe_and_compose_fallback_use_the_same_base(self):
        self.assertIn(
            f"FROM {BASE_IMAGE}",
            (ROOT / ".devcontainer/prebuild/Dockerfile").read_text(),
        )
        self.assertIn(
            "${DEVCONTAINER_IMAGE:-" + BASE_IMAGE + "}",
            (ROOT / ".devcontainer/docker-compose-base.yml").read_text(),
        )

    def test_existing_context_is_not_overwritten(self):
        target = self.root / "context"
        target.mkdir()
        (target / "keep").write_text("existing")
        with self.assertRaises(ValueError):
            prepare_context(self.root, target, fingerprint(self.root))
        self.assertEqual((target / "keep").read_text(), "existing")

    def test_missing_stale_or_other_architecture_images_fall_back(self):
        cases = [
            subprocess.CompletedProcess([], 1, "", "not found"),
            subprocess.CompletedProcess([], 0, "invalid json", ""),
            subprocess.CompletedProcess(
                [], 0, json.dumps([{"Config": {"Labels": None}}]), ""
            ),
            subprocess.CompletedProcess(
                [],
                0,
                json.dumps(
                    [
                        {
                            "Config": {"Labels": {IMAGE_LABEL: "old"}},
                            "Architecture": "arm64",
                            "Os": "linux",
                        }
                    ]
                ),
                "",
            ),
            subprocess.CompletedProcess(
                [],
                0,
                json.dumps(
                    [
                        {
                            "Config": {"Labels": {IMAGE_LABEL: "key"}},
                            "Architecture": "amd64",
                            "Os": "linux",
                        }
                    ]
                ),
                "",
            ),
        ]
        with patch("scripts.dev.prebuild.platform.machine", return_value="aarch64"):
            for result in cases:
                with (
                    self.subTest(result=result),
                    patch("scripts.dev.prebuild.subprocess.run", return_value=result),
                ):
                    available = compatible_image("test:env-key", "key")
                    self.assertFalse(available)
                    self.assertEqual(
                        selection("test:env-key", "key", available)[
                            "DEVCONTAINER_IMAGE"
                        ],
                        BASE_IMAGE,
                    )

    def test_compatible_image_gets_its_own_store_volume(self):
        response = json.dumps(
            [
                {
                    "Config": {"Labels": {IMAGE_LABEL: "key"}},
                    "Architecture": "arm64",
                    "Os": "linux",
                }
            ]
        )
        with (
            patch("scripts.dev.prebuild.platform.machine", return_value="aarch64"),
            patch(
                "scripts.dev.prebuild.subprocess.run",
                return_value=subprocess.CompletedProcess([], 0, response, ""),
            ),
        ):
            self.assertTrue(compatible_image("test:env-key", "key"))
        self.assertEqual(
            selection("test:env-key", "key", True)["DEVCONTAINER_NIX_VOLUME"],
            "step-devcontainer-nix-env-key",
        )

    def test_jsonc_preserves_urls_and_comment_markers_inside_strings(self):
        path = self.root / "config.jsonc"
        path.write_text('{// outside\n"url":"https://host/*literal*/", "text":"a,}",}')
        self.assertEqual(
            read_jsonc(path), {"url": "https://host/*literal*/", "text": "a,}"}
        )

    def test_unreachable_docker_does_not_block_local_fallback(self):
        for error in [
            OSError("not available"),
            subprocess.TimeoutExpired("docker", 15),
        ]:
            with (
                self.subTest(error=error),
                patch("scripts.dev.prebuild.subprocess.run", side_effect=error),
            ):
                self.assertFalse(compatible_image("test:env-key", "key"))

    def test_image_repository_cannot_inject_environment_lines(self):
        with self.assertRaises(ValueError):
            image_reference("key", "example/image\nUNRELATED=value")


if __name__ == "__main__":
    unittest.main()
