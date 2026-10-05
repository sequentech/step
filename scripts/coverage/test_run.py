# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Exercise the runner's decisions without compiling a Rust workspace per case.

The fake tool emits the same small LLVM fixture for each attempt. Assertions focus
on observable results: a failed test, stale source or partial report must never
leave behind a successful decision. Real Cargo execution is checked separately.
"""

import contextlib
import io
import json
import os
import runpy
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path
from unittest.mock import patch

import ci
import run
from report import CoverageError
from test_report import export, llvm_file


class CoverageRunnerTests(unittest.TestCase):
    """Give every attempt its own checkout, config and output directory."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.workspace = self.root / "packages"
        self.source = self.workspace / "sequent-core" / "src" / "codec.rs"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("pub fn encode() {}\n")
        (self.workspace / "Cargo.lock").write_text("version = 4\n")
        self.manifest = self.workspace / "sequent-core" / "Cargo.toml"
        self.manifest.write_text(
            '[package]\nname = "sequent-core"\n'
            "[features]\ndefault_features = []\nkeycloak = []\n"
        )
        (self.root / "rust-toolchain.toml").write_text(
            '[toolchain]\nchannel = "1.96.0"\n'
        )
        self.config = self.root / "profiles.toml"
        self.config.write_text("""cargo_llvm_cov_version = "0.9.1"
minimum_lines = 95
[profiles.sequent-core]
package = "sequent-core"
features = ["default_features", "keycloak"]
limitations = ["Native test profile only"]
issue = "https://github.com/sequentech/meta/issues/13292"
[profiles.sequent-core.scope_exceptions]
""")
        for name, value in (
            ("ROOT", self.root),
            ("WORKSPACE", self.workspace),
            ("CONFIG", self.config),
        ):
            self.enterContext(patch.object(run, name, value))
        self.enterContext(patch.object(run, "git_output", return_value="abc123"))
        self.enterContext(
            patch.object(run, "checkout_digest", return_value="unchanged")
        )
        self.enterContext(contextlib.redirect_stdout(io.StringIO()))
        self.enterContext(contextlib.redirect_stderr(io.StringIO()))
        self.enterContext(patch.dict(os.environ, {}, clear=True))
        self.covered = 95
        self.command_environments = []
        self.test_log = (
            "test result: ok. 3 passed; 0 failed; 1 ignored; "
            "0 measured; 0 filtered out;"
        )

    def tool_output(
        self, command: list[str], log: Path, environment: dict[str, str]
    ) -> str:
        """Produce known counters; never execute a command supplied by a test."""
        log.write_text("Recorded test command\n")
        self.command_environments.append(environment)
        if command == ["cargo", "llvm-cov", "--version"]:
            return "cargo-llvm-cov 0.9.1\n"
        if command == ["rustc", "--version"]:
            return "rustc 1.96.0 (test fixture)\n"
        if "--tests" in command:
            self.assertIn("--locked", command)
            self.assertIn("default_features,keycloak", command)
            return self.test_log
        if "--json" in command:
            destination = Path(command[command.index("--output-path") + 1])
            destination.write_text(
                json.dumps(export(llvm_file(self.source, self.covered)))
            )
        if "--lcov" in command:
            destination = Path(command[command.index("--output-path") + 1])
            destination.write_text(f"SF:{self.source}\nDA:1,1\nend_of_record\n")
        if "--html" in command:
            destination = Path(command[command.index("--output-dir") + 1]) / "html"
            destination.mkdir()
            (destination / "index.html").write_text("<html>Coverage</html>\n")
        return ""

    def attempt(self, baseline: bool = False) -> tuple[int, dict]:
        with patch.object(run, "execute", side_effect=self.tool_output):
            code = run.measure("sequent-core", baseline, offline=True)
        summaries = sorted(self.root.glob("coverage/sequent-core/*/summary.json"))
        self.assertEqual(len(summaries), 1)
        return code, json.loads(summaries[0].read_text())

    def test_strict_run_records_coverage_features_and_test_counts(self) -> None:
        code, summary = self.attempt()
        self.assertEqual(code, 0)
        self.assertTrue(summary["passes"])
        self.assertEqual(summary["status"], "measured")
        self.assertEqual(summary["tests_passed"], 3)
        self.assertEqual(summary["tests_ignored"], 1)
        self.assertEqual(summary["features"], ["default_features", "keycloak"])

    def test_consumers_run_without_contributing_to_package_counters(self):
        self.config.write_text(
            self.config.read_text().replace(
                'package = "sequent-core"',
                'package = "sequent-core"\nconsumer_packages = ["windmill"]',
            )
        )
        consumer = self.workspace / "windmill/src/query.rs"
        consumer.parent.mkdir(parents=True)
        consumer.write_text("pub fn query() {}\n")
        original = self.tool_output

        def tool_output(command, log, environment):
            output = original(command, log, environment)
            if "--tests" in command:
                packages = [
                    command[index + 1]
                    for index, argument in enumerate(command)
                    if argument == "--package"
                ]
                self.assertEqual(packages, ["sequent-core", "windmill"])
            if "--json" in command:
                destination = Path(command[command.index("--output-path") + 1])
                destination.write_text(
                    json.dumps(export(llvm_file(self.source), llvm_file(consumer, 100)))
                )
            return output

        self.tool_output = tool_output
        code, summary = self.attempt()
        self.assertEqual(code, 0)
        self.assertEqual(summary["consumer_packages"], ["windmill"])
        self.assertEqual(summary["metrics"]["lines"]["covered"], 95)
        self.assertEqual(summary["metrics"]["lines"]["count"], 100)

    def test_profile_fixture_environment_overrides_the_callers_service_settings(
        self,
    ) -> None:
        # A local fixture must never inherit a developer's production endpoint.
        # Keep unrelated caller settings and record the declared fixture values.
        self.config.write_text(
            self.config.read_text()
            + """
[profiles.sequent-core.test_environment]
HASURA_DB__HOST = "127.0.0.1"
"""
        )
        with patch.dict(
            os.environ, {"HASURA_DB__HOST": "remote.invalid", "LANG": "C.UTF-8"}
        ):
            code, summary = self.attempt()
        self.assertEqual(code, 0)
        for environment in self.command_environments:
            self.assertEqual(environment["HASURA_DB__HOST"], "127.0.0.1")
            self.assertEqual(environment["LANG"], "C.UTF-8")
        self.assertEqual(summary["test_environment"], {"HASURA_DB__HOST": "127.0.0.1"})

    def test_offline_mode_applies_to_probes_tests_and_report_commands(self) -> None:
        self.attempt()
        self.assertGreater(len(self.command_environments), 5)
        self.assertTrue(
            all(env["CARGO_NET_OFFLINE"] == "true" for env in self.command_environments)
        )

    def test_locked_metadata_failure_stops_before_test_execution(self) -> None:
        commands = []

        def failed_metadata(
            command: list[str], log: Path, environment: dict[str, str]
        ) -> str:
            commands.append(command)
            if command[:2] == ["cargo", "metadata"]:
                self.assertIn("--locked", command)
                raise CoverageError("Lockfile needs an update")
            return self.tool_output(command, log, environment)

        with patch.object(run, "execute", side_effect=failed_metadata):
            self.assertEqual(run.measure("sequent-core", True, True), 2)
        self.assertFalse(any("--tests" in command for command in commands))

    def test_cleanup_precedes_tests_and_failure_stops_measurement(self) -> None:
        commands = []

        def failing_cleanup(
            command: list[str], log: Path, environment: dict[str, str]
        ) -> str:
            commands.append(command)
            if command[:3] == ["cargo", "llvm-cov", "clean"]:
                self.assertIn("--workspace", command)
                raise CoverageError("Stale profile cleanup failed")
            return self.tool_output(command, log, environment)

        with patch.object(run, "execute", side_effect=failing_cleanup):
            self.assertEqual(run.measure("sequent-core", True, True), 2)
        self.assertTrue(any("clean" in command for command in commands))
        self.assertFalse(any("--tests" in command for command in commands))

    def test_successful_run_cleans_before_collecting_new_counters(self) -> None:
        # Clearing counters alone retains binaries from old feature profiles.
        # Their source regions must not contaminate the new denominator.
        commands = []

        def record(command: list[str], log: Path, environment: dict[str, str]) -> str:
            commands.append(command)
            return self.tool_output(command, log, environment)

        with patch.object(run, "execute", side_effect=record):
            self.assertEqual(run.measure("sequent-core", False, True), 0)
        cleanup = commands.index(["cargo", "llvm-cov", "clean", "--workspace"])
        collect = next(
            index for index, command in enumerate(commands) if "--tests" in command
        )
        self.assertLess(cleanup, collect)

    def test_strict_shortfall_fails(self) -> None:
        self.covered = 94
        code, summary = self.attempt()
        self.assertEqual(code, 1)
        self.assertFalse(summary["passes"])

    def test_baseline_records_the_shortfall_even_when_measurement_succeeds(
        self,
    ) -> None:
        self.covered = 94
        code, summary = self.attempt(baseline=True)
        self.assertEqual(code, 0)
        self.assertFalse(summary["passes"])
        self.assertEqual(summary["mode"], "baseline")

    def test_no_tests_is_an_error_in_baseline_mode_too(self) -> None:
        self.test_log = "test result: ok. 0 passed; 0 failed; 12 ignored;"
        code, summary = self.attempt(baseline=True)
        self.assertEqual(code, 2)
        self.assertEqual(summary["status"], "error")
        self.assertIn("No passing tests", summary["error"])

    def test_source_changes_in_a_dependency_invalidate_the_run(self) -> None:
        with patch.object(run, "checkout_digest", side_effect=["before", "after"]):
            code, summary = self.attempt()
        self.assertEqual(code, 2)
        self.assertFalse(summary["passes"])
        self.assertIn("Source changed", summary["error"])

    def test_wrong_toolchain_or_failed_tests_leave_an_error_artifact(self) -> None:
        for response in ("cargo-llvm-cov 0.0.1", "cargo-llvm-cov 0.9.1"):
            with self.subTest(response=response):
                with patch.object(
                    run, "execute", side_effect=[response, "rustc 1.95.0 (wrong)"]
                ):
                    self.assertEqual(run.measure("sequent-core", False, False), 2)

        def failing_tests(
            command: list[str], log: Path, environment: dict[str, str]
        ) -> str:
            if "--tests" in command:
                raise CoverageError("Tests failed")
            return self.tool_output(command, log, environment)

        with patch.object(run, "execute", side_effect=failing_tests):
            self.assertEqual(run.measure("sequent-core", True, True), 2)
        for path in self.root.glob("coverage/sequent-core/*/summary.json"):
            summary = json.loads(path.read_text())
            self.assertEqual(summary["status"], "error")
            self.assertFalse(summary["passes"])

    def test_ci_summary_explicitly_says_the_target_is_not_met(self) -> None:
        self.covered = 80
        destination = self.root / "ci-summary.md"
        with patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(destination)}):
            self.attempt(baseline=True)
        self.assertIn("TARGET NOT MET", destination.read_text())
        self.assertIn("80.00%", destination.read_text())

    def test_missing_file_is_visible_in_the_human_summary(self) -> None:
        (self.source.parent / "authorization.rs").write_text("pub fn authorize() {}\n")
        code, summary = self.attempt()
        self.assertEqual(code, 1)
        self.assertIn(
            "src/authorization.rs", run.markdown_summary("sequent-core", summary)
        )

    def test_profile_without_extra_features_does_not_enable_them_implicitly(
        self,
    ) -> None:
        self.config.write_text(
            self.config.read_text().replace('["default_features", "keycloak"]', "[]")
        )

        def tool(command: list[str], log: Path, environment: dict[str, str]) -> str:
            if "--tests" in command:
                self.assertNotIn("--features", command)
                self.assertNotIn("--offline", command)
                return self.test_log
            return self.tool_output(command, log, environment)

        with patch.object(run, "execute", side_effect=tool):
            self.assertEqual(run.measure("sequent-core", False, False), 0)

    def test_external_reports_do_not_create_coverage_files_in_the_checkout(self):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory)
            with patch.object(run, "execute", side_effect=self.tool_output):
                self.assertEqual(
                    run.measure("sequent-core", True, True, destination), 0
                )
            self.assertFalse((self.root / "coverage").exists())
            report = json.loads(
                next(destination.glob("sequent-core/*/summary.json")).read_text()
            )
            self.assertEqual(report["package"], "sequent-core")

    def test_main_returns_the_measurement_exit_code(self) -> None:
        with patch.object(sys, "argv", ["run.py", "sequent-core", "--baseline"]):
            with patch.object(run, "measure", return_value=1) as measure:
                self.assertEqual(run.main(), 1)
            measure.assert_called_once_with("sequent-core", True, False, None, False)

    def test_main_passes_the_comparison_base_flag(self) -> None:
        arguments = ["run.py", "sequent-core", "--baseline", "--comparison-base"]
        with patch.object(sys, "argv", arguments):
            with patch.object(run, "measure", return_value=0) as measure:
                self.assertEqual(run.main(), 0)
            measure.assert_called_once_with("sequent-core", True, False, None, True)

    def test_overlapping_runs_cannot_clear_each_others_counters(self) -> None:
        lock_path = self.root / ".git" / "package-coverage.lock"
        lock_path.parent.mkdir()
        with (
            lock_path.open("a") as lock,
            patch.object(run, "git_output", return_value=str(lock_path)),
            patch.object(sys, "argv", ["run.py", "sequent-core"]),
        ):
            run.fcntl.flock(lock, run.fcntl.LOCK_EX | run.fcntl.LOCK_NB)
            with patch.object(run, "measure") as measure:
                self.assertEqual(run.main(), 2)
                measure.assert_not_called()

    def test_partial_export_cannot_reuse_an_earlier_success(self) -> None:
        self.attempt()

        def failing_export(
            command: list[str], log: Path, environment: dict[str, str]
        ) -> str:
            if "--json" in command:
                raise CoverageError("Export failed")
            return self.tool_output(command, log, environment)

        with patch.object(run, "execute", side_effect=failing_export):
            self.assertEqual(run.measure("sequent-core", False, False), 2)
        summaries = [
            json.loads(path.read_text())
            for path in self.root.glob("coverage/sequent-core/*/summary.json")
        ]
        self.assertEqual(len(summaries), 2)
        self.assertEqual(
            sorted(result["status"] for result in summaries), ["error", "measured"]
        )

    def test_absent_or_truncated_human_reports_prevent_success(self) -> None:
        for lcov, html in (
            ("", "<html></html>"),
            ("SF:x\nDA:1,1\n", "<html></html>"),
            ("SF:x\nDA:1,1\nend_of_record\n", "<html>"),
        ):
            output = self.root / "report-fixture"
            (output / "html").mkdir(parents=True, exist_ok=True)
            (output / "lcov.info").write_text(lcov)
            (output / "html" / "index.html").write_text(html)
            with self.subTest(lcov=lcov, html=html), self.assertRaises(CoverageError):
                run.validate_artifacts(output)

        (output / "html" / "index.html").unlink()
        with self.assertRaises(FileNotFoundError):
            run.validate_artifacts(output)

    def test_every_visible_export_uses_the_same_fixture_exclusion(self):
        fixture = self.source.parent / "fixture.rs"
        fixture.write_text("pub fn fixture() {}\n")
        self.config.write_text(
            self.config.read_text()
            + (
                "[profiles.sequent-core.excluded_files]\n"
                '"src/fixture.rs" = "Test data only."\n'
            )
        )
        commands = []

        def tool(command, log, environment):
            commands.append(command)
            result = self.tool_output(command, log, environment)
            if "--json" in command:
                destination = Path(command[-1])
                payload = json.loads(destination.read_text())
                payload["data"][0]["functions"] = [
                    {"name": "decode", "filenames": [str(self.source)], "count": 3},
                    {"name": "fixture", "filenames": [str(fixture)], "count": 99},
                ]
                destination.write_text(json.dumps(payload))
            return result

        with patch.object(run, "execute", side_effect=tool):
            self.assertEqual(run.measure("sequent-core", False, True), 0)
        reports = [command for command in commands if "report" in command]
        self.assertEqual(len(reports), 4)
        self.assertTrue(
            all("--ignore-filename-regex" in command for command in reports)
        )
        self.assertFalse(any("llvm.raw.json" in command for command in reports))
        summary = json.loads(
            next(self.root.glob("coverage/sequent-core/*/summary.json")).read_text()
        )
        self.assertEqual(summary["metrics"]["lines"]["count"], 100)
        self.assertEqual(
            summary["excluded_files"], {"src/fixture.rs": "Test data only."}
        )
        self.assertIn("src/fixture.rs", run.markdown_summary("sequent-core", summary))
        payload = json.loads(
            next(self.root.glob("coverage/sequent-core/*/llvm.json")).read_text()
        )
        self.assertEqual(
            payload["data"][0]["functions"],
            [{"name": "decode", "filenames": [str(self.source)], "count": 3}],
        )

    def test_invalid_exclusion_fails_before_starting_cargo(self):
        self.config.write_text(
            self.config.read_text()
            + ('[profiles.sequent-core.excluded_files]\n"src/*.rs" = "Too broad."\n')
        )
        with patch.object(run, "execute") as command:
            self.assertEqual(run.measure("sequent-core", True, True), 2)
            command.assert_not_called()

    def newer_policy_files(self) -> None:
        """Name a declarations-only file and a test module that the head adds."""
        self.config.write_text(
            self.config.read_text()
            + '"src/ports.rs" = "Trait declarations only."\n'
            + "[profiles.sequent-core.excluded_files]\n"
            + '"src/load_tests.rs" = "Test module only."\n'
        )

    def test_a_comparison_base_skips_policy_entries_for_files_it_predates(self):
        self.newer_policy_files()
        with patch.object(run, "execute", side_effect=self.tool_output):
            code = run.measure("sequent-core", True, True, comparison_base=True)
        self.assertEqual(code, 0)
        summary = json.loads(
            next(self.root.glob("coverage/sequent-core/*/summary.json")).read_text()
        )
        self.assertEqual(
            summary["policy_files_absent_from_base"],
            ["src/load_tests.rs", "src/ports.rs"],
        )
        self.assertEqual(summary["excluded_files"], {})

    def comparison_base_features(self, features: str) -> tuple[list[str], dict]:
        """Run a comparison base whose head profile requests these features."""
        self.config.write_text(
            self.config.read_text().replace(
                '["default_features", "keycloak"]', features
            )
        )
        requested = []

        def tool(command: list[str], log: Path, environment: dict[str, str]) -> str:
            if "--tests" in command:
                requested.append(command[command.index("--features") + 1])
                return self.test_log
            return self.tool_output(command, log, environment)

        with patch.object(run, "execute", side_effect=tool):
            code = run.measure("sequent-core", True, True, comparison_base=True)
        self.assertEqual(code, 0)
        summary = json.loads(
            next(self.root.glob("coverage/sequent-core/*/summary.json")).read_text()
        )
        return requested, summary

    def test_a_comparison_base_is_built_without_features_it_predates(self):
        requested, summary = self.comparison_base_features(
            '["default_features", "keycloak", "election_config_xlsx"]'
        )
        self.assertEqual(requested, ["default_features,keycloak"])
        # The recorded profile stays the head's so the two runs stay comparable.
        self.assertEqual(
            summary["features"],
            ["default_features", "keycloak", "election_config_xlsx"],
        )
        self.assertEqual(summary["features_absent_from_base"], ["election_config_xlsx"])
        self.assertIn(
            "built without: `election_config_xlsx`",
            run.markdown_summary("sequent-core", summary),
        )

    def test_optional_dependencies_and_dependency_features_are_not_absent(self):
        self.manifest.write_text(
            self.manifest.read_text()
            + "[dependencies]\n"
            + 'rug = { version = "1", optional = true }\n'
            + "[target.'cfg(unix)'.dependencies]\n"
            + 'zip = { version = "2", optional = true }\n'
            + 'serde = "1"\n'
        )
        requested, summary = self.comparison_base_features(
            '["default_features", "keycloak", "rug", "zip", "strand/rayon", "serde"]'
        )
        self.assertEqual(requested, ["default_features,keycloak,rug,zip,strand/rayon"])
        # A required dependency is not a feature Cargo would accept.
        self.assertEqual(summary["features_absent_from_base"], ["serde"])

    def test_a_head_run_requests_every_profile_feature(self):
        self.config.write_text(
            self.config.read_text().replace(
                '["default_features", "keycloak"]',
                '["default_features", "keycloak", "election_config_xlsx"]',
            )
        )
        requested = []

        def tool(command: list[str], log: Path, environment: dict[str, str]) -> str:
            if "--tests" in command:
                requested.append(command[command.index("--features") + 1])
                return self.test_log
            return self.tool_output(command, log, environment)

        with patch.object(run, "execute", side_effect=tool):
            self.assertEqual(run.measure("sequent-core", True, True), 0)
        self.assertEqual(requested, ["default_features,keycloak,election_config_xlsx"])
        summary = json.loads(
            next(self.root.glob("coverage/sequent-core/*/summary.json")).read_text()
        )
        self.assertNotIn("features_absent_from_base", summary)

    def test_policy_entries_for_missing_files_fail_outside_a_comparison_base(self):
        self.newer_policy_files()
        with patch.object(run, "execute") as command:
            self.assertEqual(run.measure("sequent-core", True, True), 2)
            command.assert_not_called()

    def test_excluded_files_cannot_remain_in_the_published_llvm_export(self):
        fixture = self.source.parent / "fixture.rs"
        fixture.write_text("pub fn fixture() {}\n")
        self.config.write_text(
            self.config.read_text()
            + (
                "[profiles.sequent-core.excluded_files]\n"
                '"src/fixture.rs" = "Test data only."\n'
            )
        )

        def tool(command, log, environment):
            result = self.tool_output(command, log, environment)
            if "--json" in command and command[-1].endswith("/llvm.json"):
                Path(command[-1]).write_text(
                    json.dumps(export(llvm_file(self.source), llvm_file(fixture)))
                )
            return result

        with patch.object(run, "execute", side_effect=tool):
            self.assertEqual(run.measure("sequent-core", True, True), 2)
        self.assertEqual(list(self.root.glob("coverage/sequent-core/*/llvm.json")), [])

    def test_later_export_failure_cannot_publish_unfiltered_function_records(self):
        def tool(command, log, environment):
            result = self.tool_output(command, log, environment)
            if "--json" in command:
                path = Path(command[-1])
                payload = json.loads(path.read_text())
                payload["data"][0]["functions"] = [
                    {"name": "test", "filenames": [str(self.root / "tests/test.rs")]},
                ]
                path.write_text(json.dumps(payload))
            if "--lcov" in command:
                raise CoverageError("interrupted LCOV export")
            return result

        with patch.object(run, "execute", side_effect=tool):
            self.assertEqual(run.measure("sequent-core", True, True), 2)
        output = next(self.root.glob("coverage/sequent-core/*/summary.json")).parent
        self.assertEqual(
            json.loads((output / "llvm.json").read_text())["data"][0]["functions"], []
        )
        self.assertFalse((output / "lcov.info").exists())

    def test_interrupted_json_export_removes_partial_artifact(self):
        def tool(command, log, environment):
            result = self.tool_output(command, log, environment)
            if "--json" in command:
                Path(command[-1]).write_text("{partial")
                raise CoverageError("interrupted JSON export")
            return result

        with patch.object(run, "execute", side_effect=tool):
            self.assertEqual(run.measure("sequent-core", True, True), 2)
        self.assertEqual(list(self.root.glob("coverage/sequent-core/*/llvm.json")), [])


