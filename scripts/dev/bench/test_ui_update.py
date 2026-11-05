# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from .ui_update import BrowserProbe, ends_wait


class ProbeEventTest(unittest.TestCase):
    def error(self, cmd, text, target="voting"):
        return {"event": "error", "id": target, "cmd": cmd, "text": text}

    def test_failure_of_the_awaited_command_ends_it(self):
        self.assertTrue(ends_wait(self.error("wait", "m2"), "wait", ["voting"], "m2"))
        self.assertTrue(ends_wait(self.error("open", None), "open", ["voting"], None))

    def test_late_failures_of_abandoned_waits_are_ignored(self):
        # Sample 1 timed out; its wait fails while the revert waits on "gone"
        # for the same marker, or while the next sample waits for another one.
        self.assertFalse(ends_wait(self.error("wait", "m1"), "gone", ["voting"], "m1"))
        self.assertFalse(ends_wait(self.error("wait", "m1"), "wait", ["voting"], "m2"))
        self.assertFalse(
            ends_wait(self.error("wait", "m2", "admin"), "wait", ["voting"], "m2")
        )


class ProbeShutdownTest(unittest.TestCase):
    def test_close_sends_command_and_eof_without_killing_the_probe(self):
        # An actual pipe consumer, requiring neither Node nor a browser, models
        # the probe's input lifetime independently of its browser implementation.
        script = (
            "import json, sys; command = json.loads(sys.stdin.readline()); "
            "remaining = sys.stdin.read(); "
            "sys.exit(0 if command == {'cmd': 'close'} and not remaining else 1)"
        )
        spawn = subprocess.Popen

        def child(_command, **kwargs):
            return spawn([sys.executable, "-c", script], **kwargs)

        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            with patch(
                "scripts.dev.bench.ui_update.subprocess.Popen", side_effect=child
            ):
                probe = BrowserProbe(directory, directory / "probe.log")
            wait = probe.process.wait
            # A regression must fail promptly instead of consuming the real 30s
            # browser shutdown allowance before the assertion can report it.
            with patch.object(
                probe.process, "wait", side_effect=lambda **_: wait(timeout=2)
            ):
                probe.close()
            self.assertEqual(probe.process.returncode, 0)
            probe._reader.join(timeout=2)
            self.assertFalse(probe._reader.is_alive())
            assert probe.process.stdout is not None
            probe.process.stdout.close()


if __name__ == "__main__":
    unittest.main()
