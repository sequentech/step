# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Configuration, assembled from the environment.

Every value the workflow passes in arrives as an environment variable. Nothing
untrusted is ever interpolated into a shell command, so pull request titles,
bodies and diffs reach this process only through ``env:`` blocks.
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field

from .guard import DEFAULT_GUARDED_PATHS

# Ordered from least to most serious. A violation blocks when its severity is at
# or above the configured threshold.
SEVERITIES = ("info", "warning", "blocker")

DEFAULT_MODEL = "claude-opus-5"
DEFAULT_EFFORT = "high"
DEFAULT_POLICIES_PATH = ".github/policies"
DEFAULT_MAX_DIFF_BYTES = 400_000
DEFAULT_FAIL_ON_SEVERITY = "blocker"

DEFAULT_SLACK_MESSAGE_PROMPT = (
    "Write a short Slack alert for the engineering channel about this policy "
    "review result. Open with one sentence naming what happened — a boundary "
    "crossed, the policy review system itself changed, or both. Then list at "
    "most three concrete items as bullets, each naming the file. Close with the "
    "pull request link on its own line. Plain text with Slack mrkdwn only; no "
    "headings, no preamble, no sign-off. Keep it under 900 characters."
)


class ConfigError(RuntimeError):
    """Raised when the environment cannot produce a usable configuration."""


def _clean(name: str, default: str = "") -> str:
    """Read an environment variable, treating whitespace-only as unset."""
    value = os.environ.get(name)
    if value is None:
        return default
    value = value.strip()
    return value if value else default


def _lines(name: str, default: tuple[str, ...]) -> tuple[str, ...]:
    """Read a newline- or comma-separated list, falling back to ``default``."""
    raw = _clean(name)
    if not raw:
        return default
    parts = [
        piece.strip()
        for line in raw.splitlines()
        for piece in line.split(",")
        if piece.strip()
    ]
    return tuple(parts) or default


def _int(name: str, default: int) -> int:
    raw = _clean(name)
    if not raw:
        return default
    try:
        parsed = int(raw)
    except ValueError as exc:
        raise ConfigError(f"{name} must be an integer, got {raw!r}") from exc
    if parsed <= 0:
        raise ConfigError(f"{name} must be positive, got {parsed}")
    return parsed


@dataclass(frozen=True)
class Config:
    """Everything one policy review run needs to know."""

    # What to review
    repository: str
    pr_number: int
    base_ref: str
    head_sha: str
    event_action: str

    # How to review it
    policies_path: str = DEFAULT_POLICIES_PATH
    repo_type: str = ""
    model: str = DEFAULT_MODEL
    effort: str = DEFAULT_EFFORT
    max_diff_bytes: int = DEFAULT_MAX_DIFF_BYTES
    fail_on_severity: str = DEFAULT_FAIL_ON_SEVERITY
    guarded_paths: tuple[str, ...] = DEFAULT_GUARDED_PATHS

    # Where to shout about it
    slack_channel: str = ""
    slack_message_prompt: str = DEFAULT_SLACK_MESSAGE_PROMPT

    # Credentials
    github_token: str = field(default="", repr=False)
    anthropic_api_key: str = field(default="", repr=False)
    slack_bot_token: str = field(default="", repr=False)

    @property
    def is_ready_for_review(self) -> bool:
        """True when this run was triggered by the PR leaving draft state."""
        return self.event_action == "ready_for_review"

    @property
    def slack_enabled(self) -> bool:
        return bool(self.slack_channel and self.slack_bot_token)

    def blocks(self, severity: str) -> bool:
        """Whether a violation of ``severity`` should fail the check."""
        try:
            threshold = SEVERITIES.index(self.fail_on_severity)
        except ValueError:
            threshold = SEVERITIES.index(DEFAULT_FAIL_ON_SEVERITY)
        try:
            actual = SEVERITIES.index(severity)
        except ValueError:
            # An unrecognised severity is treated as the most serious, so a
            # malformed model response can never silently downgrade a finding.
            return True
        return actual >= threshold

    def secrets(self) -> tuple[str, ...]:
        """Secret values that must never appear in output."""
        return tuple(
            value
            for value in (
                self.github_token,
                self.anthropic_api_key,
                self.slack_bot_token,
            )
            if value
        )


def from_env() -> Config:
    """Build a :class:`Config` from the current environment.

    Raises :class:`ConfigError` when a required value is missing or malformed.
    """
    repository = _clean("GITHUB_REPOSITORY")
    if not repository:
        raise ConfigError("GITHUB_REPOSITORY is not set")

    raw_number = _clean("POLICY_PR_NUMBER")
    if not raw_number:
        raise ConfigError("POLICY_PR_NUMBER is not set")
    try:
        pr_number = int(raw_number)
    except ValueError as exc:
        raise ConfigError(
            f"POLICY_PR_NUMBER must be an integer, got {raw_number!r}"
        ) from exc

    effort = _clean("POLICY_EFFORT", DEFAULT_EFFORT)
    if effort not in ("low", "medium", "high", "xhigh", "max"):
        raise ConfigError(f"POLICY_EFFORT is not a valid effort level: {effort!r}")

    fail_on = _clean("POLICY_FAIL_ON_SEVERITY", DEFAULT_FAIL_ON_SEVERITY)
    if fail_on not in SEVERITIES:
        raise ConfigError(
            f"POLICY_FAIL_ON_SEVERITY must be one of {', '.join(SEVERITIES)}, "
            f"got {fail_on!r}"
        )

    return Config(
        repository=repository,
        pr_number=pr_number,
        base_ref=_clean("POLICY_BASE_REF"),
        head_sha=_clean("POLICY_HEAD_SHA"),
        event_action=_clean("POLICY_EVENT_ACTION"),
        policies_path=_clean("POLICY_POLICIES_PATH", DEFAULT_POLICIES_PATH),
        repo_type=_clean("POLICY_REPO_TYPE"),
        model=_clean("POLICY_MODEL", DEFAULT_MODEL),
        effort=effort,
        max_diff_bytes=_int("POLICY_MAX_DIFF_BYTES", DEFAULT_MAX_DIFF_BYTES),
        fail_on_severity=fail_on,
        guarded_paths=_lines("POLICY_GUARDED_PATHS", DEFAULT_GUARDED_PATHS),
        slack_channel=_clean("POLICY_SLACK_CHANNEL"),
        slack_message_prompt=_clean(
            "POLICY_SLACK_MESSAGE_PROMPT", DEFAULT_SLACK_MESSAGE_PROMPT
        ),
        github_token=_clean("GITHUB_TOKEN"),
        anthropic_api_key=_clean("ANTHROPIC_API_KEY"),
        slack_bot_token=_clean("SLACK_BOT_TOKEN"),
    )
