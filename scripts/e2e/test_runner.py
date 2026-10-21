# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Driver outcome contracts; no containers or external services are required."""

import contextlib
import csv
import importlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

with patch.dict(
    os.environ,
    {
        "SUPER_ADMIN_TENANT_ID": "11111111-1111-4111-8111-111111111111",
        "HASURA_ENDPOINT": "https://graphql.invalid",
        "KEYCLOAK_URL": "https://identity.invalid",
        "AWS_S3_PRIVATE_URI": "https://storage.invalid",
        "B4_URL": "https://board.invalid",
    },
):
    driver = importlib.import_module("scripts.e2e.journeys.__main__")
    bootstrap = importlib.import_module("scripts.e2e.journeys.bootstrap")
summary = importlib.import_module("scripts.e2e.summary")
coverage = importlib.import_module("scripts.e2e.coverage")
ROOT = Path(__file__).resolve().parents[2]


class DriverOutput(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.output = Path(temporary.name)
        self.enterContext(patch.object(driver, "OUTPUT", self.output))
        self.enterContext(
            patch("socket.socket", side_effect=AssertionError("Network forbidden"))
        )
        self.enterContext(contextlib.redirect_stdout(io.StringIO()))
        self.enterContext(contextlib.redirect_stderr(io.StringIO()))

    def run_case(self, case, pattern=None):
        with patch.object(driver.test_journeys, "BackendJourneys", case):
            return driver.run_tests(pattern)


class RunnerContracts(DriverOutput):
    def test_browser_run_covers_both_ceremonies_voting_and_tally(self):
        browser_driver = importlib.import_module("scripts.e2e.journeys.browser_journey")
        observed = []

        class Journeys(unittest.TestCase):
            def test_b_manual_key_ceremony_and_tally(self):
                observed.append("manual")

            def test_4_automatic_key_ceremony(self):
                observed.append("automatic")

            def test_6_online_voting_rules(self):
                observed.append("voting")

            def test_7_tally_matches_cast_ballots(self):
                observed.append("tally")

            def test_a_combined_contest_weighted_voting(self):
                observed.append("weighted")

            def test_c_trustee_graphql_board_api(self):
                observed.append("trustee API")

            def test_8_native_archive_preserves_crypto_and_results(self):
                self.fail("Archive recovery is outside this ceremony run")

        with (
            patch.object(browser_driver, "BackendJourneys", Journeys),
            patch.object(browser_driver, "OUTPUT", self.output),
            patch.dict(
                browser_driver.ENV,
                {"STEP_E2E_BROWSER_TRUSTEES_URL": "https://browser.invalid"},
            ),
        ):
            self.assertEqual(browser_driver.main(), 0)
        self.assertEqual(
            observed,
            ["manual", "automatic", "voting", "tally", "weighted", "trustee API"],
        )
        records = json.loads((self.output / "browser-journey.json").read_text())
        self.assertEqual([r["outcome"] for r in records], ["pass"] * 6)

    def test_browser_run_cannot_pass_when_manual_ceremony_is_skipped(self):
        browser_driver = importlib.import_module("scripts.e2e.journeys.browser_journey")

        class Journeys(unittest.TestCase):
            @unittest.skip("browser ceremony prerequisite unavailable")
            def test_b_manual_key_ceremony_and_tally(self):
                pass

            def test_a_combined_contest_weighted_voting(self):
                pass

            def test_c_trustee_graphql_board_api(self):
                pass

        with (
            patch.object(browser_driver, "BackendJourneys", Journeys),
            patch.object(browser_driver, "OUTPUT", self.output),
            patch.dict(
                browser_driver.ENV,
                {"STEP_E2E_BROWSER_TRUSTEES_URL": "https://browser.invalid"},
            ),
        ):
            self.assertEqual(browser_driver.main(), 1)

    def test_browser_run_cannot_pass_when_weighted_voting_is_skipped(self):
        browser_driver = importlib.import_module("scripts.e2e.journeys.browser_journey")

        class Journeys(unittest.TestCase):
            def test_b_manual_key_ceremony_and_tally(self):
                pass

            @unittest.skip("weighted voting prerequisite unavailable")
            def test_a_combined_contest_weighted_voting(self):
                pass

            def test_c_trustee_graphql_board_api(self):
                pass

        with (
            patch.object(browser_driver, "BackendJourneys", Journeys),
            patch.object(browser_driver, "OUTPUT", self.output),
            patch.dict(
                browser_driver.ENV,
                {"STEP_E2E_BROWSER_TRUSTEES_URL": "https://browser.invalid"},
            ),
        ):
            self.assertEqual(browser_driver.main(), 1)

    def test_browser_run_cannot_pass_when_trustee_api_is_skipped(self):
        browser_driver = importlib.import_module("scripts.e2e.journeys.browser_journey")

        class Journeys(unittest.TestCase):
            def test_b_manual_key_ceremony_and_tally(self):
                pass

            def test_a_combined_contest_weighted_voting(self):
                pass

            @unittest.skip("public trustee API prerequisite unavailable")
            def test_c_trustee_graphql_board_api(self):
                pass

        with (
            patch.object(browser_driver, "BackendJourneys", Journeys),
            patch.object(browser_driver, "OUTPUT", self.output),
            patch.dict(
                browser_driver.ENV,
                {"STEP_E2E_BROWSER_TRUSTEES_URL": "https://browser.invalid"},
            ),
        ):
            self.assertEqual(browser_driver.main(), 1)

    def test_filter_runs_prerequisites_and_records_only_the_selected_sequence(self):
        observed = []

        class Journeys(unittest.TestCase):
            def test_01_bootstrap(self):
                observed.append("bootstrap")

            def test_02_cast(self):
                observed.append("cast")

            def test_03_tally(self):
                observed.append("tally")

        self.assertEqual(self.run_case(Journeys, "cast"), 0)
        self.assertEqual(observed, ["bootstrap", "cast"])
        records = json.loads((self.output / "journeys.json").read_text())
        self.assertEqual(
            [record["test"] for record in records],
            ["test_01_bootstrap", "test_02_cast"],
        )
        self.assertEqual([record["outcome"] for record in records], ["pass", "pass"])
        self.assertEqual(self.run_case(Journeys, "not-a-journey"), 2)
        self.assertEqual(observed, ["bootstrap", "cast"])

    def test_empty_discovery_cannot_report_a_successful_run(self):
        class Valid(unittest.TestCase):
            def test_journey(self):
                pass

        class Empty(unittest.TestCase):
            pass

        self.assertEqual(self.run_case(Valid), 0)
        (self.output / "journeys.json").unlink()
        self.assertEqual(self.run_case(Empty), 2)
        self.assertFalse((self.output / "journeys.json").exists())

    def test_skipped_and_expected_failures_are_not_green_journeys(self):
        class Valid(unittest.TestCase):
            def test_journey(self):
                pass

        class Skipped(unittest.TestCase):
            @unittest.skip("missing prerequisite")
            def test_journey(self):
                pass

        class ExpectedFailure(unittest.TestCase):
            @unittest.expectedFailure
            def test_journey(self):
                self.fail("known defect")

        self.assertEqual(self.run_case(Valid), 0)
        for case, outcome in [(Skipped, "skip"), (ExpectedFailure, "expected failure")]:
            with self.subTest(outcome=outcome):
                self.assertEqual(self.run_case(case), 1)
                records = json.loads((self.output / "journeys.json").read_text())
                self.assertEqual([record["outcome"] for record in records], [outcome])


class SummaryContracts(DriverOutput):
    def test_summary_tables_every_recorded_journey_and_check_with_totals(self):
        class Journeys(unittest.TestCase):
            def test_1_import(self):
                pass

            def test_2_tally(self):
                self.fail("expected 3 | got 2\nsecond line")

        self.assertEqual(self.run_case(Journeys), 1)
        (self.output / "bootstrap.json").write_text(
            json.dumps({"seconds": {"tenant row": 2.0, "trustees registered": 9.5}})
        )
        (self.output / "coverage").mkdir()
        (self.output / "coverage/summary.md").write_text("### Coverage table\n")
        text = summary.render(self.output)
        self.assertIn("| bootstrap | pass | 9.5 s | |", text)
        self.assertRegex(text, r"\| `test_1_import` \| pass \| \d+\.\d s \|")
        self.assertRegex(
            text, r"\| `test_2_tally` \| \*\*fail\*\* \| .* expected 3 \\\| got 2 \|"
        )
        self.assertNotIn("second line", text)
        self.assertIn("**Failed:** 2 journeys (1 pass, 1 fail)", text)
        self.assertTrue(text.rstrip().endswith("### Coverage table"))

        self.assertEqual(self.run_case(Journeys, "import"), 0)
        self.assertIn("**Passed:** 1 journeys (1 pass)", summary.render(self.output))

    def test_summary_explains_a_run_without_results(self):
        for directory in (self.output, self.output / "never-created"):
            with self.subTest(directory=directory.name):
                text = summary.render(directory)
                self.assertIn("No journey results", text)
                self.assertNotIn("| Journey", text)
        with patch("sys.argv", ["summary.py", str(self.output)]):
            summary.main()
        self.assertIn("No journey results", (self.output / "summary.md").read_text())

    def test_ci_summarizes_and_uploads_results_after_every_outcome(self):
        workflow = (ROOT / ".github/workflows/backend-e2e.yml").read_text()
        step = workflow[workflow.index("name: Summarize the journey results") :]
        step = step[: step.index("- name:")]
        self.assertIn("if: always()", step)
        command = "python3 scripts/e2e/summary.py .cache/backend-e2e/run"
        self.assertIn(f'{command} >> "$GITHUB_STEP_SUMMARY"', step)
        upload = workflow[workflow.index("name: Upload the results summary") :]
        self.assertIn("hashFiles('.cache/backend-e2e/run/summary.md') != ''", upload)
        for name in ("summary.md", "journeys.json", "coverage/summary.json"):
            self.assertIn(f".cache/backend-e2e/run/{name}", upload)


class FakeKeycloak:
    def __init__(self, credentials):
        self.credentials = credentials
        self.requests = []

    def user(self, realm, username):
        return {"id": "admin-id", "username": username}

    def admin(self, method, path, json_body=None, expect=(200,)):
        self.requests.append((method, path, json_body))
        return self.credentials if method == "GET" else None


class BootstrapAdminContracts(unittest.TestCase):
    """The tenant template ships its administrator without a password."""

    def setUp(self):
        self.enterContext(
            patch.dict(
                os.environ,
                {"ADMIN_USERNAME": "admin", "ADMIN_PASSWORD": "synthetic-password",
                 "API_KEY_CLIENT_SECRET": "synthetic-fixture-secret"},
            )
        )
        self.reset = (
            "PUT",
            f"{bootstrap.TENANT_REALM}/users/admin-id/reset-password",
            {"type": "password", "value": "synthetic-password", "temporary": False},
        )

    def test_enrollment_completes_required_names_when_email_already_exists(self):
        keycloak = MagicMock()
        keycloak.user.return_value = {
            "id": "trustee-id",
            "email": "trustee@example.invalid",
            "firstName": "Trustee",
        }
        credentials = iter(
            [[{"type": "password"}], [{"type": "password"}, {"type": "message-otp"}]]
        )
        keycloak.admin.side_effect = lambda method, *args, **kwargs: (
            next(credentials) if method == "GET" else None
        )
        config = {
            "id": "otp-id",
            "config": {"test-mode": "false", "test-mode-code": "123456"},
        }
        with patch.object(bootstrap, "email_otp_config", return_value=config):
            self.assertTrue(
                bootstrap.enroll_mfa(keycloak, "trustee1", "synthetic-password")
            )
        profile = keycloak.admin.call_args_list[1].args[2]
        self.assertEqual(
            profile,
            {
                "id": "trustee-id",
                "email": "trustee@example.invalid",
                "emailVerified": True,
                "firstName": "Trustee",
                "lastName": "E2E",
            },
        )
        keycloak.browser_login.assert_called_once_with(
            bootstrap.TENANT_REALM,
            "admin-portal",
            "http://127.0.0.1:3002/",
            "trustee1",
            "synthetic-password",
            otp="123456",
        )
        self.assertEqual(config["config"]["test-mode"], "false")

    def test_trustees_complete_enrollment_before_password_grants(self):
        keycloak = MagicMock()
        with patch.dict(
            os.environ,
            {
                "TRUSTEE1_PASSWORD": "trustee-one",
                "TRUSTEE2_PASSWORD": "trustee-two",
                "TRUSTEE_KEYCLOAK_CLIENT_ID": "trustee-client",
                "TRUSTEE_KEYCLOAK_CLIENT_SECRET": "synthetic-secret",
            },
        ), patch.object(bootstrap, "enroll_mfa") as enroll, patch.object(
            bootstrap, "set_password"
        ) as passwords:
            bootstrap.enroll_trustees(keycloak)
        self.assertEqual(
            passwords.call_args_list,
            [
                unittest.mock.call(keycloak, "trustee1", "trustee-one"),
                unittest.mock.call(keycloak, "trustee2", "trustee-two"),
            ],
        )
        self.assertEqual(
            enroll.call_args_list,
            [
                unittest.mock.call(keycloak, "trustee1", "trustee-one"),
                unittest.mock.call(keycloak, "trustee2", "trustee-two"),
            ],
        )
        self.assertEqual(
            keycloak.password_grant.call_args_list,
            [
                unittest.mock.call(
                    bootstrap.TENANT_REALM,
                    "trustee-client",
                    "trustee1",
                    "trustee-one",
                    "synthetic-secret",
                ),
                unittest.mock.call(
                    bootstrap.TENANT_REALM,
                    "trustee-client",
                    "trustee2",
                    "trustee-two",
                    "synthetic-secret",
                ),
            ],
        )

    def test_explicit_fixture_opt_in_keeps_generated_secret_by_default(self):
        keycloak = MagicMock()
        keycloak.admin.side_effect = [[{"id": "api-id", "enabled": False}], None]
        self.assertTrue(bootstrap.enable_test_api_key_client(keycloak, "tenant-fixture"))
        keycloak.admin.assert_called_with("PUT", "tenant-fixture/clients/api-id", {"enabled": True}, expect=(204,))

    def test_super_tenant_fixture_can_set_its_explicit_synthetic_secret(self):
        keycloak = MagicMock()
        keycloak.admin.side_effect = [[{"id": "api-id", "enabled": False}], None]
        bootstrap.enable_test_api_key_client(keycloak, "tenant-fixture", fixture_secret="synthetic")
        keycloak.admin.assert_called_with("PUT", "tenant-fixture/clients/api-id", {"enabled": True, "secret": "synthetic"}, expect=(204,))

    def test_repeated_fixture_opt_in_never_resets_an_enabled_client(self):
        keycloak = MagicMock()
        keycloak.admin.return_value = [{"id": "api-id", "enabled": True}]
        self.assertFalse(bootstrap.enable_test_api_key_client(keycloak, fixture_secret="synthetic"))
        self.assertEqual(keycloak.admin.call_count, 1)

    def test_an_administrator_without_a_password_gets_the_configured_one(self):
        keycloak = FakeKeycloak([])
        bootstrap.set_admin_password(keycloak)
        self.assertIn(self.reset, keycloak.requests)

    def test_an_existing_password_is_kept(self):
        keycloak = FakeKeycloak([{"type": "password"}])
        bootstrap.set_admin_password(keycloak)
        self.assertNotIn(self.reset, keycloak.requests)

    def test_the_password_is_set_before_the_administrator_first_signs_in(self):
        calls = []
        with tempfile.TemporaryDirectory() as directory, patch.multiple(
            bootstrap,
            OUTPUT=Path(directory),
            Keycloak=MagicMock(),
            wait_until=MagicMock(),
            StepCli=MagicMock(),
            configure_step_cli=MagicMock(),
            seed_trustees=MagicMock(),
            enroll_trustees=MagicMock(),
            enable_test_api_key_client=MagicMock(),
            set_admin_password=MagicMock(
                side_effect=lambda keycloak: calls.append("password")
            ),
            enroll_admin_mfa=MagicMock(
                side_effect=lambda keycloak: calls.append("enroll")
            ),
        ), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(bootstrap.main(), 0)
        self.assertEqual(calls, ["password", "enroll"])


class CoveragePackageContracts(unittest.TestCase):
    def test_only_packages_of_the_checkout_are_measured(self):
        # Generic code of a dependency, compiled into an instrumented binary,
        # names files in the cargo registry: those are not Step's packages.
        with tempfile.TemporaryDirectory() as temporary:
            crate = Path(temporary) / "registry/src/index.crates.io-0/tracing-0.1.44"
            (crate / "src").mkdir(parents=True)
            (crate / "Cargo.toml").write_text('[package]\nname = "tracing"\n')
            (crate / "src/instrument.rs").write_text("")
            windmill = coverage.ROOT / "packages/windmill"
            payload = {"data": [{"files": [
                {"filename": str(crate / "src/instrument.rs")},
                {"filename": str(windmill / "src/lib.rs")},
            ]}]}
            self.assertEqual(
                coverage.package_directories(payload), {windmill: "windmill"}
            )


class CoverageExitContracts(unittest.TestCase):
    def report(self, codes, *, json_lines=False):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "profiles").mkdir()
            binaries = directory / "bin"
            binaries.mkdir()
            binary = binaries / "harvest"
            binary.write_text("instrumented binary placeholder")
            binary.chmod(0o700)
            for name in ("harvest", "windmill", "beat", "b4v6", "trustee1", "trustee2",
                         "step-cli", "messaging-webhooks"):
                (directory / "profiles" / f"{name}-123.profraw").write_bytes(b"profile")
            rows = [{"Service": name, "ExitCode": code} for name, code in codes.items()]
            (directory / "services.json").write_text(
                "\n".join(json.dumps(row) for row in rows)
                if json_lines
                else json.dumps(rows)
            )

            def llvm(*command):
                if command[1] == "show":
                    return "Total functions: 12\n"
                if command[1] == "export":
                    return '{"data":[{"files":[]}]}'
                self.assertEqual(command[1], "merge")
                return ""

            with patch.object(
                coverage, "llvm_tool", side_effect=lambda name: name
            ), patch.object(coverage, "run", side_effect=llvm), patch(
                "sys.argv", ["coverage.py", str(directory), str(binaries)]
            ), contextlib.redirect_stdout(
                io.StringIO()
            ):
                status = coverage.main()
            return (
                status,
                json.loads((directory / "summary.json").read_text()),
                (directory / "summary.md").read_text(),
            )

    @staticmethod
    def normal_codes():
        return {"harvest": 0, "windmill": 0,
                "beat": 143, "b4v6": 143, "trustee1": 143, "trustee2": 143,
                "messaging-webhooks": 0}

    def test_normal_shutdown_and_removed_driver_accept_nonempty_profiles(self):
        for json_lines in (False, True):
            with self.subTest(json_lines=json_lines):
                status, report, _ = self.report(
                    self.normal_codes(), json_lines=json_lines
                )
                self.assertEqual(status, 0)
                self.assertEqual(report["failures"], [])
                self.assertEqual(len(report["profiles"]), 8)
                self.assertTrue(all(row["executed_functions"] == 12 for row in report["profiles"]))

    def test_worker_crash_fails_even_when_its_profile_executed_code(self):
        for service, code in (("windmill", 101), ("harvest", 143), ("beat", 137),
                              ("b4v6", 1), ("trustee1", 101)):
            with self.subTest(service=service, code=code):
                codes = self.normal_codes()
                codes[service] = code
                status, report, markdown = self.report(codes)
                self.assertEqual(status, 1)
                self.assertEqual(len(report["failures"]), 1)
                self.assertIn(service, report["failures"][0])
                self.assertIn(str(code), report["failures"][0])
                self.assertIn("**Coverage failure:**", markdown)

    def test_missing_persistent_service_status_cannot_report_success(self):
        for absent in (True, False):
            with self.subTest(absent=absent):
                codes = self.normal_codes()
                if absent:
                    del codes["windmill"]
                else:
                    codes["windmill"] = None
                status, report, _ = self.report(codes)
                self.assertEqual(status, 1)
                self.assertEqual(len(report["failures"]), 1)
                self.assertIn("windmill", report["failures"][0])


