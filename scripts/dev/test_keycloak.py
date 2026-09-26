# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The opt-in themes keep the auth server and inherited pages authoritative."""

from __future__ import annotations

import argparse
import tempfile
import unittest
import zipfile
from pathlib import Path

from scripts.dev.keycloak import (
    PACKAGE,
    SOURCE,
    THEMES,
    Runtime,
    mount_command,
    page_template,
    prepare,
    theme_source,
    upstream,
)
from scripts.dev.mode.checkout import Checkout

HTML = (
    '<html><head><script type="module" src="/assets/app.js"></script>'
    "</head><body></body></html>"
)


class KeycloakThemeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        package = self.root / PACKAGE
        package.mkdir(parents=True)
        (package / "themes.json").write_text(
            '{"sequent-ui-admin": "sequent.admin-portal"}'
        )
        (package / "context.ftl").write_text(
            "<script>window.kcContext.sequent = {};</script>"
        )
        original = self.root / SOURCE / "sequent.admin-portal/login"
        original.mkdir(parents=True)
        (original / "login.ftl").write_text("Original profile and credential widgets")
        (original / "template.ftl").write_text(
            "<head></head><body>Original layout</body>"
        )
        output = package / "dist_keycloak"
        output.mkdir()
        with zipfile.ZipFile(output / "sequent-ui.jar", "w") as jar:
            for page in ("login.ftl", "message-otp.login.ftl", "register.ftl"):
                jar.writestr("theme/sequent-ui-admin/login/" + page, HTML)
            jar.writestr("theme/sequent-ui-admin/login/resources/dist/app.js", "built")

    def test_hot_pages_load_vite_and_inherit_registration(self):
        prepare(self.root, Runtime.HOT, skip_build=True)
        login = self.root / THEMES / "sequent-ui-admin/login"
        page = (login / "login.ftl").read_text()
        self.assertIn('src="/@vite/client"', page)
        self.assertIn('src="/src/main.tsx"', page)
        self.assertNotIn("/assets/app.js", page)
        self.assertIn("window.kcContext.sequent", page)
        self.assertIn('<#include "sequent-login.ftl">', page)
        self.assertEqual(
            (login / "sequent-login.ftl").read_text(),
            "Original profile and credential widgets",
        )
        self.assertEqual(
            (login / "theme.properties").read_text(),
            "parent=sequent.admin-portal\nimport=common/keycloak\n",
        )
        self.assertFalse((login / "register.ftl").exists())
        self.assertIn("/@vite/client", (login / "template.ftl").read_text())

    def test_built_pages_keep_compiled_entry_and_remove_dev_client(self):
        prepare(self.root, Runtime.HOT, skip_build=True)
        prepare(self.root, Runtime.BUILT, skip_build=True)
        login = self.root / THEMES / "sequent-ui-admin/login"
        self.assertIn("/assets/app.js", (login / "login.ftl").read_text())
        self.assertNotIn("/@vite/client", (login / "login.ftl").read_text())
        self.assertNotIn("/@vite/client", (login / "template.ftl").read_text())
        self.assertEqual((login / "resources/dist/app.js").read_text(), "built")

    def test_unknown_generated_entry_fails_before_installing_broken_template(self):
        with self.assertRaisesRegex(ValueError, "unique module entry"):
            page_template(
                HTML.replace('type="module"', 'type="text/javascript"'),
                "",
                Runtime.HOT,
                login=True,
            )

    def test_voting_layout_follows_source_theme_inheritance(self):
        child = self.root / SOURCE / "sequent.voting-portal/login"
        child.mkdir(parents=True)
        (child / "theme.properties").write_text("parent=sequent.admin-portal")
        self.assertEqual(
            theme_source(self.root, "sequent.voting-portal", "template.ftl"),
            self.root / SOURCE / "sequent.admin-portal/login/template.ftl",
        )

    def test_source_theme_cycle_is_rejected(self):
        original = self.root / SOURCE / "sequent.admin-portal/login"
        (original / "theme.properties").write_text("parent=sequent.admin-portal")
        with self.assertRaisesRegex(ValueError, "No inherited"):
            theme_source(self.root, "sequent.admin-portal", "missing.ftl")

    def test_mount_requires_an_explicit_daemon_and_checkout_prefix(self):
        checkout = Checkout(
            self.root,
            {
                "COMPOSE_PROJECT_NAME": "isolated-ui",
                "DEVCONTAINER_NAME_PREFIX": "isolated-ui-",
            },
        )
        for host in ("", "unix:///var/run/docker.sock", "unix:///run/docker.sock"):
            with (
                self.subTest(host=host),
                self.assertRaisesRegex(ValueError, "isolated development daemon"),
            ):
                mount_command(checkout, host)
        prepare(self.root, Runtime.HOT, skip_build=True)
        command = mount_command(checkout, "unix:///isolated/docker.sock")
        self.assertEqual(
            command[:6],
            [
                "docker",
                "--host",
                "unix:///isolated/docker.sock",
                "compose",
                "--project-name",
                "isolated-ui",
            ],
        )
        self.assertEqual(command[-4:], ["up", "-d", "--no-build", "keycloak"])
        with self.assertRaisesRegex(ValueError, "Initialize this checkout"):
            mount_command(
                Checkout(self.root, {"COMPOSE_PROJECT_NAME": "step_devcontainer"}),
                "unix:///isolated/docker.sock",
            )

    def test_proxy_accepts_origins_without_credentials_or_paths(self):
        self.assertEqual(upstream("http://localhost:8090/"), "http://localhost:8090")
        for value in (
            "file:///tmp/server",
            "https://name:password@localhost",
            "http://localhost/realms/master",
            "http://localhost?secret=value",
        ):
            with (
                self.subTest(value=value),
                self.assertRaises(argparse.ArgumentTypeError),
            ):
                upstream(value)


if __name__ == "__main__":
    unittest.main()
