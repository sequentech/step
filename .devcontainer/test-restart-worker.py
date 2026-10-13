#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest


class RestartWorkerTest(unittest.TestCase):
    def test_failure_restarts_and_group_shutdown_stops_worker(self):
        script = Path(__file__).with_name("restart-worker.sh")
        with tempfile.TemporaryDirectory() as directory:
            attempts = Path(directory) / "attempts"
            child = """import os, sys, time
from pathlib import Path
path = Path(sys.argv[1])
with path.open("a") as log:
    log.write(str(os.getpid()) + "\\n")
if len(path.read_text().splitlines()) == 1:
    sys.exit(17)
time.sleep(60)
"""
            process = subprocess.Popen(
                ["sh", str(script), sys.executable, "-c", child, str(attempts)],
                start_new_session=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            try:
                deadline = time.monotonic() + 15
                pids = []
                while time.monotonic() < deadline:
                    pids = attempts.read_text().splitlines() if attempts.exists() else []
                    if len(pids) == 2:
                        break
                    time.sleep(0.05)
                self.assertEqual(len(pids), 2, "failed child was not restarted")
                with self.assertRaises(ProcessLookupError):
                    os.kill(int(pids[0]), 0)
                os.kill(int(pids[1]), 0)
                os.killpg(process.pid, signal.SIGTERM)
                self.assertEqual(process.wait(timeout=5), 0)
                with self.assertRaises(ProcessLookupError):
                    os.kill(int(pids[1]), 0)
                self.assertEqual(len(attempts.read_text().splitlines()), 2)
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()


if __name__ == "__main__":
    unittest.main()
