# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import json
import unittest
from replay_profile import compile_profile


class ProfileTests(unittest.TestCase):
    def fixture(self):
        urls = {
            key + "_url": f"http://s3/publication-one/{key}.json?signature=SECRET"
            for key in ("event", "election", "summary", "style")
        }
        requests = [
            ("GET", "http://portal/index.js"),
            (
                "GET",
                "http://kc/realms/test/protocol/openid-connect/auth?state=SECRET&code_challenge=SECRET&client_id=portal&redirect_uri=http%3A%2F%2Fportal%2Flogin&response_type=code",
            ),
            (
                "POST",
                "http://kc/realms/test/login-actions/authenticate?session_code=SECRET",
            ),
            ("GET", "http://portal/login#code=SECRET"),
            ("GET", "http://portal/index.js"),
            ("POST", "http://kc/realms/test/protocol/openid-connect/token"),
            ("POST", "http://graphql/v1/graphql"),
            *[("GET", url) for url in urls.values()],
            ("POST", "http://graphql/v1/graphql"),
        ]
        har = {
            "log": {
                "entries": [
                    dict(
                        startedDateTime=f"2026-09-07T00:00:{i:02d}Z",
                        request={"method": method, "url": url},
                        response={"status": 302 if i == 2 else 200},
                    )
                    for i, (method, url) in enumerate(requests)
                ]
            }
        }
        capture = dict(
            engine="chromium",
            completed=True,
            persistence_verified=True,
            elapsed_ms=12000,
            casts=[{"tenant_id": "tenant", "election_event_id": "event"}],
            publication_files=[{"urls": urls}],
            requests=[
                dict(
                    id=7,
                    url="http://graphql/v1/graphql",
                    operation="GetVoterStatus",
                    query_payload={
                        "operationName": "GetVoterStatus",
                        "query": "query GetVoterStatus { status }",
                        "variables": {},
                    },
                ),
                dict(
                    id=12, url="http://graphql/v1/graphql", operation="InsertCastVote"
                ),
            ],
        )
        return capture, har

    def test_preserves_repeats_order_and_pacing_without_session_values(self):
        capture, har = self.fixture()
        profile = compile_profile(capture, har)
        self.assertNotIn("SECRET", json.dumps(profile))
        self.assertEqual(len(profile["steps"]), 12)
        self.assertEqual(
            [s["offset_ms"] for s in profile["steps"]], list(range(0, 12000, 1000))
        )
        self.assertEqual(
            sum(s.get("url") == "http://portal/index.js" for s in profile["steps"]), 2
        )
        self.assertEqual(sum(s["kind"] == "publication" for s in profile["steps"]), 4)

    def test_rejects_unverified_or_non_chromium_capture(self):
        for override in (
            {"completed": False},
            {"persistence_verified": False},
            {"engine": "other"},
        ):
            capture, har = self.fixture()
            capture.update(override)
            with self.assertRaises(ValueError):
                compile_profile(capture, har)

    def test_new_mutations_and_unbound_publications_fail_closed(self):
        capture, har = self.fixture()
        capture["requests"][0]["operation"] = "UnexpectedMutation"
        with self.assertRaises(ValueError):
            compile_profile(capture, har)
        capture, har = self.fixture()
        har["log"]["entries"][7]["request"][
            "url"
        ] = "http://s3/publication-other/style.json"
        with self.assertRaises(ValueError):
            compile_profile(capture, har)

    def test_missing_or_reordered_auth_is_rejected(self):
        capture, har = self.fixture()
        har["log"]["entries"][2]["startedDateTime"] = "2026-09-07T00:00:30Z"
        with self.assertRaises(ValueError):
            compile_profile(capture, har)
