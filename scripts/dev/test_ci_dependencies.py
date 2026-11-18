# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""CI verifies native tools without depending on Ubuntu package mirrors."""

import subprocess
import unittest
from unittest.mock import Mock, call

from scripts.dev.ci_dependencies import verify_dependencies


class CiDependenciesTests(unittest.TestCase):
    def setUp(self):
        self.run = Mock(return_value=subprocess.CompletedProcess([], 0))

    def test_native_build_checks_actual_compilers_and_openssl(self):
        verify_dependencies(run=self.run)
        self.assertEqual(
            self.run.call_args_list,
            [
                call(["cc", "--version"], check=True),
                call(["c++", "--version"], check=True),
                call(["make", "--version"], check=True),
                call(["pkg-config", "--modversion", "openssl"], check=True),
            ],
        )

    def test_wasm_adds_the_required_clang_check(self):
        verify_dependencies(clang=True, run=self.run)
        self.run.assert_any_call(["clang", "--version"], check=True)

    def test_postgres_checks_the_same_binaries_as_the_core_fixture(self):
        def run(command, **kwargs):
            return subprocess.CompletedProcess(
                command, 0, "/usr/lib/postgresql/16/bin\n"
            )

        self.run.side_effect = run
        verify_dependencies(postgres=True, run=self.run)
        self.assertEqual(
            self.run.call_args_list[-3:],
            [
                call(
                    ["pg_config", "--bindir"],
                    check=True,
                    capture_output=True,
                    text=True,
                ),
                call(["/usr/lib/postgresql/16/bin/initdb", "--version"], check=True),
                call(["/usr/lib/postgresql/16/bin/pg_ctl", "--version"], check=True),
            ],
        )

    def test_missing_compiler_fails_instead_of_skipping_tests(self):
        self.run.side_effect = FileNotFoundError("cc")
        with self.assertRaises(FileNotFoundError):
            verify_dependencies(run=self.run)
        self.assertEqual(self.run.call_count, 1)

    def test_missing_openssl_fails_instead_of_skipping_tests(self):
        def run(command, **kwargs):
            if command[0] == "pkg-config":
                raise subprocess.CalledProcessError(1, command)

        self.run.side_effect = run
        with self.assertRaises(subprocess.CalledProcessError):
            verify_dependencies(run=self.run)

    def test_empty_postgres_location_is_rejected(self):
        self.run.return_value = subprocess.CompletedProcess([], 0, "\n")
        with self.assertRaisesRegex(ValueError, "absolute PostgreSQL binary"):
            verify_dependencies(postgres=True, run=self.run)

    def test_missing_postgres_binary_fails(self):
        def run(command, **kwargs):
            if command[0].endswith("/initdb"):
                raise FileNotFoundError(command[0])
            return subprocess.CompletedProcess(
                command, 0, "/usr/lib/postgresql/16/bin\n"
            )

        self.run.side_effect = run
        with self.assertRaises(FileNotFoundError):
            verify_dependencies(postgres=True, run=self.run)


if __name__ == "__main__":
    unittest.main()
