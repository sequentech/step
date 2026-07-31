# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""The model calls: one to produce the verdict, one to write the Slack alert."""

from __future__ import annotations

import anthropic

from .prompts import SLACK_SYSTEM_PROMPT
from .verdict import VERDICT_SCHEMA

# Comfortably above any realistic verdict, and low enough to stay well inside
# the SDK's non-streaming timeout.
VERDICT_MAX_TOKENS = 16_000
SLACK_MAX_TOKENS = 1_024

# Server-side fallback re-runs a request that the safety classifiers decline on
# another model, inside the same call. This matters here: reviewing diffs that
# touch cryptography or authentication can look adjacent to restricted topics,
# and a refusal would otherwise stall the check.
FALLBACK_BETA = "server-side-fallback-2026-07-01"


class ModelError(RuntimeError):
    """Raised when the model could not produce a usable answer."""


class ModelRefusal(ModelError):
    """Raised when safety classifiers declined the request."""


def _first_text(message) -> str:
    for block in message.content:
        if getattr(block, "type", None) == "text":
            return block.text
    return ""


def _check_stop_reason(message) -> None:
    stop_reason = getattr(message, "stop_reason", None)
    if stop_reason == "refusal":
        details = getattr(message, "stop_details", None)
        category = getattr(details, "category", None) if details else None
        raise ModelRefusal(
            "the model declined to review this pull request"
            + (f" (category: {category})" if category else "")
        )
    if stop_reason == "max_tokens":
        raise ModelError(
            "the model hit the output limit before finishing its verdict; "
            "the diff is likely too large to review in one pass"
        )


def _create(client: anthropic.Anthropic, **kwargs):
    """Send a request, preferring server-side refusal fallbacks.

    Organisations without the fallback beta enabled get a 400 rather than a
    refusal safety net, so the call is retried once without it. Everything else
    is re-raised.
    """
    try:
        return client.beta.messages.create(betas=[FALLBACK_BETA], fallbacks="default", **kwargs)
    except anthropic.BadRequestError as exc:
        detail = str(exc).lower()
        if "fallback" not in detail and "beta" not in detail:
            raise
        return client.messages.create(**kwargs)


def review(
    *,
    api_key: str,
    model: str,
    effort: str,
    system_prompt: str,
    user_prompt: str,
) -> str:
    """Run the policy review and return the raw JSON verdict.

    The system prompt is cached: it holds the policies, which are identical for
    every pull request in a repository and change only when a policy is edited.
    """
    client = anthropic.Anthropic(api_key=api_key)
    try:
        message = _create(
            client,
            model=model,
            max_tokens=VERDICT_MAX_TOKENS,
            system=[
                {
                    "type": "text",
                    "text": system_prompt,
                    "cache_control": {"type": "ephemeral"},
                }
            ],
            output_config={
                "effort": effort,
                "format": {"type": "json_schema", "schema": VERDICT_SCHEMA},
            },
            messages=[{"role": "user", "content": user_prompt}],
        )
    except anthropic.APIStatusError as exc:
        raise ModelError(f"policy review request failed ({exc.status_code}): {exc}") from exc
    except anthropic.APIConnectionError as exc:
        raise ModelError(f"could not reach the model API: {exc}") from exc

    _check_stop_reason(message)
    payload = _first_text(message)
    if not payload.strip():
        raise ModelError("the model returned no verdict text")
    return payload


def slack_message(
    *,
    api_key: str,
    model: str,
    user_prompt: str,
) -> str:
    """Generate the Slack alert body. Returns an empty string on failure.

    A failure here must never fail the check: the PR comment is the primary
    channel and it has already been posted by the time this runs.
    """
    client = anthropic.Anthropic(api_key=api_key)
    try:
        message = _create(
            client,
            model=model,
            max_tokens=SLACK_MAX_TOKENS,
            # Notifying a channel is a summarisation task, not a reasoning one.
            output_config={"effort": "low"},
            system=SLACK_SYSTEM_PROMPT,
            messages=[{"role": "user", "content": user_prompt}],
        )
        if getattr(message, "stop_reason", None) == "refusal":
            return ""
        return _first_text(message).strip()
    except (anthropic.APIError, anthropic.APIConnectionError):
        return ""
