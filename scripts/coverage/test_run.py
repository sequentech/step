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
import unittest
from pathlib import Path
from unittest.mock import patch

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

    def test_main_returns_the_measurement_exit_code(self) -> None:
        with patch.object(sys, "argv", ["run.py", "sequent-core", "--baseline"]):
            with patch.object(run, "measure", return_value=1) as measure:
                self.assertEqual(run.main(), 1)
            measure.assert_called_once_with("sequent-core", True, False)

    def test_overlapping_runs_cannot_clear_each_others_counters(self) -> None:
        lock_path = self.root / "coverage" / ".lock"
        lock_path.parent.mkdir()
        with (
            lock_path.open("a") as lock,
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