class CombinedBallotInputContracts(unittest.TestCase):
    def test_weighted_census_uses_the_platform_import_column(self):
        from scripts.e2e.journeys import fixtures

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "voters.csv"
            voters = fixtures.census("weighted", {"A": 2})
            fixtures.write_census(
                path,
                voters,
                {"A": "main-election"},
                weights={voters[0].username: 3, voters[1].username: 5},
            )
            with path.open(newline="") as handle:
                rows = list(csv.DictReader(handle))
            self.assertEqual(
                [
                    (r["username"], r["vote-weight"], r["authorized-election-ids"])
                    for r in rows
                ],
                [
                    ("e2e-weighted-a1", "3", "main-election"),
                    ("e2e-weighted-a2", "5", "main-election"),
                ],
            )

    def test_each_contest_uses_its_own_selection_in_the_native_encoder(self):
        from scripts.e2e.journeys.voting import Portal

        with tempfile.TemporaryDirectory() as directory:
            cli = MagicMock(directory=Path(directory), binary=Path("step-cli"))
            with patch.dict(
                os.environ, {"VOTING_PORTAL_URL": "https://voting.invalid"}
            ):
                portal = Portal("event", cli)
            style = {
                "contests": [
                    {
                        "id": "council",
                        "candidates": [
                            {"id": "alice", "name": "Alice"},
                            {"id": "bob", "name": "Bob"},
                        ],
                    },
                    {
                        "id": "chair",
                        "candidates": [
                            {"id": "carol", "name": "Carol"},
                            {"id": "dave", "name": "Dave"},
                        ],
                    },
                ]
            }
            with patch(
                "scripts.e2e.journeys.voting.subprocess.run",
                return_value=MagicMock(stdout='{"ballotId":"native"}\n'),
            ):
                self.assertEqual(
                    portal.encrypt(style, {"council": "Bob", "chair": "Carol"}),
                    {"ballotId": "native"},
                )
            choices = json.loads((Path(directory) / "choices.json").read_text())
            self.assertEqual(
                [
                    (c["contest_id"], [(v["id"], v["selected"]) for v in c["choices"]])
                    for c in choices
                ],
                [
                    ("council", [("alice", -1), ("bob", 0)]),
                    ("chair", [("carol", 0), ("dave", -1)]),
                ],
            )


