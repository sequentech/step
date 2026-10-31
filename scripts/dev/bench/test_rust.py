# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from . import rust, timed_linker
from .rust import build_environment, critical_path, parse_timings, timings_summary
from .wasm import BUILD_SCRIPT, default_build_command, patched_script, script_phases

SCRIPT = "TARGET_DIR=/workspaces/step/packages/sequent-core"


def unit(
    index,
    name,
    start,
    duration,
    rmeta=None,
    unlocked=(),
    unlocked_rmeta=(),
    target=" lib",
):
    return {
        "i": index,
        "name": name,
        "version": "0.1.0",
        "mode": "todo",
        "target": target,
        "start": start,
        "duration": duration,
        "rmeta_time": rmeta,
        "unblocked_units": list(unlocked),
        "unblocked_rmeta_units": list(unlocked_rmeta),
    }


# sequent-core's metadata lets windmill start early; its full build unlocks
# the harvest binary, which also waits for windmill.
UNITS = [
    unit(0, "sequent-core", 0.0, 10.0, rmeta=4.0, unlocked=[2], unlocked_rmeta=[1]),
    unit(1, "windmill", 4.0, 20.0, rmeta=12.0, unlocked=[2]),
    unit(2, "harvest", 24.0, 30.0, target=' bin "harvest"'),
    # Fresh units appear with no duration and must not count as compiled.
    unit(3, "serde", 0.0, 0.0),
]


class TimingsTest(unittest.TestCase):
    def test_reads_unit_data_from_the_report(self):
        html = f"<script>\nconst UNIT_DATA = {json.dumps(UNITS)};\nconst OTHER = [];"
        self.assertEqual(parse_timings(html), UNITS)
        with self.assertRaisesRegex(ValueError, "UNIT_DATA"):
            parse_timings("<html></html>")

    def test_critical_path_follows_the_latest_unlocker(self):
        path = critical_path(UNITS)
        self.assertEqual(
            [step["name"] for step in path], ["sequent-core", "windmill", "harvest"]
        )

    def test_reads_reports_that_still_say_unlocked(self):
        legacy = [
            {key.replace("unblocked", "unlocked"): value for key, value in item.items()}
            for item in UNITS
        ]
        self.assertEqual([step["i"] for step in critical_path(legacy)], [0, 1, 2])

    def test_summary_names_units_and_path_length(self):
        summary = timings_summary(UNITS)
        self.assertEqual(summary["units_compiled"], 3)
        self.assertEqual(summary["critical_path_seconds"], 54.0)
        self.assertEqual(
            summary["longest_units"][0],
            {"unit": 'harvest bin "harvest"', "seconds": 30.0},
        )
        self.assertEqual(timings_summary([])["critical_path"], [])


