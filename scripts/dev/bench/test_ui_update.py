# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import unittest

from .ui_update import ends_wait


class ProbeEventTest(unittest.TestCase):
    def error(self, cmd, text, target="voting"):
        return {"event": "error", "id": target, "cmd": cmd, "text": text}

    def test_failure_of_the_awaited_wait_ends_it(self):
        self.assertTrue(
            ends_wait(self.error("wait", "m2"), "visible", ["voting"], "m2")
        )
        self.assertTrue(ends_wait(self.error("open", None), "opened", ["voting"], None))

    def test_late_failures_of_abandoned_waits_are_ignored(self):
        # Sample 1 timed out; its wait fails while the revert waits for "gone".
        self.assertFalse(ends_wait(self.error("wait", "m1"), "gone", ["voting"], "m1"))
        self.assertFalse(
            ends_wait(self.error("wait", "m1"), "visible", ["voting"], "m2")
        )
        self.assertFalse(
            ends_wait(self.error("wait", "m2", "admin"), "visible", ["voting"], "m2")
        )


if __name__ == "__main__":
    unittest.main()
