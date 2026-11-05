# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for the janitor's approval matrix preset module (packages/windmill/
external-bin/janitor/approval_matrix_preset.py), which run.py uses to write
the bundle's enrollment approval matrix."""

import importlib.util
import unittest

from scripts.dev.scenario.organizations import ROOT

JANITOR = ROOT / "packages/windmill/external-bin/janitor"
PRESETS = {
    "COMELEC": JANITOR / "templates/COMELEC/approvalMatrix.json",
    "association": JANITOR / "templates/association/approvalMatrix.json",
}

_spec = importlib.util.spec_from_file_location(
    "janitor_approval_matrix_preset", JANITOR / "approval_matrix_preset.py"
)
approval_matrix_preset = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(approval_matrix_preset)


class JanitorApprovalMatrixPresetTest(unittest.TestCase):
    def test_the_default_preset_is_the_comelec_matrix(self):
        self.assertEqual(
            JANITOR / approval_matrix_preset.DEFAULT_PATH, PRESETS["COMELEC"]
        )

    def test_the_bundle_carries_the_matrix_as_it_is(self):
        for name, path in PRESETS.items():
            with self.subTest(name):
                matrix = approval_matrix_preset.load(path)
                bundle = approval_matrix_preset.add_to_bundle(
                    {"tenant_id": "tenant"}, matrix
                )
                self.assertEqual(bundle["approval_matrix"], matrix)
                self.assertEqual(bundle["tenant_id"], "tenant")

    def test_presets_never_accept_with_their_last_rule(self):
        for name, path in PRESETS.items():
            with self.subTest(name):
                matrix = approval_matrix_preset.load(path)
                self.assertEqual(
                    set(matrix), {"compared_fields", "rules", "otherwise"}
                )
                self.assertNotEqual(matrix["otherwise"]["decision"], "ACCEPTED")
                self.assertTrue(matrix["otherwise"].get("reason"))

    def test_the_presets_differ_only_in_their_configuration(self):
        comelec = approval_matrix_preset.load(PRESETS["COMELEC"])
        association = approval_matrix_preset.load(PRESETS["association"])
        self.assertEqual(len(comelec["rules"]), 7)
        self.assertEqual(
            association["compared_fields"],
            ["firstName", "lastName", "dateOfBirth"],
        )
        self.assertEqual(
            association["otherwise"], {"decision": "PENDING", "reason": "NO_VOTER"}
        )


if __name__ == "__main__":
    unittest.main()
