# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Exercise coordinator failure paths without provisioning external services."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import urllib.error
from unittest.mock import patch

import driver


class CoordinatorTests(unittest.TestCase):
    """Partial executions must retain evidence and never hide a worker failure."""

    def test_local_worker_failure_still_generates_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            (directory / "settings.yaml").write_text("{}")
            with patch(
                "driver.runner.node", side_effect=RuntimeError("worker failed")
            ), patch("driver.report") as report:
                with self.assertRaisesRegex(RuntimeError, "worker failed"):
                    driver.run(directory, 2, "local")
                report.assert_called_once_with(directory)

    def test_kubernetes_failure_collects_evidence_without_deleting_claims(self):
        with tempfile.TemporaryDirectory() as tmp, patch.dict(
            os.environ, LOAD_PASSWORD="synthetic"
        ):
            directory = Path(tmp)
            (directory / "inputs").mkdir()
            (directory / "inputs/config.json").write_text("{}")
            settings = {
                "execution": {
                    "storage_class": "shared",
                    "namespace": "load",
                    "image": "worker:test",
                    "storage_size": "1Gi",
                    "wait_timeout": "2m",
                }
            }
            calls = []

            def execute(arguments, **kwargs):
                calls.append(arguments)
                if "wait" in arguments and any(
                    value.startswith("job/") for value in arguments
                ):
                    raise subprocess.CalledProcessError(1, arguments)

            with patch("driver.command", side_effect=execute), patch(
                "driver.subprocess.check_output",
                return_value=json.dumps({"metadata": {"name": "test"}}),
            ):
                with self.assertRaises(subprocess.CalledProcessError):
                    driver.kubernetes(directory, settings, 3)
            self.assertTrue((directory / "job.json").exists())
            self.assertIn("cp", calls[-1])
            self.assertNotIn(
                "delete", [argument for call in calls for argument in call]
            )
            self.assertNotIn(
                "synthetic", [argument for call in calls for argument in call]
            )

    def test_relative_fixture_paths_follow_configuration_location(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            self.assertEqual(
                driver.path_from("fixture.json", base), base / "fixture.json"
            )
            self.assertIsNone(driver.path_from(None, base))

    def test_devcontainer_mount_uses_longest_real_path_prefix(self):
        mounts = [
            {"Destination": "/workspaces", "Source": "/home/user"},
            {"Destination": "/workspaces/project", "Source": "/srv/project"},
        ]
        self.assertEqual(
            driver.mapped_mount(Path("/workspaces/project/runs/test/inputs"), mounts),
            Path("/srv/project/runs/test/inputs"),
        )
        self.assertEqual(
            driver.mapped_mount(Path("/workspaces/project-other/runs"), mounts),
            Path("/home/user/project-other/runs"),
        )
        with self.assertRaises(ValueError):
            driver.mapped_mount(Path("/private/run"), mounts)

    def test_connectivity_probe_accepts_protected_endpoints_but_rejects_outages(self):
        settings = {
            "runtime": {"python": "python3", "k6": "k6"},
            "workload": {"engine": "k6", "request_timeout": "1m500ms"},
            "target": {
                "portal_url": "https://vote.example.org",
                "keycloak_url": "https://auth.example.org",
                "graphql_url": "https://api.example.org/v1/graphql",
                "storage_origins": ["https://ballots.example.org"],
            },
        }
        with patch("driver.shutil.which", return_value="/bin/tool"), patch(
            "driver.urllib.request.urlopen"
        ) as probe:
            probe.side_effect = urllib.error.HTTPError("", 403, "protected", {}, None)
            driver.check(settings)
            self.assertEqual(probe.call_count, 4)
            self.assertEqual(probe.call_args.kwargs["timeout"], 60.5)
            probe.side_effect = urllib.error.HTTPError("", 503, "down", {}, None)
            with self.assertRaisesRegex(ValueError, "unavailable"):
                driver.check(settings)
            probe.side_effect = urllib.error.URLError("DNS")
            with self.assertRaisesRegex(ValueError, "Cannot reach"):
                driver.check(settings)

    def test_bundled_fixture_has_one_contest_and_no_exported_credentials(self):
        fixture = json.loads((driver.HERE / "fixtures/election.json").read_text())
        self.assertEqual(len(fixture["contests"]), 1)
        realm = fixture["keycloak_event_realm"]
        self.assertEqual(realm["users"], [])
        self.assertTrue(all("secret" not in client for client in realm["clients"]))
        self.assertTrue(
            all(
                "siteSecret" not in item.get("config", {})
                for item in realm["authenticatorConfig"]
            )
        )
