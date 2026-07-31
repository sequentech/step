# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""The GitHub client, exercised against a stubbed transport."""

from __future__ import annotations

import unittest
from unittest import mock

from policy_review.github_api import COMMENT_MARKER, GitHubClient, GitHubError, Response


class ClientTestCase(unittest.TestCase):
    def setUp(self):
        self.client = GitHubClient("ghs_token_value", "example-org/example-repo")
        self.calls: list[tuple[str, str, dict | None]] = []

    def stub(self, responses: list[Response]):
        """Patch the transport, recording calls and replaying ``responses``."""
        queue = list(responses)

        def fake(method, path, payload=None):
            self.calls.append((method, path, payload))
            return queue.pop(0) if queue else Response(200, None)

        return mock.patch.object(self.client, "_request", side_effect=fake)


class ConstructionTests(unittest.TestCase):
    def test_refuses_to_build_without_a_token(self):
        with self.assertRaises(GitHubError):
            GitHubClient("", "example-org/example-repo")


class PullRequestTests(ClientTestCase):
    def test_fetches_a_pull_request(self):
        with self.stub([Response(200, {"title": "A change", "number": 7})]):
            data = self.client.get_pull_request(7)
        self.assertEqual(data["title"], "A change")
        self.assertEqual(self.calls[0][:2], ("GET", "/repos/example-org/example-repo/pulls/7"))

    def test_raises_on_an_error_status(self):
        with self.stub([Response(404, {"message": "Not Found"})]), self.assertRaises(GitHubError):
            self.client.get_pull_request(7)


class LinkedIssueTests(ClientTestCase):
    def test_returns_the_issue_when_reachable(self):
        with self.stub([Response(200, {"title": "Track this", "body": "Details"})]):
            issue = self.client.get_issue("example-org", "tracker", 12)
        self.assertEqual(issue["title"], "Track this")

    def test_returns_none_when_the_issue_is_out_of_scope(self):
        """A tracking issue in another repository is a 404, and that is fine."""
        with self.stub([Response(404, {"message": "Not Found"})]):
            self.assertIsNone(self.client.get_issue("example-org", "tracker", 12))

    def test_returns_none_on_a_permission_error(self):
        with self.stub([Response(403, {"message": "Forbidden"})]):
            self.assertIsNone(self.client.get_issue("example-org", "tracker", 12))


class CommentTests(ClientTestCase):
    def test_creates_a_comment_when_none_exists(self):
        with self.stub([Response(200, []), Response(201, {"id": 1})]):
            self.client.upsert_comment(7, "hello")
        self.assertEqual(self.calls[-1][0], "POST")
        self.assertIn("/issues/7/comments", self.calls[-1][1])
        self.assertEqual(self.calls[-1][2], {"body": "hello"})

    def test_updates_the_existing_comment_instead_of_adding_another(self):
        existing = [{"id": 55, "body": f"{COMMENT_MARKER}\nold text", "user": {"type": "Bot"}}]
        with self.stub([Response(200, existing), Response(200, {"id": 55})]):
            self.client.upsert_comment(7, "new text")
        self.assertEqual(self.calls[-1][0], "PATCH")
        self.assertIn("/issues/comments/55", self.calls[-1][1])

    def test_ignores_comments_from_other_authors(self):
        others = [{"id": 1, "body": "a human said something", "user": {"type": "User"}}]
        with self.stub([Response(200, others), Response(201, {"id": 2})]):
            self.client.upsert_comment(7, "text")
        self.assertEqual(self.calls[-1][0], "POST")

    def test_will_not_adopt_a_marker_comment_written_by_a_person(self):
        """Otherwise anyone could pre-empt the report by planting the marker."""
        planted = [
            {
                "id": 99,
                "body": f"{COMMENT_MARKER}\nnothing to see here",
                "user": {"type": "User", "login": "attacker"},
            }
        ]
        with self.stub([Response(200, planted), Response(201, {"id": 100})]):
            self.client.upsert_comment(7, "the real findings")

        # A new comment is posted rather than the planted one being overwritten.
        self.assertEqual(self.calls[-1][0], "POST")
        self.assertEqual(self.calls[-1][2], {"body": "the real findings"})

    def test_tolerates_a_comment_with_no_body(self):
        with self.stub(
            [Response(200, [{"id": 1, "body": None, "user": {"type": "Bot"}}]), Response(201, {})]
        ):
            self.client.upsert_comment(7, "text")
        self.assertEqual(self.calls[-1][0], "POST")

    def test_tolerates_a_comment_with_no_author(self):
        with self.stub([Response(200, [{"id": 1, "body": "x"}]), Response(201, {})]):
            self.client.upsert_comment(7, "text")
        self.assertEqual(self.calls[-1][0], "POST")


class RequestChangesTests(ClientTestCase):
    def test_submits_a_changes_requested_review(self):
        with self.stub([Response(200, {"id": 9})]):
            self.assertTrue(self.client.request_changes(7, "please fix"))
        method, path, payload = self.calls[0]
        self.assertEqual(method, "POST")
        self.assertIn("/pulls/7/reviews", path)
        self.assertEqual(payload["event"], "REQUEST_CHANGES")

    def test_reports_failure_when_github_declines(self):
        # GitHub refuses when the token's identity opened the pull request.
        for status in (403, 422):
            with self.subTest(status=status):
                self.calls.clear()
                with self.stub([Response(status, {"message": "no"})]):
                    self.assertFalse(self.client.request_changes(7, "body"))

    def test_raises_on_an_unexpected_status(self):
        with self.stub([Response(500, {"message": "boom"})]), self.assertRaises(GitHubError):
            self.client.request_changes(7, "body")


if __name__ == "__main__":
    unittest.main()
