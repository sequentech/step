# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""End-to-end orchestration, with every outbound call stubbed."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from unittest import mock

from policy_review import runner
from policy_review.config import Config
from policy_review.context import PullRequestContext
from policy_review.model import ModelError
from policy_review.policies import Policy
from policy_review.runner import EXIT_ERROR, EXIT_OK, EXIT_VIOLATIONS

POLICIES = [Policy(policy_id="10-scope", title="Repository scope", body="Only config.")]

VIOLATION = {
    "policy_id": "10-scope",
    "policy_title": "Repository scope",
    "severity": "blocker",
    "path": "charts/app/Chart.yaml",
    "explanation": "A Helm chart does not belong here.",
    "remediation": "Move it to the repository that owns shared code.",
}


def config(**overrides) -> Config:
    defaults = dict(
        repository="example-org/example-repo",
        pr_number=7,
        base_ref="origin/main",
        head_sha="deadbeef",
        event_action="opened",
        github_token="ghs_token_value",
        anthropic_api_key="sk-ant-key-value",
    )
    defaults.update(overrides)
    return Config(**defaults)


def pr_context(**overrides) -> PullRequestContext:
    defaults = dict(
        number=7,
        title="Add a chart",
        body="",
        author="octocat",
        base_ref="main",
        head_sha="deadbeef",
        is_draft=False,
        changed_paths=["charts/app/Chart.yaml"],
        diff="diff --git a/charts/app/Chart.yaml b/charts/app/Chart.yaml\n+name: app\n",
        diff_truncated=False,
        linked_issue=None,
    )
    defaults.update(overrides)
    return PullRequestContext(**defaults)


def verdict_json(*violations: dict) -> str:
    return json.dumps(
        {
            "verdict": "violations" if violations else "pass",
            "summary": "Reviewed.",
            "violations": list(violations),
        }
    )


class RunnerTestCase(unittest.TestCase):
    """Stubs every boundary so `run` can be exercised in isolation."""

    def _patch(self, target: object, name: str, **kwargs):
        patcher = mock.patch.object(target, name, **kwargs)
        stub = patcher.start()
        self.addCleanup(patcher.stop)
        return stub

    def setUp(self):
        self.client = mock.Mock()
        self.client.request_changes.return_value = True

        self.load_policies = self._patch(
            runner.policy_loader, "load_policies", return_value=(POLICIES, "origin/main")
        )
        self._patch(runner, "GitHubClient", return_value=self.client)
        self._patch(runner, "_build_pr_context", return_value=pr_context())
        self.review = self._patch(
            runner.model_api, "review", return_value=verdict_json()
        )
        self.slack_message = self._patch(
            runner.model_api, "slack_message", return_value="alert text"
        )
        self.post_message = self._patch(runner.slack, "post_message")

    def run_with(self, cfg: Config) -> int:
        return runner.run(cfg, Path("/tmp"))


class PassPathTests(RunnerTestCase):
    def test_returns_success_and_comments_when_clean(self):
        self.assertEqual(self.run_with(config()), EXIT_OK)
        self.client.upsert_comment.assert_called_once()
        body = self.client.upsert_comment.call_args[0][1]
        self.assertIn("all policies passed", body)

    def test_does_not_notify_slack_on_a_pass(self):
        self.run_with(config(slack_channel="C1", slack_bot_token="xoxb-token"))
        self.post_message.assert_not_called()

    def test_does_not_request_changes_on_a_pass(self):
        self.run_with(config(event_action="ready_for_review"))
        self.client.request_changes.assert_not_called()


