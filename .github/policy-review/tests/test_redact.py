# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Secret scrubbing."""

from __future__ import annotations

import unittest

from policy_review.redact import PLACEHOLDER, redact


class RedactTests(unittest.TestCase):
    def test_replaces_a_secret(self):
        self.assertEqual(
            redact("token is ghp_abcdefghijkl here", ["ghp_abcdefghijkl"]),
            f"token is {PLACEHOLDER} here",
        )

    def test_replaces_every_occurrence(self):
        result = redact("a sk-ant-secret-value b sk-ant-secret-value", ["sk-ant-secret-value"])
        self.assertNotIn("sk-ant-secret-value", result)
        self.assertEqual(result.count(PLACEHOLDER), 2)

    def test_replaces_several_secrets(self):
        result = redact(
            "ghp_aaaaaaaaaa and xoxb_bbbbbbbbbb",
            ["ghp_aaaaaaaaaa", "xoxb_bbbbbbbbbb"],
        )
        self.assertNotIn("ghp_aaaaaaaaaa", result)
        self.assertNotIn("xoxb_bbbbbbbbbb", result)

    def test_ignores_short_values(self):
        # Blanket-replacing a short string would corrupt ordinary prose.
        self.assertEqual(redact("the cat sat", ["cat"]), "the cat sat")

    def test_ignores_empty_secrets(self):
        self.assertEqual(redact("unchanged", ["", None]), "unchanged")

    def test_handles_empty_text(self):
        self.assertEqual(redact("", ["ghp_abcdefghijkl"]), "")

    def test_leaves_text_alone_when_no_secret_appears(self):
        self.assertEqual(redact("all clear", ["ghp_abcdefghijkl"]), "all clear")


if __name__ == "__main__":
    unittest.main()