class LinkerTest(unittest.TestCase):
    def test_times_the_configured_linker_instead_of_replacing_it_with_cc(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            linker = root / "selected-linker"
            linker.write_text("#!/bin/sh\nexit 37\n")
            linker.chmod(0o755)
            log = root / "links.jsonl"
            with (
                mock.patch.dict(
                    os.environ,
                    {"CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER": str(linker)},
                ),
                mock.patch.object(
                    rust,
                    "host_triple",
                    return_value="aarch64-unknown-linux-gnu",
                ),
            ):
                environment = build_environment(root / "target", log)
            with (
                mock.patch.dict(os.environ, environment),
                mock.patch("sys.argv", ["timed_linker", "-o", "bin/example"]),
            ):
                self.assertEqual(timed_linker.main(), 37)
            self.assertEqual(json.loads(log.read_text())["status"], 37)

    def test_missing_configured_linker_is_not_silently_replaced(self):
        with (
            mock.patch.dict(
                os.environ,
                {"CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER": "/missing/linker"},
            ),
            mock.patch.object(
                rust,
                "host_triple",
                return_value="aarch64-unknown-linux-gnu",
            ),
        ):
            with self.assertRaisesRegex(RuntimeError, "no executable linker"):
                build_environment(Path("target"), Path("links.jsonl"))

    def test_finds_the_output_in_arguments_and_response_files(self):
        self.assertEqual(
            timed_linker.output_of(["a.o", "-o", "out/harvest"]), "out/harvest"
        )
        with tempfile.TemporaryDirectory() as directory:
            response = Path(directory) / "args"
            response.write_text('"a.o"\n"-o"\n"out/windmill"\n')
            self.assertEqual(timed_linker.output_of([f"@{response}"]), "out/windmill")
        self.assertIsNone(timed_linker.output_of(["a.o"]))

    def test_records_the_link_and_keeps_the_linker_status(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "links.jsonl"
            environment = {
                "STEP_BENCH_REAL_LINKER": "false",
                "STEP_BENCH_LINK_LOG": str(log),
            }
            with (
                mock.patch.dict(os.environ, environment),
                mock.patch("sys.argv", ["timed_linker", "-o", "bin/x"]),
            ):
                self.assertEqual(timed_linker.main(), 1)
            record = json.loads(log.read_text())
            self.assertEqual((record["output"], record["status"]), ("bin/x", 1))


class WasmScriptTest(unittest.TestCase):
    def write_script(self, root, text):
        (root / ".devcontainer" / "scripts").mkdir(parents=True)
        (root / BUILD_SCRIPT).write_text(text)

    def test_retargets_the_build_script_at_the_checkout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_script(
                root, "TARGET_DIR=/workspaces/step/packages/sequent-core\ncd ..\n"
            )
            patched = patched_script(root, root / "build.sh").read_text()
        self.assertEqual(patched, f"TARGET_DIR={root}/packages/sequent-core\ncd ..\n")

    def test_current_wrapper_runs_from_its_checkout(self):
        wrapper = Path(__file__).resolve().parents[3] / BUILD_SCRIPT
        with tempfile.TemporaryDirectory(prefix="wasm checkout ") as directory:
            root = Path(directory)
            self.write_script(root, wrapper.read_text())
            entrypoint = root / "scripts/dev/step-dev"
            entrypoint.parent.mkdir(parents=True)
            entrypoint.write_text('#!/usr/bin/env bash\nprintf "%s\\n" "$@"\n')
            entrypoint.chmod(0o755)
            destination = root / "logs/build.sh"
            result = subprocess.run(
                default_build_command(root, destination),
                shell=True,
                capture_output=True,
                text=True,
                cwd=root.parent,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines(), ["wasm", "--release-package"])
            self.assertFalse(destination.exists())

    def test_legacy_wrapper_runs_with_the_measured_checkout(self):
        with tempfile.TemporaryDirectory(prefix="wasm checkout ") as directory:
            root = Path(directory)
            self.write_script(root, f'{SCRIPT}\nprintf "%s\\n" "$TARGET_DIR"\n')
            result = subprocess.run(
                default_build_command(root, root / "build.sh"),
                shell=True,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), str(root / "packages/sequent-core"))

    def test_refuses_scripts_with_other_workspace_paths(self):
        cases = {
            "cd /workspaces/step/packages\n": "no longer sets",
            f"{SCRIPT}\nrm -rf /workspaces/step/x\n": "still refers",
        }
        for text, message in cases.items():
            with self.subTest(text=text), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.write_script(root, text)
                with self.assertRaisesRegex(ValueError, message):
                    patched_script(root, root / "build.sh")

    def test_phases_come_from_the_first_trace_line_of_each_step(self):
        trace = (
            "+100.5 which rustc\n"
            "+101.0 wasm-pack build --mode no-install\n"
            "[INFO]: Compiling\n"
            "+160.0 wasm-pack -v pack .\n"
            "+170.0 awk -v hash=abc\n"
            "+171.0 cp sequent-core/pkg/a.tgz ./ui-core/rust/a.tgz\n"
            "+171.5 cp sequent-core/pkg/a.tgz ./admin-portal/rust/a.tgz\n"
            "+172.0 rm -rf node_modules ui-core/node_modules\n"
        )
        self.assertEqual(
            script_phases(trace, 100.0),
            {
                "wasm_pack_build": 1.0,
                "wasm_pack_pack": 60.0,
                "lock_update": 70.0,
                "archives_copied": 71.0,
                "node_modules_removed": 72.0,
            },
        )


if __name__ == "__main__":
    unittest.main()
