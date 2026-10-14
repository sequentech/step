# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The opt-in themes keep the auth server and inherited pages authoritative."""

from __future__ import annotations

import argparse
import json
import tempfile
import unittest
import zipfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from threading import Barrier, Event
from unittest.mock import patch

from scripts.dev import keycloak
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
        (original / "register.ftl").write_text("Original registration form")
        (original / "template.ftl").write_text(
            "<head></head><body>Original layout</body>"
        )
        output = package / "dist_keycloak"
        output.mkdir()
        with zipfile.ZipFile(output / "sequent-ui.jar", "w") as jar:
            for page in (
                "login.ftl",
                "login-username.ftl",
                "message-otp.login.ftl",
                "scanovate-capture.ftl",
                "scanovate-confirmation.ftl",
                "scanovate-error.ftl",
                "register.ftl",
                "registration-finish.ftl",
                "registration-manual-finish.ftl",
                "registration-rejected-finish.ftl",
                "message-finish.ftl",
                "info.ftl",
            ):
                jar.writestr("theme/sequent-ui-admin/login/" + page, HTML)
            jar.writestr("theme/sequent-ui-admin/login/resources/dist/app.js", "built")

    def test_concurrent_atomic_writes_publish_complete_readable_assets(self):
        destination = self.root / "resources" / "shared.css"
        payloads = [b"first" * 1000, b"second" * 1000]
        ready = Barrier(2)
        replace = Path.replace

        def simultaneous_replace(source, target):
            ready.wait(timeout=5)
            return replace(source, target)

        with (
            patch.object(Path, "replace", simultaneous_replace),
            ThreadPoolExecutor(2) as pool,
        ):
            futures = [
                pool.submit(keycloak.write, destination, value) for value in payloads
            ]
            for future in futures:
                future.result(timeout=10)
        self.assertIn(destination.read_bytes(), payloads)
        self.assertTrue(destination.stat().st_mode & 0o004)
        self.assertEqual(list(destination.parent.iterdir()), [destination])

    def test_concurrent_prepares_reread_sources_after_the_previous_writer_finishes(
        self,
    ):
        source = self.root / SOURCE / "sequent.admin-portal/login/login.ftl"
        first_waiting, release_first, second_started, second_writing = (
            Event() for _ in range(4)
        )
        write = keycloak.write

        def hold_first_template(path, content):
            if path.name == "sequent-login.ftl":
                if content == "Original profile and credential widgets":
                    first_waiting.set()
                    self.assertTrue(release_first.wait(timeout=5))
                else:
                    second_writing.set()
            write(path, content)

        def second_prepare():
            second_started.set()
            prepare(self.root, Runtime.HOT, skip_build=True)

        with (
            patch.object(keycloak, "write", hold_first_template),
            ThreadPoolExecutor(2) as pool,
        ):
            first = pool.submit(prepare, self.root, Runtime.HOT, True)
            self.assertTrue(first_waiting.wait(timeout=5))
            source.write_text("Updated realm template")
            second = pool.submit(second_prepare)
            try:
                self.assertTrue(second_started.wait(timeout=5))
                self.assertFalse(second_writing.wait(timeout=0.2))
            finally:
                release_first.set()
                first.result(timeout=5)
                second.result(timeout=5)
        self.assertEqual(
            (
                self.root / THEMES / "sequent-ui-admin/login/sequent-login.ftl"
            ).read_text(),
            "Updated realm template",
        )

    def test_hot_pages_load_vite_and_inherit_unported_pages(self):
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
        self.assertFalse((login / "info.ftl").exists())
        self.assertIn("/@vite/client", (login / "template.ftl").read_text())
        # The username-first page renders in React, identity providers included:
        # only the password page keeps the FreeMarker fallback.
        username = (login / "login-username.ftl").read_text()
        self.assertIn('src="/src/main.tsx"', username)
        self.assertIn("window.kcContext.sequent", username)
        self.assertNotIn("sequent-login.ftl", username)

    def test_identity_verification_pages_render_in_react(self):
        # The Scanovate capture only exists in React: its FreeMarker fallback
        # just says that the step needs this theme.
        prepare(self.root, Runtime.BUILT, skip_build=True)
        login = self.root / THEMES / "sequent-ui-admin/login"
        for page in (
            "scanovate-capture.ftl",
            "scanovate-confirmation.ftl",
            "scanovate-error.ftl",
        ):
            text = (login / page).read_text()
            self.assertIn("/assets/app.js", text, page)
            self.assertIn("window.kcContext.sequent", text, page)
            self.assertNotIn("sequent-login.ftl", text, page)

    def test_enrollment_renders_in_react_and_login_mode_keeps_freemarker(self):
        prepare(self.root, Runtime.BUILT, skip_build=True)
        login = self.root / THEMES / "sequent-ui-admin/login"
        register = (login / "register.ftl").read_text()
        self.assertIn("/assets/app.js", register)
        self.assertIn("window.kcContext.sequent", register)
        # The form that signs voters in, CAPTCHA and terms aren't React pages.
        self.assertIn("(formMode!'REGISTRATION') == 'LOGIN'", register)
        self.assertIn("recaptchaRequired??", register)
        self.assertIn("termsAcceptanceRequired??", register)
        self.assertIn('<#include "sequent-register.ftl">', register)
        self.assertNotIn("sequent-login.ftl", register)
        self.assertEqual(
            (login / "sequent-register.ftl").read_text(), "Original registration form"
        )
        for page in (
            "registration-finish.ftl",
            "registration-manual-finish.ftl",
            "registration-rejected-finish.ftl",
            "message-finish.ftl",
        ):
            text = (login / page).read_text()
            self.assertIn("/assets/app.js", text, page)
            self.assertIn("window.kcContext.sequent", text, page)
            self.assertNotIn("<#include", text, page)

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
                "login.ftl",
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

    def test_the_overlay_mounts_every_prepared_theme(self):
        repository = Path(__file__).resolve().parents[2]
        themes = json.loads((repository / keycloak.PACKAGE / "themes.json").read_text())
        overlay = (repository / ".devcontainer" / keycloak.OVERLAY).read_text()
        for theme in themes:
            self.assertIn(f"target: /opt/keycloak/themes/{theme}\n", overlay)

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
