# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for the janitor's signing preset module (packages/windmill/
external-bin/janitor/signing_preset.py), which run.py uses to write the
bundle's signing configuration, the SBEI titles in admins.csv and the
tenant's display name. Expectations come from the client's preset file."""

import importlib.util
import json
import unittest

from scripts.dev.scenario.organizations import ROOT

JANITOR = ROOT / "packages/windmill/external-bin/janitor"
PRESET = JANITOR / "templates/COMELEC/signing.json"
CLIENT_TENANT = JANITOR / "templates/COMELEC/tenant.json"

_spec = importlib.util.spec_from_file_location(
    "janitor_signing_preset", JANITOR / "signing_preset.py"
)
signing_preset = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(signing_preset)


def sbei(username, role, labels, trustee_id=""):
    return {
        "username": username,
        "miru_role": role,
        "permission_label": labels,
        "trustee_id": trustee_id,
    }


class JanitorSigningPresetTest(unittest.TestCase):
    def setUp(self):
        self.preset = signing_preset.load(PRESET)

    def test_sbei_rows_carry_their_roles_title_and_trustees_none(self):
        rows = signing_preset.sbei_admin_rows(
            [
                sbei("sbei-madrid-01", "01", ["madrid", "SBEI"]),
                sbei("sbei-madrid-03", "03", ["madrid", "SBEI"]),
                sbei("trustee-madrid-01", "01", ["madrid", "OFOV"], "trustee-1"),
            ],
            self.preset,
        )
        titles = self.preset["sbei_titles"]
        self.assertEqual(
            rows,
            [
                [
                    True,
                    "sbei-madrid-01",
                    "sbei-madrid-01",
                    "SBEI|madrid",
                    "sbei-madrid-01",
                    "sbei",
                    "",
                    titles["01"],
                ],
                [
                    True,
                    "sbei-madrid-03",
                    "sbei-madrid-03",
                    "SBEI|madrid",
                    "sbei-madrid-03",
                    "sbei",
                    "",
                    titles["03"],
                ],
                [
                    True,
                    "trustee-madrid-01",
                    "trustee-madrid-01",
                    "OFOV|madrid",
                    "trustee-madrid-01",
                    "trustee",
                    "trustee-1",
                    "",
                ],
            ],
        )

    def test_a_role_without_a_title_is_warned_about(self):
        with self.assertLogs(level="WARNING") as logs:
            rows = signing_preset.sbei_admin_rows(
                [sbei("sbei-dili-06", "06", ["dili"])], self.preset
            )
        self.assertEqual(rows[0][-1], "")
        self.assertIn("'06'", logs.output[0])

    def test_the_bundle_gets_the_presets_rules_and_checks(self):
        final_json = signing_preset.add_to_bundle({"areas": []}, self.preset)
        self.assertEqual(final_json["signing_rules"], self.preset["signing_rules"])
        self.assertEqual(final_json["signing_checks"], self.preset["signing_checks"])
        self.assertEqual(final_json["areas"], [])

    def test_the_tenant_template_gets_the_display_name_as_json(self):
        client = signing_preset.load(CLIENT_TENANT)
        context = signing_preset.tenant_context(client)
        self.assertEqual(
            json.loads(context["display_name_json"]), client["display_name"]
        )
        quoted = signing_preset.tenant_context({"display_name": 'A "B" C'})
        self.assertEqual(json.loads(quoted["display_name_json"]), 'A "B" C')
        self.assertEqual(signing_preset.tenant_context({}), {})


if __name__ == "__main__":
    unittest.main()
