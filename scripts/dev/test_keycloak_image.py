# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The Keycloak image provides extension test inputs and realm login themes."""

from __future__ import annotations

import json
import re
import shlex
import subprocess
import tempfile
import unittest
import xml.etree.ElementTree as ElementTree
import zipfile
from pathlib import Path

from scripts.dev.keycloak import PACKAGE, THEMES

ROOT = Path(__file__).resolve().parents[2]
DOCKERFILE = ROOT / "packages/Dockerfile.keycloak"
OFFLINE_DEPENDENCIES = ROOT / "packages/Dockerfile.offline-dependencies"
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
    def test_the_extension_build_provides_the_shared_rust_contract_sources(self):
        # Maven's OTP contract tests read these siblings of keycloak-extensions.
        dockerfile = DOCKERFILE.read_text().split("AS oid4vp-build", 1)[0]
        ignore = (ROOT / "packages/Dockerfile.keycloak.dockerignore").read_text()
        for filename in ("messaging.rs", "dial_codes.rs"):
            source = f"sequent-core/src/types/{filename}"
            with self.subTest(source=source):
                self.assertTrue((ROOT / "packages" / source).is_file())
                self.assertIn(f"!{source}", ignore.splitlines())
                self.assertRegex(
                    dockerfile,
                    rf"COPY\s+\./{re.escape(source)}\s+"
                    r"/build/context/packages/sequent-core/src/types/",
                )
        for directory in ("sequent-core", "sequent-core/src", "sequent-core/src/types"):
            with self.subTest(directory=directory):
                self.assertIn(f"!{directory}", ignore.splitlines())

    def test_the_offline_dependency_build_provides_the_shared_rust_contract_sources(self):
        # Its Maven stage runs the same extension tests while vendoring, so
        # airgap/prepare.sh fails when they cannot read these files.
        dockerfile = OFFLINE_DEPENDENCIES.read_text()
        stage = dockerfile.split("AS keycloak-m2", 1)[1].split("RUN mvn", 1)[0]
        for filename in ("messaging.rs", "dial_codes.rs"):
            source = f"sequent-core/src/types/{filename}"
            with self.subTest(source=source):
                self.assertRegex(
                    stage,
                    rf"COPY\s+\./{re.escape(source)}\s+"
                    r"/build/packages/sequent-core/src/types/",
                )

    def test_the_image_installs_the_prepared_react_themes(self):
        dockerfile = DOCKERFILE.read_text()
        self.assertIn(
            "python3 -m scripts.dev.keycloak prepare --runtime built", dockerfile
        )
        self.assertRegex(
            dockerfile,
            rf"COPY [^\n]*--from=themes-build [^\n]*/{re.escape(str(THEMES))}/ "
            r"/\s*\n",
        )
        # The intermediate context lets controlled builds omit React themes;
        # ordinary builds must still install its contents in Keycloak.
        self.assertIn("FROM scratch AS keycloak-ui-themes", dockerfile)
        self.assertRegex(
            dockerfile,
            r"COPY [^\n]*--from=keycloak-ui-themes / /opt/keycloak/themes/",
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


class KeycloakEvaluatedProviderTests(unittest.TestCase):
    def test_production_reactor_does_not_activate_the_hmac_provider(self):
        # The private reactor currently consists solely of the HMAC login route.
        # Its file-activated profile must be explicitly disabled in production.
        command = next(line.strip().rstrip("; \\") for line in DOCKERFILE.read_text().splitlines()
                       if line.strip().startswith("mvn -B") and "keycloak_extensions_dir" in line)
        arguments = shlex.split(command)
        self.assertIn("-P", arguments)
        self.assertEqual(arguments[arguments.index("-P") + 1], "!beyond-keycloak-extensions")

    def test_provider_assembly_rejects_a_reintroduced_hmac_jar(self):
        # Run the actual assembly guard against a clean directory and one bad input.
        guard = next((line.strip().rstrip("; \\") for line in DOCKERFILE.read_text().splitlines()
                      if line.strip().startswith("test -z") and "smart-link-hmac" in line), None)
        self.assertIsNotNone(guard, "The delivered provider set needs a fail-closed HMAC guard")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            command = guard.replace("/build/keycloak-provider-jars", str(root))
            (root / "sequent.idp-linking-authenticator.jar").write_bytes(b"retained provider")
            self.assertEqual(subprocess.run(["sh", "-c", command]).returncode, 0)
            (root / "sequent.smart-link-hmac-authenticator.jar").write_bytes(b"forbidden provider")
            self.assertNotEqual(subprocess.run(["sh", "-c", command]).returncode, 0)

    def test_only_the_sign_in_links_feature_builds_the_sign_in_link_handlers(self):
        # Run the Dockerfile's own selection of the Maven profile for several
        # FEATURES values.
        lines = [line.strip().rstrip("; \\")
                 for line in DOCKERFILE.read_text().splitlines()]
        selection = [line for line in lines if line.startswith("sign_in_links=")
                     or (line.startswith("case ") and "sign-in-links" in line)]
        self.assertEqual(len(selection), 2)
        script = ('FEATURES="$1"; ' + "; ".join(selection)
                  + '; printf %s "${sign_in_links}"')
        disabled = "-P !sign-in-link-handlers"
        expected = {"": disabled, "plugins": disabled, "sign-in-links-extra": disabled,
                    "sign-in-links": "", "plugins,sign-in-links": "",
                    "plugins sign-in-links": ""}
        for features, profile in expected.items():
            with self.subTest(features=features):
                result = subprocess.run(["sh", "-c", script, "sh", features],
                                        capture_output=True, text=True, check=True)
                self.assertEqual(result.stdout, profile)

    def test_provider_assembly_rejects_sign_in_link_handlers_without_the_feature(self):
        guards = [line.strip().rstrip("; \\")
                  for line in DOCKERFILE.read_text().splitlines()
                  if line.strip().startswith("test -z")
                  and ("sign-in-link-handlers" in line
                       or "OTLActionTokenHandler" in line)]
        self.assertEqual(len(guards), 2,
                         "The provider set needs fail-closed sign-in link guards")
        handlers = "sequent/keycloak/sign_in_links/"
        forbidden = {"sequent.sign-in-link-handlers.jar": None,
                     "sequent.renamed.jar": handlers + "OTLActionTokenHandler.class",
                     "sequent.other.jar":
                         handlers + "ManualVerificationTokenHandler.class",
                     "sequent.bridge.jar":
                         handlers + "LoginBridgeActionTokenHandler.class",
                     "sequent.legacy.jar":
                         "sequent/keycloak/login_bridge/LoginBridgeActionTokenHandlerFactory.class",
                     "sequent.issuer.jar":
                         handlers + "ManualVerificationProvider.class",
                     "sequent.verification.jar":
                         "sequent/keycloak/conditional_authenticators/ManualVerificationProviderFactory.class",
                     "sequent.smart.jar":
                         "sequent/keycloak/authenticator/smart_link/SmartLinkAuthenticatorFactory.class"}
        for name, entry in forbidden.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                command = " && ".join(
                    guard.replace("/build/keycloak-provider-jars", str(root))
                    for guard in guards)
                # The token classes and the manual verification settings stay in
                # every build.
                with zipfile.ZipFile(root / "sequent.conditional-authenticators.jar",
                                     "w", zipfile.ZIP_DEFLATED) as jar:
                    for entry_name in ("ManualVerificationToken.class",
                                       "ManualVerificationConfigAuthenticator.class"):
                        jar.writestr(
                            "sequent/keycloak/conditional_authenticators/" + entry_name,
                            b"retained provider")
                with zipfile.ZipFile(root / "sequent.message-otp-authenticator.jar",
                                     "w", zipfile.ZIP_DEFLATED) as jar:
                    jar.writestr("sequent/keycloak/authenticator/otl/OTLActionToken.class",
                                 b"retained provider")
                with zipfile.ZipFile(root / "sequent.action-token-login-bridge.jar",
                                     "w", zipfile.ZIP_DEFLATED) as jar:
                    for entry_name in ("LoginBridgeActionToken.class",
                                       "LoginBridge.class"):
                        jar.writestr("sequent/keycloak/login_bridge/" + entry_name,
                                     b"retained provider")
                self.assertEqual(subprocess.run(["sh", "-c", command]).returncode, 0)
                with zipfile.ZipFile(root / name, "w", zipfile.ZIP_DEFLATED) as jar:
                    jar.writestr(entry or "META-INF/MANIFEST.MF", b"forbidden provider")
                self.assertNotEqual(subprocess.run(["sh", "-c", command]).returncode, 0)

    def test_the_sign_in_link_handlers_are_a_separate_profile_module(self):
        extensions = ROOT / "packages/keycloak-extensions"
        for name in ("ManualVerificationTokenHandler.java",
                     "OTLActionTokenHandler.java",
                     "LoginBridgeActionTokenHandler.java",
                     "LoginBridgeActionTokenHandlerFactory.java",
                     "ManualVerificationProvider.java",
                     "ManualVerificationProviderFactory.java",
                     "SmartLinkAuthenticator.java",
                     "SmartLinkAuthenticatorFactory.java",
                     "SmartLinkResource.java"):
            with self.subTest(name=name):
                found = [path.relative_to(extensions).parts[0]
                         for path in extensions.glob("*/src/main/java/**/" + name)]
                self.assertEqual(found, ["sign-in-link-handlers"])
        pom = ElementTree.parse(extensions / "pom.xml")
        maven = {"m": "http://maven.apache.org/POM/4.0.0"}
        self.assertNotIn("sign-in-link-handlers",
                         [module.text
                          for module in pom.findall("m:modules/m:module", maven)])
        profile = next(profile for profile in pom.findall("m:profiles/m:profile", maven)
                       if profile.findtext("m:id", namespaces=maven)
                       == "sign-in-link-handlers")
        self.assertEqual([module.text
                          for module in profile.findall("m:modules/m:module", maven)],
                         ["sign-in-link-handlers"])

    def test_development_and_ci_builds_include_the_sign_in_link_handlers(self):
        # Only the evaluated build, which passes no FEATURES, leaves them out.
        for path in IMAGE_BUILDS:
            with self.subTest(path=path):
                text = (ROOT / path).read_text()
                self.assertTrue("FEATURES=sign-in-links" in text
                                or "features: sign-in-links" in text)


if __name__ == "__main__":
    unittest.main()
