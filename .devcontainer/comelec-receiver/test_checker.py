# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequent.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Receiver configuration contract; never contact or start a service."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

CHECKER = Path(__file__).with_name("check-compose.py")
P = "WINDMILL_WEBAUTHN_PACKAGES"
TENANT = "11111111-1111-4111-8111-111111111111"
BASE = f"""SUPER_ADMIN_TENANT_ID={TENANT}
SEQUENT_RECEIVER_RP_ID=architect.ems.smartbox.io
SEQUENT_RECEIVER_ORIGIN=https://architect.ems.smartbox.io
"""
DEPRECATED = "SEQUENT_RECEIVER_ADMIN_SUBJECT=22222222-2222-4222-8222-222222222222\n"


class CheckerTest(unittest.TestCase):
    def check(self, site, **ambient):
        env = {
            k: v for k, v in os.environ.items()
            if k != "SUPER_ADMIN_TENANT_ID"
            and not k.startswith((P, "SEQUENT_RECEIVER_", "ARCHITECT_ROSTER_ACTIVATION_"))
        }
        with tempfile.TemporaryDirectory() as d:
            f = Path(d).resolve() / "site.env"
            f.touch(mode=0o600)
            f.write_text(site, encoding="ascii")
            return subprocess.run(
                [sys.executable, CHECKER, "--site", f],
                env=env | ambient, text=True, capture_output=True, check=False,
            )

    def test_legacy_values_do_not_affect_rendered_authority(self):
        normal = self.check(BASE)
        legacy = self.check(BASE + DEPRECATED, **{P + "_ROSTER_ADMIN_SUBJECTS": "ignored"})
        self.assertEqual(normal.returncode, 0, normal.stderr)
        self.assertEqual(legacy.returncode, 0, legacy.stderr)
        self.assertEqual(json.loads(normal.stdout), json.loads(legacy.stdout))
        for target in json.loads(normal.stdout)["services"].values():
            env = target["environment"]
            self.assertNotIn(P + "_ROSTER_ADMIN_SUBJECTS", env)
            self.assertEqual(env[P + "_TENANT_ID"], TENANT)
            self.assertEqual(env[P + "_REQUIRED_APPROVALS"], "2")

    def test_tenant_is_still_required_and_policy_conflicts_stop(self):
        for site, ambient in (
            (BASE.partition("\n")[2], {}),
            (BASE, {P: "site-pinned"}),
            (BASE, {"SEQUENT_RECEIVER_ORIGIN": "https://wrong.example"}),
        ):
            self.assertNotEqual(self.check(site, **ambient).returncode, 0)


if __name__ == "__main__":
    unittest.main()