class VoterRealmReadinessContracts(unittest.TestCase):
    def test_voter_login_waits_for_the_new_realm_key_without_opening_a_ballot(self):
        from scripts.e2e.journeys import voting

        with patch.dict(
            os.environ, {"VOTING_PORTAL_URL": "https://voting.invalid"}
        ), patch.object(voting, "Keycloak") as keycloak, patch.object(
            voting, "Hasura"
        ) as hasura, patch(
            "scripts.e2e.journeys.client.time.sleep"
        ):
            keycloak.return_value.browser_login.return_value = {
                "access_token": "new-realm-token"
            }
            hasura.return_value.execute.side_effect = [
                {
                    "errors": [
                        {
                            "message": "Could not verify JWT",
                            "extensions": {"code": "invalid-jwt"},
                        }
                    ]
                },
                {"data": {"__typename": "query_root"}},
            ]
            self.assertEqual(
                voting.Portal("event", MagicMock()).login("voter"), "new-realm-token"
            )
            self.assertEqual(
                hasura.return_value.execute.call_args_list,
                [
                    unittest.mock.call("{ __typename }"),
                    unittest.mock.call("{ __typename }"),
                ],
            )

    def test_voter_login_does_not_retry_other_graphql_errors(self):
        from scripts.e2e.journeys import voting

        with patch.dict(
            os.environ, {"VOTING_PORTAL_URL": "https://voting.invalid"}
        ), patch.object(voting, "Keycloak") as keycloak, patch.object(
            voting, "Hasura"
        ) as hasura:
            keycloak.return_value.browser_login.return_value = {"access_token": "token"}
            hasura.return_value.execute.return_value = {
                "errors": [
                    {"message": "unauthorized", "extensions": {"code": "access-denied"}}
                ]
            }
            with self.assertRaisesRegex(AssertionError, "unauthorized"):
                voting.Portal("event", MagicMock()).login("voter")
            hasura.return_value.execute.assert_called_once()


if __name__ == "__main__":
    unittest.main()