class ViolationPathTests(RunnerTestCase):
    def setUp(self):
        super().setUp()
        self.review.return_value = verdict_json(VIOLATION)

    def test_returns_a_failure_code_for_a_blocking_violation(self):
        self.assertEqual(self.run_with(config()), EXIT_VIOLATIONS)

    def test_comments_with_the_findings(self):
        self.run_with(config())
        body = self.client.upsert_comment.call_args[0][1]
        self.assertIn("charts/app/Chart.yaml", body)
        self.assertIn("Repository scope", body)

    def test_requests_changes_once_the_pull_request_is_ready(self):
        self.run_with(config(event_action="ready_for_review"))
        self.client.request_changes.assert_called_once()

    def test_does_not_request_changes_while_still_a_draft(self):
        self.run_with(config(event_action="synchronize"))
        self.client.request_changes.assert_not_called()

    def test_survives_github_declining_the_review(self):
        self.client.request_changes.return_value = False
        self.assertEqual(
            self.run_with(config(event_action="ready_for_review")), EXIT_VIOLATIONS
        )

    def test_notifies_slack_when_configured(self):
        self.run_with(config(slack_channel="C1", slack_bot_token="xoxb-token"))
        self.post_message.assert_called_once()
        self.assertEqual(self.post_message.call_args.kwargs["text"], "alert text")

    def test_falls_back_to_a_plain_alert_when_generation_fails(self):
        self.slack_message.return_value = ""
        self.run_with(config(slack_channel="C1", slack_bot_token="xoxb-token"))
        text = self.post_message.call_args.kwargs["text"]
        self.assertIn("example-org/example-repo", text)
        self.assertIn("#7", text)

    def test_a_slack_failure_does_not_change_the_verdict(self):
        self.post_message.side_effect = runner.slack.SlackError("channel_not_found")
        self.assertEqual(
            self.run_with(config(slack_channel="C1", slack_bot_token="xoxb-token")),
            EXIT_VIOLATIONS,
        )

    def test_skips_slack_without_a_channel(self):
        self.run_with(config())
        self.post_message.assert_not_called()

    def test_a_non_blocking_violation_still_succeeds(self):
        self.review.return_value = verdict_json({**VIOLATION, "severity": "info"})
        self.assertEqual(self.run_with(config()), EXIT_OK)
        self.client.upsert_comment.assert_called_once()


class FailurePathTests(RunnerTestCase):
    def test_a_model_failure_is_reported_and_never_read_as_a_pass(self):
        self.review.side_effect = ModelError("the model was unreachable")

        self.assertEqual(self.run_with(config()), EXIT_ERROR)

        body = self.client.upsert_comment.call_args[0][1]
        self.assertIn("could not be completed", body)
        self.assertIn("not** a pass", body)

    def test_a_malformed_verdict_is_reported_as_a_failure(self):
        self.review.return_value = "this is not json"
        self.assertEqual(self.run_with(config()), EXIT_ERROR)

    def test_secrets_never_reach_the_failure_comment(self):
        self.review.side_effect = ModelError("auth failed for sk-ant-key-value")

        self.run_with(config())

        body = self.client.upsert_comment.call_args[0][1]
        self.assertNotIn("sk-ant-key-value", body)
        self.assertIn("***", body)


class NoPolicyTests(RunnerTestCase):
    def test_succeeds_quietly_when_no_policies_are_configured(self):
        with mock.patch.object(
            runner.policy_loader, "load_policies", return_value=([], "origin/main")
        ):
            self.assertEqual(self.run_with(config()), EXIT_OK)
        self.client.upsert_comment.assert_not_called()


class EntryPointTests(unittest.TestCase):
    def test_skips_when_no_model_credentials_are_available(self):
        """A fork pull request gets no secrets; skipping is correct."""
        env = {"GITHUB_REPOSITORY": "example-org/example-repo", "POLICY_PR_NUMBER": "7"}
        with mock.patch.dict("os.environ", env, clear=True):
            with mock.patch.object(runner, "run") as run_stub:
                self.assertEqual(runner.main(), EXIT_OK)
        run_stub.assert_not_called()

    def test_fails_when_the_environment_is_unusable(self):
        with mock.patch.dict("os.environ", {}, clear=True):
            self.assertEqual(runner.main(), EXIT_ERROR)

    def test_fails_without_a_github_token(self):
        env = {
            "GITHUB_REPOSITORY": "example-org/example-repo",
            "POLICY_PR_NUMBER": "7",
            "ANTHROPIC_API_KEY": "sk-ant-key-value",
        }
        with mock.patch.dict("os.environ", env, clear=True):
            self.assertEqual(runner.main(), EXIT_ERROR)


if __name__ == "__main__":
    unittest.main()
