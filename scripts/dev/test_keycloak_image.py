# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The Keycloak image ships the React login themes the realms select."""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path

from scripts.dev.keycloak import PACKAGE, THEMES

ROOT = Path(__file__).resolve().parents[2]
DOCKERFILE = ROOT / "packages/Dockerfile.keycloak"
COMELEC = ROOT / "packages/windmill/external-bin/janitor/templates/COMELEC/keycloak.hbs"
# Every build of the Keycloak image, and how each passes the scripts context.
IMAGE_BUILDS = {
    ".devcontainer/docker-compose.yml": "scripts: ../scripts",
    ".devcontainer/docker-compose-remote.yml": "scripts: ../scripts",
    ".github/workflows/reusable_build_push.yml": "scripts=./scripts",
    ".github/workflows/reusable_build_push_root.yml": "scripts=./scripts",
}


def react_themes() -> dict[str, str]:
    return json.loads((ROOT / PACKAGE / "themes.json").read_text())


def comelec_realm() -> dict:
    # The janitor fills the Handlebars placeholders; their values don't matter here.
    template = re.sub(r"\{\{\{?[^}]*\}?\}\}", "0", COMELEC.read_text())
    return json.loads(template)


class KeycloakImageTests(unittest.TestCase):
    def test_the_image_installs_the_prepared_react_themes(self):
        dockerfile = DOCKERFILE.read_text()
        self.assertIn("python3 -m scripts.dev.keycloak prepare --runtime built", dockerfile)
        self.assertRegex(
            dockerfile,
            rf"COPY [^\n]*--from=themes-build [^\n]*/{re.escape(str(THEMES))}/ "
            r"/opt/keycloak/themes/",
        )

    def test_the_theme_build_provides_the_sequent_core_peer_dependency(self):
        # The login themes compile ui-core sources that import sequent-core,
        # which the portals provide and this image does not install.
        dockerfile = DOCKERFILE.read_text()
        package = json.loads((ROOT / "packages/ui-core/package.json").read_text())
        tarball = package["peerDependencies"]["sequent-core"].removeprefix("file:./")
        self.assertRegex(
            dockerfile,
            rf"tar -xzf ui-core/{re.escape(tarball)} -C node_modules/sequent-core\b",
        )

    def test_every_image_build_passes_the_scripts_context(self):
        for path, context in IMAGE_BUILDS.items():
            with self.subTest(path=path):
                self.assertIn(context, (ROOT / path).read_text())


class ComelecThemeTests(unittest.TestCase):
    def test_identity_verification_runs_on_a_react_login_theme(self):
        # The Scanovate capture page only exists in the React themes; the
        # FreeMarker one just says the step needs them.
        realm = comelec_realm()
        scanovate = [
            execution
            for flow in realm["authenticationFlows"]
            for execution in flow["authenticationExecutions"]
            if execution.get("authenticator") == "scanovate-authenticator"
        ]
        self.assertTrue(scanovate)
        self.assertEqual(realm["loginTheme"], "sequent-ui-voting")
        for client in realm["clients"]:
            theme = client.get("attributes", {}).get("login_theme")
            if theme is not None:
                with self.subTest(client=client["clientId"]):
                    self.assertEqual(theme, "sequent-ui-voting")

    def test_the_voting_theme_inherits_the_comelec_pages(self):
        # Pages not ported to React, like the enrollment form, keep rendering
        # the FreeMarker theme the realm used before, which still owns accounts.
        realm = comelec_realm()
        self.assertEqual(react_themes()["sequent-ui-voting"], realm["accountTheme"])


if __name__ == "__main__":
    unittest.main()
