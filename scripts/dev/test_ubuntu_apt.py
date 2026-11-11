# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Ubuntu CI uses official archives without changing package trust or scope."""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from scripts.dev.ubuntu_apt import configure_sources


class UbuntuAptTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "sources.list.d").mkdir()

    def write(self, path, content):
        target = self.root / path
        target.write_text(content)
        return target

    def test_direct_azure_list_keeps_options_suites_and_components(self):
        source = self.write(
            "sources.list",
            "deb [arch=amd64 signed-by=/keys/ubuntu.gpg] "
            "http://azure.archive.ubuntu.com/ubuntu noble main universe\n"
            "deb-src https://azure.archive.ubuntu.com/ubuntu/ noble-updates main\n",
        )
        configure_sources(self.root)
        self.assertEqual(
            source.read_text(),
            "deb [arch=amd64 signed-by=/keys/ubuntu.gpg] "
            "https://archive.ubuntu.com/ubuntu noble main universe\n"
            "deb-src https://archive.ubuntu.com/ubuntu/ noble-updates main\n",
        )

    def test_deb822_mirror_uris_keep_signing_and_security_suites(self):
        source = self.write(
            "sources.list.d/ubuntu.sources",
            "Types: deb\nURIs: mirror+file:/etc/apt/apt-mirrors.txt\n"
            "Suites: noble noble-updates noble-backports\n"
            "Components: main universe\nSigned-By: /keys/ubuntu.gpg\n\n"
            "Types: deb\nURIs: mirror+file:/etc/apt/apt-security-mirrors.txt\n"
            "Suites: noble-security\nComponents: main universe\n"
            "Signed-By: /keys/ubuntu.gpg\n",
        )
        before = source.read_text()
        configure_sources(self.root)
        self.assertEqual(
            source.read_text(),
            before.replace(
                "mirror+file:/etc/apt/apt-mirrors.txt",
                "https://archive.ubuntu.com/ubuntu",
            ).replace(
                "mirror+file:/etc/apt/apt-security-mirrors.txt",
                "https://security.ubuntu.com/ubuntu",
            ),
        )

    def test_uri_lists_and_continuations_preserve_whitespace(self):
        source = self.write(
            "sources.list.d/ubuntu.sources",
            "URIs: https://azure.archive.ubuntu.com/ubuntu  https://example.org\n"
            "  mirror+file:/etc/apt/apt-security-mirrors.txt\n"
            "Suites: noble\n",
        )
        configure_sources(self.root)
        self.assertEqual(
            source.read_text(),
            "URIs: https://archive.ubuntu.com/ubuntu  https://example.org\n"
            "  https://security.ubuntu.com/ubuntu\nSuites: noble\n",
        )

    def test_unrelated_sources_comments_options_and_mirror_files_unchanged(self):
        contents = {
            "sources.list": (
                "# deb http://azure.archive.ubuntu.com/ubuntu noble main\n"
                "deb https://azure.archive.ubuntu.com/ubuntu-custom noble main\n"
                "deb https://packages.microsoft.com/ubuntu/24.04/prod noble main\n"
            ),
            "sources.list.d/custom.sources": (
                "# URIs: http://azure.archive.ubuntu.com/ubuntu\n"
                "URIs: https://example.org/ubuntu\n"
                "Signed-By: http://azure.archive.ubuntu.com/ubuntu\n"
            ),
            "sources.list.d/ubuntu.sources.save": (
                "URIs: http://azure.archive.ubuntu.com/ubuntu\n"
            ),
            "apt-mirrors.txt": "http://azure.archive.ubuntu.com/ubuntu\n",
        }
        for name, content in contents.items():
            self.write(name, content)
        self.assertEqual(configure_sources(self.root), [])
        for name, content in contents.items():
            self.assertEqual((self.root / name).read_text(), content)

    def test_missing_and_already_official_sources_are_noops(self):
        self.assertEqual(configure_sources(self.root), [])
        source = self.write(
            "sources.list", "deb https://archive.ubuntu.com/ubuntu noble main\n"
        )
        self.assertEqual(configure_sources(self.root), [])
        self.assertEqual(
            source.read_text(), "deb https://archive.ubuntu.com/ubuntu noble main\n"
        )

    def test_command_line_uses_only_the_requested_fixture_and_preserves_mode(self):
        source = self.write(
            "sources.list", "deb http://azure.archive.ubuntu.com/ubuntu noble main\n"
        )
        source.chmod(0o640)
        helper = Path(__file__).with_name("ubuntu_apt.py")
        result = subprocess.run(
            [sys.executable, str(helper), "--apt-root", str(self.root)],
            capture_output=True,
            text=True,
            check=True,
        )
        self.assertIn(str(source), result.stdout)
        self.assertEqual(source.stat().st_mode & 0o777, 0o640)
        self.assertEqual(
            source.read_text(), "deb https://archive.ubuntu.com/ubuntu noble main\n"
        )

    def test_repeated_preparation_is_a_noop(self):
        source = self.write(
            "sources.list", "deb http://azure.archive.ubuntu.com/ubuntu noble main\n"
        )
        self.assertEqual(configure_sources(self.root), [source])
        modified = source.stat().st_mtime_ns
        self.assertEqual(configure_sources(self.root), [])
        self.assertEqual(source.stat().st_mtime_ns, modified)


if __name__ == "__main__":
    unittest.main()
