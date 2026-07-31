# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""A small GitHub REST client, built on the standard library.

Only the handful of calls this engine needs are implemented. Using ``urllib``
rather than adding an HTTP dependency keeps the supply chain of a workflow that
handles credentials as small as it can reasonably be.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from dataclasses import dataclass

API_ROOT = "https://api.github.com"
USER_AGENT = "sequent-policy-review"

# Identifies the comment this engine owns, so repeated runs update one comment
# instead of burying the discussion under a new one on every push.
COMMENT_MARKER = "<!-- sequent-policy-review -->"


class GitHubError(RuntimeError):
    """Raised when a GitHub API call fails in a way we cannot work around."""


@dataclass(frozen=True)
class Response:
    status: int
    body: object


class GitHubClient:
    """Minimal GitHub REST client scoped to one repository."""

    def __init__(self, token: str, repository: str, api_root: str = API_ROOT) -> None:
        if not token:
            raise GitHubError("no GitHub token available")
        self._token = token
        self.repository = repository
        self._api_root = api_root.rstrip("/")

    def _request(
        self,
        method: str,
        path: str,
        payload: dict | None = None,
    ) -> Response:
        url = path if path.startswith("http") else f"{self._api_root}{path}"
        data = json.dumps(payload).encode("utf-8") if payload is not None else None
        request = urllib.request.Request(url=url, data=data, method=method)
        request.add_header("Authorization", f"Bearer {self._token}")
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("X-GitHub-Api-Version", "2022-11-28")
        request.add_header("User-Agent", USER_AGENT)
        if data is not None:
            request.add_header("Content-Type", "application/json")

        try:
            with urllib.request.urlopen(request, timeout=30) as raw:
                text = raw.read().decode("utf-8")
                return Response(raw.status, json.loads(text) if text else None)
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")
            try:
                parsed = json.loads(detail)
            except json.JSONDecodeError:
                parsed = detail
            return Response(exc.code, parsed)
        except urllib.error.URLError as exc:
            raise GitHubError(f"{method} {path} failed: {exc.reason}") from exc

    def _ok(self, method: str, path: str, payload: dict | None = None) -> object:
        response = self._request(method, path, payload)
        if response.status >= 400:
            raise GitHubError(f"{method} {path} returned {response.status}: {response.body}")
        return response.body

    def get_pull_request(self, number: int) -> dict:
        body = self._ok("GET", f"/repos/{self.repository}/pulls/{number}")
        if not isinstance(body, dict):
            raise GitHubError(f"unexpected pull request payload for #{number}")
        return body

    def get_issue(self, owner: str, repo: str, number: int) -> dict | None:
        """Fetch a referenced issue, or ``None`` when it is not reachable.

        The default workflow token is scoped to the repository it runs in, so a
        tracking issue in another repository is usually a 404. That is expected
        and never fatal: the issue only ever adds background context.
        """
        response = self._request("GET", f"/repos/{owner}/{repo}/issues/{number}")
        if response.status == 200 and isinstance(response.body, dict):
            return response.body
        return None

    def find_own_comment(self, number: int) -> int | None:
        """Locate this engine's existing comment on the pull request.

        Only a comment written by a bot counts. The marker alone is not enough:
        anyone can post a comment containing it, and adopting theirs would let
        them suppress the real report by pre-empting it.
        """
        page = 1
        while page <= 10:
            body = self._ok(
                "GET",
                f"/repos/{self.repository}/issues/{number}/comments"
                f"?per_page=100&page={page}",
            )
            if not isinstance(body, list) or not body:
                return None
            for comment in body:
                author_type = (comment.get("user") or {}).get("type")
                if author_type != "Bot":
                    continue
                if COMMENT_MARKER in (comment.get("body") or ""):
                    return int(comment["id"])
            if len(body) < 100:
                return None
            page += 1
        return None

    def upsert_comment(self, number: int, body: str) -> None:
        """Create the review comment, or update the one already there."""
        existing = self.find_own_comment(number)
        if existing is not None:
            self._ok(
                "PATCH",
                f"/repos/{self.repository}/issues/comments/{existing}",
                {"body": body},
            )
            return
        self._ok(
            "POST",
            f"/repos/{self.repository}/issues/{number}/comments",
            {"body": body},
        )

    def request_changes(self, number: int, body: str) -> bool:
        """Submit a formal "changes requested" review.

        Returns ``False`` when GitHub refuses — most often because the token's
        identity opened the pull request, and nobody may review their own. The
        caller has already posted the findings as a comment, so this is a
        best-effort escalation rather than a required step.
        """
        response = self._request(
            "POST",
            f"/repos/{self.repository}/pulls/{number}/reviews",
            {"body": body, "event": "REQUEST_CHANGES"},
        )
        if response.status < 400:
            return True
        if response.status in (403, 422):
            return False
        raise GitHubError(
            f"could not request changes on #{number}: "
            f"{response.status}: {response.body}"
        )
