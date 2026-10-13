# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import unittest
from measurements import summarize_sql


class SqlTests(unittest.TestCase):
    def test_all_accepted_statement_formats_are_counted(self):
        for message in (
            "statement: SELECT 1",
            "execute <unnamed>: SELECT 1",
            "duration: 0.274 ms  statement: SELECT 1",
            "duration: 1.234 ms  execute prepared_1: SELECT 1",
        ):
            with self.subTest(message=message):
                self.assertEqual(
                    summarize_sql([{"message": message}], {}),
                    [
                        {
                            "service": "unattributed",
                            "kind": "submitted SQL",
                            "verb": "SELECT",
                            "count": 1,
                        }
                    ],
                )
        # Duration-only records accompany log_statement=all and must not count twice.
        self.assertEqual(
            summarize_sql(
                [{"message": "statement: SELECT 1"}, {"message": "duration: 0.274 ms"}],
                {},
            )[0]["count"],
            1,
        )
