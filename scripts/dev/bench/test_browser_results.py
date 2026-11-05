# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from . import ui_update, wasm
from .common import SampleTimer
from .process import CommandResult
from .results import SampleRole


class BrowserResultTest(unittest.TestCase):
    def check_result(self, sample, errors, violations):
        self.assertEqual(sample.ok, errors == violations == 0)
        self.assertEqual(sample.detail["page_errors"], errors)
        self.assertEqual(sample.detail["mock_violations"], violations)
        if errors or violations:
            self.assertIsNone(sample.seconds)
            self.assertIn("browser reported", sample.error)
        else:
            self.assertEqual(sample.seconds, 1.25)
            self.assertIsNone(sample.error)

    def probe(self, errors, violations):
        probe = Mock()
        probe.collect.side_effect = [
            {},
            {
                "voting": {
                    "t": 101.25,
                    "reloads": 1,
                    "page_errors": errors,
                    "violations": violations,
                    "wasm_modules": 1,
                }
            },
        ]
        return probe

    def test_visible_ui_with_browser_failures_is_not_a_successful_sample(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            options = ui_update.UiUpdateOptions(
                root,
                "test",
                "voting-screen",
                ui_update.EDITS["voting-screen"],
                ["voting"],
                {},
                None,
                1,
                0,
                12345,
                10,
                10,
                0,
                root,
            )
            for errors, violations in ((0, 0), (1, 0), (0, 2)):
                with self.subTest(errors=errors, violations=violations):
                    run = Mock()
                    edit = Mock()
                    edit.apply.return_value = 100.0
                    ui_update.measure(
                        options,
                        self.probe(errors, violations),
                        edit,
                        {"voting": run},
                        root,
                        1,
                        SampleRole.MEASURED,
                        "new-content",
                        apply=True,
                    )
                    self.check_result(run.add.call_args.args[0], errors, violations)

    def test_loaded_wasm_with_browser_failures_is_not_a_successful_sample(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "build-1.log").write_text("")
            options = wasm.WasmOptions(
                root,
                "test",
                "sequent-core-wasm",
                wasm.WASM_EDITS["sequent-core-wasm"],
                "build",
                "",
                wasm.ServerRestart.NEVER,
                "voting",
                12345,
                1,
                0,
                10,
                root,
            )
            for errors, violations in ((0, 0), (1, 0), (0, 2)):
                with self.subTest(errors=errors, violations=violations):
                    run = Mock()
                    edit = Mock()
                    edit.apply.return_value = 100.0
                    with patch.object(
                        wasm,
                        "run_command",
                        return_value=CommandResult("build", 0, 0.1, ""),
                    ):
                        wasm.measure(
                            options,
                            run,
                            self.probe(errors, violations),
                            Mock(),
                            edit,
                            "build",
                            root,
                            SampleTimer(1),
                            "new-wasm",
                        )
                    self.check_result(run.add.call_args.args[0], errors, violations)


if __name__ == "__main__":
    unittest.main()
