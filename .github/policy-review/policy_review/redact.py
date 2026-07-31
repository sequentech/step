# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Last-resort scrubbing of secret values from anything we publish.

Nothing in this engine deliberately writes a credential to a comment, a Slack
message or a log line. This module exists so that a bug elsewhere — an exception
string carrying a URL with an embedded token, say — cannot turn into a leak.
"""

from __future__ import annotations

from collections.abc import Iterable

PLACEHOLDER = "***"

# Below this length a "secret" is more likely to be an empty default or a stray
# character than a real credential, and blanket-replacing it would corrupt
# ordinary text.
MIN_SECRET_LENGTH = 8


def redact(text: str, secrets: Iterable[str]) -> str:
    """Replace every occurrence of each secret in ``text``."""
    if not text:
        return text
    result = text
    for secret in secrets:
        if secret and len(secret) >= MIN_SECRET_LENGTH:
            result = result.replace(secret, PLACEHOLDER)
    return result
