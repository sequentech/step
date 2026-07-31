# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Configuration parsing and severity gating."""

from __future__ import annotations

import unittest
from unittest import mock

from policy_review import config as config_module
from policy_review.config import Config, ConfigError, from_env

MINIMAL_ENV = {
    "GITHUB_REPOSITORY": "example-org/example-repo",
    "POLICY_PR_NUMBER": "42",
}


def env(**overrides: str) -> dict[str, str]:
    merged = dict(MINIMAL_ENV)
    merged.update(overrides)
    return merged


class FromEnvTests(unittest.TestCase):
    def test_reads_the_minimal_environment(self):
        with mock.patch.dict("os.environ", env(), clear=True):
            cfg = from_env()
        self.assertEqual(cfg.repository, "example-org/example-repo")
        self.assertEqual(cfg.pr_number, 42)
        self.assertEqual(cfg.policies_path, config_module.DEFAULT_POLICIES_PATH)
        self.assertEqual(cfg.model, config_module.DEFAULT_MODEL)

    def test_requires_a_repository(self):
        with (
            mock.patch.dict("os.environ", {"POLICY_PR_NUMBER": "1"}, clear=True),
            self.assertRaises(ConfigError),
        ):
            from_env()

    def test_requires_a_pull_request_number(self):
        with (
            mock.patch.dict("os.environ", {"GITHUB_REPOSITORY": "o/r"}, clear=True),
            self.assertRaises(ConfigError),
        ):
            from_env()

    def test_rejects_a_non_numeric_pull_request_number(self):
        with (
            mock.patch.dict("os.environ", env(POLICY_PR_NUMBER="abc"), clear=True),
            self.assertRaises(ConfigError),
        ):
            from_env()

    def test_treats_whitespace_only_values_as_unset(self):
        # GitHub renders an unset workflow input as an empty string, so blank
        # must fall back to the default rather than becoming a literal value.
        with mock.patch.dict("os.environ", env(POLICY_MODEL="   "), clear=True):
            cfg = from_env()
        self.assertEqual(cfg.model, config_module.DEFAULT_MODEL)

    def test_rejects_an_unknown_effort_level(self):
        with (
            mock.patch.dict("os.environ", env(POLICY_EFFORT="turbo"), clear=True),
            self.assertRaises(ConfigError),
        ):
            from_env()

    def test_rejects_an_unknown_severity_threshold(self):
        with (
            mock.patch.dict("os.environ", env(POLICY_FAIL_ON_SEVERITY="nit"), clear=True),
            self.assertRaises(ConfigError),
        ):
            from_env()

    def test_rejects_a_non_positive_diff_budget(self):
        with (
            mock.patch.dict("os.environ", env(POLICY_MAX_DIFF_BYTES="0"), clear=True),
            self.assertRaises(ConfigError),
        ):
            from_env()

    def test_accepts_a_full_environment(self):
        with mock.patch.dict(
            "os.environ",
            env(
                POLICY_POLICIES_PATH=".github/rules",
                POLICY_REPO_TYPE="Config only",
                POLICY_MODEL="claude-sonnet-5",
                POLICY_EFFORT="medium",
                POLICY_MAX_DIFF_BYTES="1000",
                POLICY_FAIL_ON_SEVERITY="warning",
                POLICY_SLACK_CHANNEL="C123",
                POLICY_EVENT_ACTION="ready_for_review",
                SLACK_BOT_TOKEN="xoxb-token-value",
            ),
            clear=True,
        ):
            cfg = from_env()
        self.assertEqual(cfg.policies_path, ".github/rules")
        self.assertEqual(cfg.effort, "medium")
        self.assertEqual(cfg.max_diff_bytes, 1000)
        self.assertEqual(cfg.fail_on_severity, "warning")
        self.assertTrue(cfg.is_ready_for_review)


class SeverityGateTests(unittest.TestCase):
    def gate(self, threshold: str) -> Config:
        return Config(
            repository="o/r",
            pr_number=1,
            base_ref="origin/main",
            head_sha="abc",
            event_action="opened",
            fail_on_severity=threshold,
        )

    def test_blocker_threshold_blocks_only_blockers(self):
        cfg = self.gate("blocker")
        self.assertTrue(cfg.blocks("blocker"))
        self.assertFalse(cfg.blocks("warning"))
        self.assertFalse(cfg.blocks("info"))

    def test_warning_threshold_blocks_warnings_and_above(self):
        cfg = self.gate("warning")
        self.assertTrue(cfg.blocks("blocker"))
        self.assertTrue(cfg.blocks("warning"))
        self.assertFalse(cfg.blocks("info"))

    def test_info_threshold_blocks_everything(self):
        cfg = self.gate("info")
        self.assertTrue(all(cfg.blocks(s) for s in ("blocker", "warning", "info")))

    def test_an_unknown_severity_always_blocks(self):
        # Enforcement must never be weakened by a value we do not recognise.
        self.assertTrue(self.gate("blocker").blocks("cosmetic"))


class SecretInventoryTests(unittest.TestCase):
    def test_collects_only_the_secrets_that_are_set(self):
        cfg = Config(
            repository="o/r",
            pr_number=1,
            base_ref="origin/main",
            head_sha="abc",
            event_action="opened",
            github_token="ghs_token_value",
            anthropic_api_key="",
            slack_bot_token="xoxb_token_value",
        )
        self.assertEqual(set(cfg.secrets()), {"ghs_token_value", "xoxb_token_value"})

    def test_repr_does_not_expose_credentials(self):
        cfg = Config(
            repository="o/r",
            pr_number=1,
            base_ref="origin/main",
            head_sha="abc",
            event_action="opened",
            github_token="ghs_super_secret",
            anthropic_api_key="sk-ant-super-secret",
            slack_bot_token="xoxb-super-secret",
        )
        rendered = repr(cfg)
        self.assertNotIn("ghs_super_secret", rendered)
        self.assertNotIn("sk-ant-super-secret", rendered)
        self.assertNotIn("xoxb-super-secret", rendered)

    def test_slack_needs_both_a_channel_and_a_token(self):
        base = dict(
            repository="o/r",
            pr_number=1,
            base_ref="origin/main",
            head_sha="abc",
            event_action="opened",
        )
        self.assertFalse(Config(**base, slack_channel="C1").slack_enabled)
        self.assertFalse(Config(**base, slack_bot_token="xoxb-1").slack_enabled)
        self.assertTrue(Config(**base, slack_channel="C1", slack_bot_token="xoxb-1").slack_enabled)


if __name__ == "__main__":
    unittest.main()
