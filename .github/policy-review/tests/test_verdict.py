# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Verdict parsing. A malformed response must fail loudly, never pass quietly."""

from __future__ import annotations

import json
import unittest

from policy_review.config import Config
from policy_review.verdict import VERDICT_SCHEMA, Verdict, VerdictError, Violation, parse


def violation(**overrides) -> dict:
    base = {
        "policy_id": "10-scope",
        "policy_title": "Repository scope",
        "severity": "blocker",
        "path": "charts/app/Chart.yaml",
        "explanation": "A Helm chart is shared code and does not belong here.",
        "remediation": "Move the chart to the repository that owns shared code.",
    }
    base.update(overrides)
    return base


def payload(**overrides) -> str:
    data = {"verdict": "pass", "summary": "Looks fine.", "violations": []}
    data.update(overrides)
    return json.dumps(data)


class SchemaTests(unittest.TestCase):
    def test_every_object_forbids_extra_properties(self):
        # Structured outputs require this; without it the schema is rejected.
        self.assertFalse(VERDICT_SCHEMA["additionalProperties"])
        item = VERDICT_SCHEMA["properties"]["violations"]["items"]
        self.assertFalse(item["additionalProperties"])

    def test_line_is_optional_but_the_rest_is_required(self):
        item = VERDICT_SCHEMA["properties"]["violations"]["items"]
        self.assertNotIn("line", item["required"])
        for field in ("policy_id", "severity", "path", "explanation"):
            self.assertIn(field, item["required"])


class ParseTests(unittest.TestCase):
    def test_parses_a_clean_pass(self):
        result = parse(payload(summary="No issues found."))
        self.assertTrue(result.passed)
        self.assertEqual(result.summary, "No issues found.")

    def test_parses_violations(self):
        result = parse(
            payload(verdict="violations", summary="One issue.", violations=[violation()])
        )
        self.assertFalse(result.passed)
        self.assertEqual(len(result.violations), 1)
        self.assertEqual(result.violations[0].policy_id, "10-scope")
        self.assertEqual(result.violations[0].severity, "blocker")

    def test_rejects_empty_input(self):
        with self.assertRaises(VerdictError):
            parse("")

    def test_rejects_invalid_json(self):
        with self.assertRaises(VerdictError):
            parse("not json at all")

    def test_rejects_a_non_object_response(self):
        with self.assertRaises(VerdictError):
            parse("[1, 2, 3]")

    def test_rejects_a_violation_missing_a_required_field(self):
        broken = violation()
        del broken["path"]
        with self.assertRaises(VerdictError):
            parse(payload(verdict="violations", violations=[broken]))

    def test_rejects_a_non_object_violation(self):
        with self.assertRaises(VerdictError):
            parse(payload(verdict="violations", violations=["oops"]))

    def test_violations_win_over_a_contradictory_pass_claim(self):
        """A response cannot report breaches and still declare itself a pass."""
        result = parse(
            payload(verdict="pass", summary="All good.", violations=[violation()])
        )
        self.assertFalse(result.passed)
        self.assertIn("take precedence", result.summary)

    def test_an_unknown_severity_is_escalated_to_blocker(self):
        result = parse(
            payload(verdict="violations", violations=[violation(severity="trivial")])
        )
        self.assertEqual(result.violations[0].severity, "blocker")

    def test_normalises_severity_case(self):
        result = parse(
            payload(verdict="violations", violations=[violation(severity="WARNING")])
        )
        self.assertEqual(result.violations[0].severity, "warning")

    def test_accepts_a_numeric_line(self):
        result = parse(payload(verdict="violations", violations=[violation(line=12)]))
        self.assertEqual(result.violations[0].line, 12)

    def test_accepts_a_numeric_string_line(self):
        result = parse(payload(verdict="violations", violations=[violation(line="12")]))
        self.assertEqual(result.violations[0].line, 12)

    def test_discards_a_nonsensical_line(self):
        for bad in (0, -3, "abc", None, True):
            result = parse(
                payload(verdict="violations", violations=[violation(line=bad)])
            )
            self.assertIsNone(result.violations[0].line, f"line={bad!r}")

    def test_supplies_a_summary_when_the_model_omits_one(self):
        self.assertIn("No policy violations", parse(payload(summary="")).summary)

    def test_treats_a_missing_violations_key_as_a_pass(self):
        result = parse(json.dumps({"verdict": "pass", "summary": "Fine."}))
        self.assertTrue(result.passed)

    def test_falls_back_to_the_policy_id_when_the_title_is_missing(self):
        item = violation()
        del item["policy_title"]
        result = parse(payload(verdict="violations", violations=[item]))
        self.assertEqual(result.violations[0].policy_title, "10-scope")

    def test_supplies_remediation_when_the_model_omits_it(self):
        item = violation()
        del item["remediation"]
        result = parse(payload(verdict="violations", violations=[item]))
        self.assertTrue(result.violations[0].remediation)


class BlockingTests(unittest.TestCase):
    def config(self, threshold: str = "blocker") -> Config:
        return Config(
            repository="o/r",
            pr_number=1,
            base_ref="origin/main",
            head_sha="abc",
            event_action="opened",
            fail_on_severity=threshold,
        )

    def verdict(self, *severities: str) -> Verdict:
        return Verdict(
            summary="s",
            violations=[
                Violation(
                    policy_id="p",
                    policy_title="P",
                    severity=severity,
                    path="f",
                    explanation="e",
                    remediation="r",
                )
                for severity in severities
            ],
        )

    def test_only_blockers_block_at_the_default_threshold(self):
        blocking = self.verdict("blocker", "warning", "info").blocking(self.config())
        self.assertEqual([v.severity for v in blocking], ["blocker"])

    def test_a_lower_threshold_blocks_more(self):
        blocking = self.verdict("blocker", "warning", "info").blocking(
            self.config("warning")
        )
        self.assertEqual([v.severity for v in blocking], ["blocker", "warning"])

    def test_warnings_alone_do_not_block_by_default(self):
        self.assertEqual(self.verdict("warning", "info").blocking(self.config()), [])


class ViolationTests(unittest.TestCase):
    def test_location_includes_the_line_when_known(self):
        item = Violation(
            policy_id="p",
            policy_title="P",
            severity="blocker",
            path="src/a.py",
            explanation="e",
            remediation="r",
            line=42,
        )
        self.assertEqual(item.location, "src/a.py:42")

    def test_location_is_just_the_path_without_a_line(self):
        item = Violation(
            policy_id="p",
            policy_title="P",
            severity="blocker",
            path="src/a.py",
            explanation="e",
            remediation="r",
        )
        self.assertEqual(item.location, "src/a.py")

    def test_as_dict_omits_an_absent_line(self):
        item = Violation(
            policy_id="p",
            policy_title="P",
            severity="blocker",
            path="src/a.py",
            explanation="e",
            remediation="r",
        )
        self.assertNotIn("line", item.as_dict())


if __name__ == "__main__":
    unittest.main()