class CheckoutIdentityTests(unittest.TestCase):
    """A package-local hash is insufficient when workspace inputs can change."""

    def test_identity_changes_for_dependencies_links_and_deleted_inputs(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run, "ROOT", Path(directory)),
        ):
            root = Path(directory)
            dependency = root / "dependency.rs"
            dependency.write_text("version one")
            link = root / "linked.rs"
            link.symlink_to("dependency.rs")
            with patch.object(
                run, "git_output", return_value="dependency.rs\0linked.rs\0missing.rs\0"
            ):
                before = run.checkout_digest()
                dependency.write_text("version two")
                self.assertNotEqual(before, run.checkout_digest())

    def test_git_identity_command_preserves_arguments_without_a_shell(self) -> None:
        with patch.object(
            run.subprocess, "check_output", return_value="abc123\n"
        ) as command:
            self.assertEqual(run.git_output("rev-parse", "HEAD"), "abc123")
        command.assert_called_once_with(
            ["git", "-C", str(run.ROOT), "rev-parse", "HEAD"], text=True
        )

    def test_help_entrypoint_does_not_run_cargo(self) -> None:
        with (
            patch.object(sys, "argv", ["run.py", "--help"]),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            with self.assertRaises(SystemExit) as exit_result:
                runpy.run_path(run.__file__, run_name="__main__")
        self.assertEqual(exit_result.exception.code, 0)

    def test_nul_delimited_filenames_preserve_whitespace(self) -> None:
        names = " leading.rs\0trailing .rs\0"
        with patch.object(run.subprocess, "check_output", return_value=names):
            self.assertEqual(run.git_output("ls-files", "-z"), names)


class CommandExecutionTests(unittest.TestCase):
    """Check real process exit handling with tiny, deterministic Python commands."""

    def test_outer_native_timeout_cleans_the_inner_command_session(self) -> None:
        self.assert_nested_cleanup("timeout")

    def test_outer_keyboard_interrupt_cleans_the_inner_command_session(self) -> None:
        self.assert_nested_cleanup("interrupt")

    def test_outer_sigterm_cleans_the_inner_command_session(self) -> None:
        self.assert_nested_cleanup("sigterm")

    def assert_nested_cleanup(self, cancellation: str) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "packages/fixture/src"
            package.mkdir(parents=True)
            (package / "lib.rs").write_text("pub fn fixture() {}\n")
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            config = root / "profiles.toml"
            config.write_text(
                '[profiles.fixture]\npackage="fixture"\nfeatures=[]\nscope_exceptions={}\n'
            )
            marker = root / "cargo.pid"
            tools = root / "bin"
            tools.mkdir()
            cargo = tools / "cargo"
            cargo.write_text(
                f"#!{sys.executable}\nimport os,time\nfrom pathlib import Path\n"
                f"Path({str(marker)!r}).write_text(str(os.getpid()))\n"
                "time.sleep(60)\n"
            )
            cargo.chmod(0o755)
            script = (
                f"import sys; sys.path.insert(0, {str(Path(run.__file__).parent)!r}); "
                "import run; from pathlib import Path; "
                f"run.CONFIG=Path({str(config)!r}); "
                "sys.argv=['run.py','fixture','--baseline','--checkout',"
                f"{str(root)!r}]; "
                "raise SystemExit(run.main())"
            )
            # The outer guard is deliberately shorter than its inner command.
            environment = dict(
                os.environ,
                PATH=f"{tools}:{os.environ['PATH']}",
                NATIVE_COVERAGE_TIMEOUT_SECONDS="2",
            )
            script = (
                'import os; os.environ["NATIVE_COVERAGE_TIMEOUT_SECONDS"]="3600"; '
                + script
            )
            pid = None
            sender = None
            try:
                if cancellation != "timeout":

                    def send_when_running():
                        deadline = time.monotonic() + 1.5
                        while not marker.exists() and time.monotonic() < deadline:
                            time.sleep(0.01)
                        if marker.exists():
                            os.kill(
                                os.getpid(),
                                run.signal.SIGINT
                                if cancellation == "interrupt"
                                else run.signal.SIGTERM,
                            )

                    sender = threading.Thread(target=send_when_running)
                    sender.start()
                expected = (
                    KeyboardInterrupt if cancellation == "interrupt" else CoverageError
                )
                with (
                    patch.dict(os.environ, environment),
                    run.native_cancellation(),
                    self.assertRaises(expected),
                ):
                    ci.command(
                        [sys.executable, "-c", script], root, root, "outer", native=True
                    )
                self.assertTrue(marker.exists(), (root / "outer.log").read_text())
                pid = int(marker.read_text())
                deadline = time.monotonic() + 2
                while time.monotonic() < deadline:
                    try:
                        os.kill(pid, 0)
                    except ProcessLookupError:
                        break
                    time.sleep(0.02)
                else:
                    self.fail("Outer timeout left the native command session running")
            finally:
                if sender is not None:
                    sender.join()
                if pid is None and marker.exists():
                    pid = int(marker.read_text())
                if pid is not None:
                    try:
                        os.killpg(pid, run.signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    def test_pending_interrupt_during_creation_cleans_the_assigned_group(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen") as popen,
            patch.object(run.os, "killpg") as kill,
        ):
            child = popen.return_value
            child.pid = 12345
            child.wait.return_value = 0

            def create(*_args, **_kwargs):
                os.kill(os.getpid(), run.signal.SIGINT)
                return child

            popen.side_effect = create
            with self.assertRaises(KeyboardInterrupt):
                run.execute(["cargo"], Path(directory) / "creation.log", {})
            kill.assert_called_once_with(12345, run.signal.SIGKILL)
            child.wait.assert_called_once()

    def test_native_budget_reaches_the_actual_process_wait(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen") as popen,
        ):
            popen.return_value.wait.return_value = 0
            for budget, expected in (
                ({}, 1200),
                ({"NATIVE_COVERAGE_TIMEOUT_SECONDS": "3600"}, 3600),
            ):
                run.execute(["cargo"], Path(directory) / "budget.log", budget)
                popen.return_value.wait.assert_called_with(timeout=expected)

    def test_invalid_native_budgets_never_start_a_command(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen") as popen,
        ):
            for value in ["0", "-1", "3601", "1.5", "many", "", " 30"]:
                with (
                    self.subTest(value=value),
                    self.assertRaisesRegex(
                        CoverageError, "NATIVE_COVERAGE_TIMEOUT_SECONDS"
                    ),
                ):
                    run.execute(
                        ["cargo"],
                        Path(directory) / "invalid.log",
                        {"NATIVE_COVERAGE_TIMEOUT_SECONDS": value},
                    )
            popen.assert_not_called()

    def test_extended_timeout_still_kills_the_entire_process_group(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen") as popen,
            patch.object(run.os, "killpg") as kill,
        ):
            popen.return_value.pid = 12345
            popen.return_value.wait.side_effect = [
                subprocess.TimeoutExpired("cargo", 3600),
                0,
            ]
            with self.assertRaisesRegex(CoverageError, "3600 seconds"):
                run.execute(
                    ["cargo"],
                    Path(directory) / "timeout.log",
                    {"NATIVE_COVERAGE_TIMEOUT_SECONDS": "3600"},
                )
            kill.assert_called_once_with(12345, run.signal.SIGKILL)
            self.assertEqual(
                popen.return_value.wait.call_args_list[0].kwargs["timeout"], 3600
            )

    def test_uncooperative_child_is_killed_after_bounded_grace(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen") as popen,
            patch.object(run.os, "killpg") as kill,
        ):
            popen.return_value.pid = 12345
            popen.return_value.wait.side_effect = [
                subprocess.TimeoutExpired("cargo", 1200),
                subprocess.TimeoutExpired("cargo", 5),
                0,
            ]
            with self.assertRaises(CoverageError):
                run.execute(
                    ["cargo"], Path(directory) / "timeout.log", {}, cooperative=True
                )
            self.assertEqual(
                [c.args for c in kill.call_args_list],
                [(12345, run.signal.SIGTERM), (12345, run.signal.SIGKILL)],
            )
            self.assertEqual(
                popen.return_value.wait.call_args_list[1].kwargs["timeout"], 5
            )

    def test_a_group_that_finishes_during_cancellation_is_still_reaped(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen") as popen,
            patch.object(run.os, "killpg", side_effect=ProcessLookupError),
        ):
            popen.return_value.pid = 12345
            popen.return_value.wait.side_effect = [
                subprocess.TimeoutExpired("cargo", 1200),
                0,
            ]
            with self.assertRaisesRegex(CoverageError, "1200 seconds"):
                run.execute(["cargo"], Path(directory) / "finished.log", {})
            self.assertEqual(popen.return_value.wait.call_count, 2)

    def test_native_cancellation_restores_the_importing_process(self) -> None:
        previous_handler = run.signal.getsignal(run.signal.SIGTERM)
        previous_mask = run.signal.pthread_sigmask(run.signal.SIG_BLOCK, set())
        with (
            self.assertRaisesRegex(CoverageError, "cancelled"),
            run.native_cancellation(),
        ):
            run.cancel_native_run(run.signal.SIGTERM, None)
        self.assertEqual(run.signal.getsignal(run.signal.SIGTERM), previous_handler)
        self.assertEqual(
            run.signal.pthread_sigmask(run.signal.SIG_BLOCK, set()), previous_mask
        )

    def test_a_failed_launch_restores_the_signal_mask(self) -> None:
        previous_mask = run.signal.pthread_sigmask(run.signal.SIG_BLOCK, set())
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen", side_effect=FileNotFoundError),
        ):
            with self.assertRaises(FileNotFoundError):
                run.execute(
                    [str(Path(directory) / "missing")],
                    Path(directory) / "failed.log",
                    {},
                )
        self.assertEqual(
            run.signal.pthread_sigmask(run.signal.SIG_BLOCK, set()), previous_mask
        )

    def test_normal_child_keeps_the_original_signal_mask(self) -> None:
        previous_mask = run.signal.pthread_sigmask(run.signal.SIG_BLOCK, set())
        with tempfile.TemporaryDirectory() as directory:
            result = run.execute(
                [
                    sys.executable,
                    "-c",
                    "import signal; print(sorted(int(s) for s in "
                    "signal.pthread_sigmask(signal.SIG_BLOCK,set())))",
                ],
                Path(directory) / "mask.log",
                dict(os.environ),
            )
        self.assertEqual(result.strip(), repr(sorted(int(s) for s in previous_mask)))

    def test_missing_executable_fails_and_retains_exec_error(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "exec.log"
            with self.assertRaisesRegex(CoverageError, "Command failed"):
                run.execute([str(Path(directory) / "missing")], log, dict(os.environ))
            self.assertIn("FileNotFoundError", log.read_text())

    def test_failed_command_is_an_error_and_keeps_its_log(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "failed.log"
            with self.assertRaises(CoverageError):
                run.execute(
                    [
                        sys.executable,
                        "-c",
                        'print("failure detail"); raise SystemExit(7)',
                    ],
                    log,
                    dict(os.environ),
                )
            self.assertIn("failure detail", log.read_text())

    def test_successful_command_returns_the_captured_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = run.execute(
                [sys.executable, "-c", 'print("done")'],
                Path(directory) / "ok.log",
                dict(os.environ),
            )
            self.assertEqual(output, "done\n")

    def test_timeout_terminates_the_process_group_and_is_an_error(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen") as popen,
        ):
            process = popen.return_value
            process.pid = 12345
            process.wait.side_effect = [subprocess.TimeoutExpired("cargo", 1200), 0]
            with (
                patch.object(run.os, "killpg") as kill,
                self.assertRaises(CoverageError),
            ):
                run.execute(["cargo"], Path(directory) / "timeout.log", {})
            kill.assert_called_once_with(12345, run.signal.SIGKILL)

    def test_interruption_also_cleans_up_the_child_process_group(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(run.subprocess, "Popen") as popen,
        ):
            process = popen.return_value
            process.pid = 12345
            process.wait.side_effect = [KeyboardInterrupt, 0]
            with (
                patch.object(run.os, "killpg") as kill,
                self.assertRaises(KeyboardInterrupt),
            ):
                run.execute(["cargo"], Path(directory) / "interrupted.log", {})
            kill.assert_called_once_with(12345, run.signal.SIGKILL)


if __name__ == "__main__":
    unittest.main()
