# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Bound the actual CLI subprocess and expose timeouts to the retry policy."""
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import load_test_common as common


class CommandTimeoutTests(unittest.TestCase):
    def test_timeout_is_a_retryable_cli_error(self):
        with patch.object(common.subprocess, "run", side_effect=subprocess.TimeoutExpired("synthetic-cli", 600)):
            with self.assertRaisesRegex(common.StepCliError, "timed out"):
                common.run_step("synthetic-cli", "refresh-token")

    def test_command_has_a_finite_default_deadline(self):
        with patch.object(common.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "Success!")) as run:
            self.assertEqual(common.run_step("synthetic-cli", "refresh-token"), "Success!")
        self.assertEqual(run.call_args.kwargs["timeout"], 600)

    def test_real_child_is_killed_and_reaped_on_timeout(self):
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "step-cli"
            executable.write_text("#!/usr/bin/env python3\nimport time\ntime.sleep(30)\n")
            executable.chmod(0o700)
            with self.assertRaisesRegex(common.StepCliError, "timed out"):
                common.run_step(str(executable), "refresh-token", timeout=0.05)


if __name__ == "__main__":
    unittest.main()
