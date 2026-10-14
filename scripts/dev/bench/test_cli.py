# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from . import rust, ui_update, wasm
from .__main__ import build_choices, edit_choice, key_values, main, parser
from .edits import EditError, EditSpec, insert_marker
from .environment import HARNESS_ROOT
from .focused_test import SUITES


def checkout(root):
    (root / "packages").mkdir()
    (root / ".devcontainer").mkdir()
    return root


def rejected(arguments):
    """The exit status and stderr of a command line rejected before running."""
    errors = io.StringIO()
    with contextlib.redirect_stderr(errors):
        try:
            status = main(arguments)
        except SystemExit as exit_:
            status = exit_.code
    return status, errors.getvalue()


class ArgumentTest(unittest.TestCase):
    def test_label_must_be_a_slug(self):
        with tempfile.TemporaryDirectory() as directory:
            root = checkout(Path(directory))
            status, errors = rejected(
                [
                    "test",
                    "--label",
                    "Before",
                    "--checkout",
                    str(root),
                    "--suite",
                    "jest-voting",
                ]
            )
        self.assertEqual(status, 2)
        self.assertIn("invalid label", errors)

    def test_checkout_must_look_like_step(self):
        with tempfile.TemporaryDirectory() as directory:
            status, errors = rejected(
                [
                    "test",
                    "--label",
                    "before",
                    "--checkout",
                    directory,
                    "--suite",
                    "jest-voting",
                ]
            )
        self.assertEqual(status, 2)
        self.assertIn("not a step checkout", errors)

    def test_workspace_cold_refuses_a_given_daemon(self):
        with tempfile.TemporaryDirectory() as directory:
            root = checkout(Path(directory))
            status, errors = rejected(
                [
                    "workspace",
                    "--label",
                    "before",
                    "--checkout",
                    str(root),
                    "--cache",
                    "cold",
                    "--docker-host",
                    "unix:///tmp/x.sock",
                ]
            )
        self.assertEqual(status, 2)
        self.assertIn("create their own daemon", errors)

    def test_wasm_needs_an_edit_or_no_change(self):
        with tempfile.TemporaryDirectory() as directory:
            root = checkout(Path(directory))
            status, _ = rejected(["wasm", "--label", "before", "--checkout", str(root)])
        self.assertEqual(status, 2)

    def test_server_overrides_must_name_selected_targets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = checkout(Path(directory))
            status, errors = rejected(
                [
                    "ui-update",
                    "--label",
                    "before",
                    "--checkout",
                    str(root),
                    "--edit",
                    "shared-header",
                    "--target",
                    "voting",
                    "--server-cmd",
                    "admin=yarn start",
                ]
            )
        self.assertEqual(status, 2)
        self.assertIn("not selected", errors)

    def test_every_scenario_has_help(self):
        for scenario in (
            "workspace",
            "ui-update",
            "test",
            "wasm",
            "rust",
            "ci",
            "summarize",
            "clean",
        ):
            with (
                self.subTest(scenario=scenario),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                with self.assertRaises(SystemExit) as exit_:
                    parser().parse_args([scenario, "--help"])
                self.assertEqual(exit_.exception.code, 0)


class ChoiceTest(unittest.TestCase):
    def test_named_edits_come_from_builtins_or_a_file(self):
        self.assertIs(
            edit_choice("shared-header", None, ui_update.EDITS),
            ui_update.EDITS["shared-header"],
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "edits.json"
            path.write_text('{"mine": {"path": "a.rs", "template": "// {marker}"}}')
            self.assertEqual(
                edit_choice("mine", path, rust.RUST_EDITS),
                EditSpec("a.rs", "// {marker}"),
            )
        with self.assertRaisesRegex(EditError, "unknown edit"):
            edit_choice("missing", None, rust.RUST_EDITS)

    def test_builds_are_named_or_explicit(self):
        self.assertEqual(
            build_choices(["harvest", "core=cargo build -p sequent-core"]),
            {"harvest": rust.BUILDS["harvest"], "core": "cargo build -p sequent-core"},
        )
        with self.assertRaisesRegex(ValueError, "unknown build"):
            build_choices(["b4"])

    def test_key_values_need_a_separator(self):
        self.assertEqual(
            key_values(["voting=yarn start --port {port}"], "--server-cmd"),
            {"voting": "yarn start --port {port}"},
        )
        with self.assertRaisesRegex(ValueError, "NAME=VALUE"):
            key_values(["voting"], "--server-cmd")


class PresetTest(unittest.TestCase):
    """Built-in edits point inside the repository and carry the marker field."""

    def test_presets_are_valid_edits(self):
        edits = {**ui_update.EDITS, **rust.RUST_EDITS, **wasm.WASM_EDITS}
        edits.update({name: suite.edit for name, suite in SUITES.items()})
        for name, spec in edits.items():
            with self.subTest(name=name):
                self.assertTrue(spec.path.startswith("packages/"))
                self.assertIn("{marker}", spec.template)

    def test_presets_apply_to_this_repository(self):
        # A moved anchor must fail here rather than in a long benchmark run.
        edits = {**ui_update.EDITS, **rust.RUST_EDITS, **wasm.WASM_EDITS}
        edits.update({name: suite.edit for name, suite in SUITES.items()})
        for name, spec in edits.items():
            with self.subTest(name=name):
                text = (HARNESS_ROOT / spec.path).read_text(encoding="utf-8")
                self.assertIn("marker-check", insert_marker(text, spec, "marker-check"))

    def test_server_commands_take_the_port(self):
        for name, target in ui_update.TARGETS.items():
            with self.subTest(name=name):
                self.assertIn("--port 41000", target.server.format(port=41000))


if __name__ == "__main__":
    unittest.main()
