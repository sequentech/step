# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import json
import unittest
from traffic import inventory, validate_s3_flow


class TrafficTests(unittest.TestCase):
    def test_operation_survives_absent_har_body_and_secrets_are_removed(self):
        result = inventory(
            [
                dict(
                    url="http://graphql:8080/v1/graphql?token=SECRET",
                    method="POST",
                    operation="GetVoterStatus",
                    status=200,
                    timing={"responseEnd": 12},
                )
            ]
        )
        self.assertEqual(result[0]["operation"], "GetVoterStatus")
        self.assertEqual(result[0]["unknown_size_count"], 1)
        self.assertNotIn("SECRET", json.dumps(result))

    def test_old_path_cannot_pass(self):
        self.assertTrue(
            validate_s3_flow(
                [
                    dict(
                        url="http://graphql/v1/graphql",
                        method="POST",
                        operation="GetVoterContext",
                    )
                ]
            )
        )

    def test_s3_path_requires_download_success(self):
        rows = [
            dict(url="http://graphql/v1/graphql", method="POST", operation=op)
            for op in ("GetVoterStatus", "InsertCastVote")
        ]
        rows += [
            dict(
                url=f"http://s3/publication-123/{i}.json?signature=SECRET",
                method="GET",
                status=200,
            )
            for i in ("event", "election", "summary", "style")
        ]
        self.assertEqual(validate_s3_flow(rows), [])
        rows[-1]["status"] = 403
        self.assertTrue(validate_s3_flow(rows))
