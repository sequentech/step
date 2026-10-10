# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Failures after remote creation must leave enough local state to clean up."""
import argparse
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import cleanup_telephone_load_test as cleanup
import load_test_common as common
import setup_telephone_load_test as setup


class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.event = self.directory / "event.json"
        self.event.write_text(json.dumps({"election_event": {"presentation": {"i18n": {"en": {"name": "Synthetic"}}}}}))
        self.cfg = {
            "tenant_id": "bootstrap", "keycloak_client_secret": "synthetic-secret",
            "endpoint_url": "http://synthetic/graphql", "keycloak_url": "http://synthetic/keycloak",
            "admin_portal_user": "synthetic-admin", "admin_portal_password": "synthetic-password",
            "keycloak_client_id": "synthetic-client", "keycloak_admin_user": "synthetic-master",
            "keycloak_admin_password": "synthetic-password", "out_dir": str(self.directory),
            "election_event_json": str(self.event), "new_tenants": 1,
        }
        self.calls = []
        for name, value in [("load_config", {"setup": self.cfg}), ("find_step_cli", "synthetic-cli")]:
            mock = patch.object(common, name, return_value=value)
            mock.start()
            self.addCleanup(mock.stop)

    def cli(self, binary, *args):
        self.calls.append(args)
        return {
            "export-tenant-config": "Success! ID: export-id",
            "create-tenant": "Success! ID: created-tenant",
            "import-election": "Success! ID: imported-event",
        }.get(args[0], "Success!")

    def test_created_tenant_is_indexed_before_its_credentials_can_fail(self):
        with patch.object(common, "run_step", side_effect=self.cli), patch.object(setup, "parse_trustees", return_value=[("trustee", "pk")]), patch.object(common, "lookup_client_secret", side_effect=RuntimeError("injected credential lookup failure")):
            with self.assertRaisesRegex(RuntimeError, "credential lookup"):
                setup.main()
        self.assertEqual(json.loads((self.directory / "tenants.json").read_text()), {
            "tenant_ids": ["created-tenant"],
            "tenants": [{"tenant_id": "created-tenant", "dir": "tenant-created-tenant", "source": "new"}],
        })

    def test_imported_event_is_recorded_before_later_provisioning_fails(self):
        self.cfg["new_tenants"] = 0
        with patch.object(common, "run_step", side_effect=self.cli), patch.object(setup, "area_restricted_election_event", side_effect=RuntimeError("injected voter configuration failure")):
            with self.assertRaisesRegex(RuntimeError, "voter configuration"):
                setup.main()
        index = json.loads((self.directory / "tenants.json").read_text())
        self.assertEqual(index["tenants"], [{"tenant_id": "bootstrap", "dir": "tenant-bootstrap", "source": "existing"}])
        summary = json.loads((self.directory / "tenant-bootstrap" / "summary.json").read_text())
        self.assertEqual(summary["election_event_id"], "imported-event")
        self.calls.clear()
        with patch.object(common, "run_step", side_effect=self.cli), patch.object(cleanup, "parse_args", return_value=argparse.Namespace(events_only=False, new_tenants_only=False)):
            cleanup.main()
        self.assertIn(("delete-election-event", "--election-event-id", "imported-event"), self.calls)
        self.assertFalse(any(args[0] == "delete-tenant" for args in self.calls))

    def test_missing_summary_keeps_new_tenant_cleanup_and_scope_flags(self):
        common.write_json(self.directory / "tenants.json", {"tenants": [
            {"tenant_id": "created-tenant", "dir": "tenant-created-tenant", "source": "new"},
            {"tenant_id": "reused-tenant", "dir": "tenant-reused-tenant", "source": "existing"},
            {"tenant_id": "bootstrap", "dir": "tenant-bootstrap", "source": "existing"},
        ]})
        for events_only, new_only, expected in [(False, True, ["created-tenant"]), (True, False, []), (False, False, ["created-tenant", "reused-tenant"])]:
            with self.subTest(events_only=events_only, new_only=new_only):
                self.calls.clear()
                with patch.object(common, "run_step", side_effect=self.cli), patch.object(cleanup, "parse_args", return_value=argparse.Namespace(events_only=events_only, new_tenants_only=new_only)):
                    cleanup.main()
                self.assertEqual([args[2] for args in self.calls if args[0] == "delete-tenant"], expected)
                self.assertFalse(any(args[0] == "delete-election-event" for args in self.calls))


if __name__ == "__main__":
    unittest.main()
