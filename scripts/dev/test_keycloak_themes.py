# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The development Keycloak serves the checkout's theme sources."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEVCONTAINER = ROOT / ".devcontainer"
THEME_RESOURCES = ROOT / "packages/keycloak-extensions/sequent-theme/src/main/resources"
THEMES_TARGET = "/opt/keycloak/themes"
# A mount entry: the shared anchor, then its source and target.
MOUNT = re.compile(
    r"<<: \*keycloak-theme\s+source: (?P<source>\S+)\s+target: (?P<target>\S+)"
)


def registered_themes() -> set[tuple[str, str]]:
    document = json.loads(
        (THEME_RESOURCES / "META-INF/keycloak-themes.json").read_text(encoding="utf-8")
    )
    return {
        (theme["name"], theme_type)
        for theme in document["themes"]
        for theme_type in theme["types"]
    }


def compose_available() -> bool:
    if shutil.which("docker") is None:
        return False
    return (
        subprocess.run(
            ["docker", "compose", "version"], capture_output=True, check=False
        ).returncode
        == 0
    )


def keycloak_volume_targets(*files: str) -> set[str]:
    """Bind targets of the merged keycloak service, as ``scripts/e2e/run.sh`` merges."""
    arguments = ["--env-file", str(DEVCONTAINER / ".env.development")]
    for name in files:
        arguments += ["-f", str(DEVCONTAINER / name)]
    completed = subprocess.run(
        ["docker", "compose", "--project-name", "theme-mount-check", *arguments]
        + ["--profile", "full", "--profile", "e2e", "config", "--format", "json"],
        capture_output=True,
        text=True,
        check=True,
        # What scripts/e2e/run.sh writes to its compose.env; only interpolated.
        env={
            **os.environ,
            "STEP_E2E_BIN_DIR": "/nonexistent",
            "STEP_E2E_OUTPUT_DIR": "/nonexistent",
            "STEP_E2E_UID": "1000",
            "STEP_E2E_GID": "1000",
        },
    )
    service = json.loads(completed.stdout)["services"]["keycloak"]
    return {volume["target"] for volume in service.get("volumes", [])}


def theme_mounts(compose_file: str) -> list[tuple[str, str]]:
    text = (DEVCONTAINER / compose_file).read_text(encoding="utf-8")
    return [(match["source"], match["target"]) for match in MOUNT.finditer(text)]


class ThemeMountTest(unittest.TestCase):
    def test_every_registered_theme_type_is_mounted_once(self):
        targets = [target for _, target in theme_mounts("docker-compose.yml")]
        self.assertEqual(len(targets), len(set(targets)))
        self.assertEqual(
            set(targets),
            {f"{THEMES_TARGET}/{name}/{kind}" for name, kind in registered_themes()},
        )

    def test_mounts_serve_the_matching_source_directory(self):
        # create_host_path is off: a missing source stops Keycloak from starting.
        prefix = "${LOCAL_WORKSPACE_FOLDER:-..}/"
        for source, target in theme_mounts("docker-compose.yml"):
            with self.subTest(target=target):
                self.assertTrue(source.startswith(prefix), source)
                relative = source.removeprefix(prefix)
                self.assertTrue((ROOT / relative).is_dir(), relative)
                self.assertEqual(
                    Path(relative).relative_to(THEME_RESOURCES.relative_to(ROOT)),
                    Path("theme") / Path(target).relative_to(THEMES_TARGET),
                )

    @unittest.skipUnless(compose_available(), "needs docker compose")
    def test_backend_e2e_uses_the_packaged_themes(self):
        def themes(targets: set[str]) -> set[str]:
            return {target for target in targets if target.startswith(THEMES_TARGET)}

        development = keycloak_volume_targets("docker-compose.yml")
        e2e = keycloak_volume_targets("docker-compose.yml", "docker-compose-ci.yml")
        self.assertTrue(themes(development))
        self.assertEqual(themes(e2e), set())
        self.assertIn("/opt/keycloak/data/import", e2e)


if __name__ == "__main__":
    unittest.main()
