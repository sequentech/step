# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Slack posting."""

from __future__ import annotations

import io
import json
import unittest
import urllib.error
from unittest import mock

from policy_review.slack import MAX_TEXT, SlackError, post_message


class FakeResponse(io.BytesIO):
    """Minimal stand-in for the object urlopen returns as a context manager."""

    def __enter__(self):
        return self

    def __exit__(self, *args):
        return False


def ok_response() -> FakeResponse:
    return FakeResponse(json.dumps({"ok": True}).encode("utf-8"))


class ValidationTests(unittest.TestCase):
    def test_requires_a_token(self):
        with self.assertRaises(SlackError):
            post_message(token="", channel="C1", text="hi")

    def test_requires_a_channel(self):
        with self.assertRaises(SlackError):
            post_message(token="xoxb-1", channel="", text="hi")

    def test_refuses_an_empty_message(self):
        with self.assertRaises(SlackError):
            post_message(token="xoxb-1", channel="C1", text="   ")


class PostTests(unittest.TestCase):
    def test_posts_the_message(self):
        with mock.patch("urllib.request.urlopen", return_value=ok_response()) as opened:
            post_message(token="xoxb-token", channel="C123", text="violations found")

        request = opened.call_args[0][0]
        payload = json.loads(request.data.decode("utf-8"))
        self.assertEqual(payload["channel"], "C123")
        self.assertEqual(payload["text"], "violations found")
        self.assertFalse(payload["unfurl_links"])
        self.assertEqual(request.get_header("Authorization"), "Bearer xoxb-token")

    def test_truncates_an_overlong_message(self):
        with mock.patch("urllib.request.urlopen", return_value=ok_response()) as opened:
            post_message(token="xoxb-1", channel="C1", text="x" * (MAX_TEXT + 500))

        payload = json.loads(opened.call_args[0][0].data.decode("utf-8"))
        self.assertEqual(len(payload["text"]), MAX_TEXT)

    def test_raises_when_slack_reports_an_application_error(self):
        # Slack signals these with HTTP 200 and ok=false.
        body = FakeResponse(json.dumps({"ok": False, "error": "channel_not_found"}).encode())
        with (
            mock.patch("urllib.request.urlopen", return_value=body),
            self.assertRaises(SlackError) as caught,
        ):
            post_message(token="xoxb-1", channel="C1", text="hi")
        self.assertIn("channel_not_found", str(caught.exception))

    def test_raises_on_an_http_error(self):
        error = urllib.error.HTTPError("url", 500, "boom", {}, None)
        with mock.patch("urllib.request.urlopen", side_effect=error), self.assertRaises(SlackError):
            post_message(token="xoxb-1", channel="C1", text="hi")

    def test_raises_when_slack_is_unreachable(self):
        error = urllib.error.URLError("no route to host")
        with mock.patch("urllib.request.urlopen", side_effect=error), self.assertRaises(SlackError):
            post_message(token="xoxb-1", channel="C1", text="hi")

    def test_raises_on_a_malformed_response(self):
        with (
            mock.patch("urllib.request.urlopen", return_value=FakeResponse(b"not json")),
            self.assertRaises(SlackError),
        ):
            post_message(token="xoxb-1", channel="C1", text="hi")


if __name__ == "__main__":
    unittest.main()
