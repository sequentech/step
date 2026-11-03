# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import contextlib
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import load_test_common as common
from setup_telephone_load_test import provision_tenant


class SetupTests(unittest.TestCase):
    def test_command_failure_and_retry_do_not_print_credentials(self):
        # Legacy commands also report errors with an exit code of zero.
        for password, returncode in [("--SYNTHETIC_PASSWORD", 1), ("Error!", 0)]:
            with self.subTest(password=password):
                args = (
                    "config",
                    "--keycloak-password",
                    password,
                    "--keycloak-client-secret=SYNTHETIC_SECRET",
                )
                output = io.StringIO()
                with patch.object(
                    common.subprocess,
                    "run",
                    return_value=subprocess.CompletedProcess(
                        [],
                        returncode,
                        f"Error! failed with argument '{password}'; SYNTHETIC_SECRET",
                    ),
                ), contextlib.redirect_stderr(output):
                    with self.assertRaises(common.StepCliError) as error:
                        common.run_step("step-cli", *args)
                    with self.assertRaises(SystemExit):
                        common.retry_step("step-cli", 1, 0, *args)
                text = str(error.exception) + output.getvalue()
                self.assertIn("config", text)
                self.assertIn("[REDACTED]", text)
                self.assertNotIn(password, text)
                self.assertNotIn("SYNTHETIC_SECRET", text)

    def test_setup_generates_census_from_imported_identities(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = {
                "election_event": {
                    "id": "old-event",
                    "tenant_id": "tenant",
                    "presentation": {"i18n": {"en": {"name": "Test"}}},
                },
                "elections": [{"id": "old-election"}],
                "areas": [{"id": "old-area", "name": "Area"}],
                "contests": [{"id": "old-contest", "election_id": "old-election"}],
                "area_contests": [{"area_id": "old-area", "contest_id": "old-contest"}],
            }
            source = root / "template.json"
            source.write_text(json.dumps(original))
            imported = json.loads(json.dumps(original).replace("old-", "new-"))

            # Stop exactly at census generation: later provisioning must not run.
            class CensusReached(Exception):
                pass

            def run_step(binary, command, *args):
                if command == "import-election":
                    return "Success! ID: new-event"
                if command == "export-election-event":
                    self.assertEqual(
                        args[args.index("--election-event-id") + 1], "new-event"
                    )
                    export = Path(args[args.index("--output-dir") + 1])
                    with zipfile.ZipFile(
                        export / "election_event_export.zip", "w"
                    ) as archive:
                        archive.writestr("event.json", json.dumps(imported))
                    return ""
                self.assertEqual(command, "generate-voters")
                census_input = json.loads(
                    (root / "run/election-event.json").read_text()
                )
                self.assertEqual(census_input["elections"][0]["id"], "new-election")
                self.assertEqual(
                    census_input["area_contests"][0]["area_id"], "new-area"
                )
                raise CensusReached

            with patch.object(common, "run_step", side_effect=run_step), patch.object(
                common, "extract_id", return_value="new-event"
            ), self.assertRaises(CensusReached):
                provision_tenant(
                    step_cli_bin="step-cli",
                    tenant_id="tenant",
                    tenant_out_dir=root / "run",
                    configure_as=None,
                    admin_portal_user="admin",
                    admin_portal_password="unused",
                    election_event_json=source,
                    election_event_alias_suffix="test",
                    voter_area_name=None,
                    num_voters=1,
                    voter_pin_digits=6,
                    voter_username_start=0,
                    threshold=2,
                    ceremony_policy="AUTOMATIC",
                    trustee1_user=None,
                    trustee1_password=None,
                    trustee2_user=None,
                    trustee2_password=None,
                    voting_channel="TELEPHONE",
                    voting_portal_url="http://portal",
                    keycloak_url="http://keycloak",
                    endpoint_url="http://graphql",
                )
