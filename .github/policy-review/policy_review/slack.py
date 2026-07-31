# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Posting the violation alert to Slack.

Slack is a notification channel, never a gate: if posting fails the policy
decision still stands and the pull request comment still carries the detail.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request

POST_MESSAGE_URL = "https://slack.com/api/chat.postMessage"

# Slack rejects text over 40k; well before that a wall of text stops being a
# useful alert.
MAX_TEXT = 3_000


class SlackError(RuntimeError):
    """Raised when Slack rejects the message."""


def post_message(
    *,
    token: str,
    channel: str,
    text: str,
    url: str = POST_MESSAGE_URL,
) -> None:
    """Post ``text`` to ``channel``. Raises :class:`SlackError` on failure."""
    if not token:
        raise SlackError("no Slack bot token available")
    if not channel:
        raise SlackError("no Slack channel configured")
    if not text.strip():
        raise SlackError("refusing to post an empty message")

    payload = json.dumps(
        {
            "channel": channel,
            "text": text[:MAX_TEXT],
            # The alert is already a summary; unfurling pull request links on
            # top of it adds noise to the channel.
            "unfurl_links": False,
            "unfurl_media": False,
        }
    ).encode("utf-8")

    request = urllib.request.Request(url=url, data=payload, method="POST")
    request.add_header("Authorization", f"Bearer {token}")
    request.add_header("Content-Type", "application/json; charset=utf-8")

    try:
        with urllib.request.urlopen(request, timeout=30) as raw:
            body = json.loads(raw.read().decode("utf-8") or "{}")
    except urllib.error.HTTPError as exc:
        raise SlackError(f"Slack returned HTTP {exc.code}") from exc
    except urllib.error.URLError as exc:
        raise SlackError(f"could not reach Slack: {exc.reason}") from exc
    except json.JSONDecodeError as exc:
        raise SlackError("Slack returned a malformed response") from exc

    if not body.get("ok"):
        # Slack reports application errors with HTTP 200 and ok=false.
        raise SlackError(f"Slack rejected the message: {body.get('error', 'unknown')}")
