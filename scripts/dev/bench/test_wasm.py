# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import json
import shlex
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from . import common, wasm

FAKE_COMMAND = """
import json
import sys
from pathlib import Path

root, record = map(Path, sys.argv[1:3])
mode, restore_status = sys.argv[3:]
lock = root / 'packages/yarn.lock'
archive = root / 'packages/core.tgz'
if mode == 'install':
    restored = lock.read_bytes() == b'original lock\\n'
    with record.open('a') as output:
        output.write(json.dumps([lock.read_text(), archive.read_text()]) + '\\n')
lock.write_text('normalized lock\\n')
archive.write_text('generated archive\\n')
(root / 'packages/package.json').write_text('normalized manifest\\n')
if mode == 'install' and restored:
    print('restoring dependencies')
    sys.exit(int(restore_status))
"""


class WasmRestorationTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="wasm restoration ")
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.root = self.directory / "checkout"
        self.root.mkdir()
        (self.root / "packages").mkdir()
        self.original = {
            "packages/yarn.lock": b"original lock\n",
            "packages/core.tgz": b"original archive\n",
            "packages/package.json": b"original manifest\n",
            "notes.txt": b"original notes\n",
            "staged.txt": b"original staged\n",
        }
        for name, content in self.original.items():
            (self.root / name).write_bytes(content)
        self.git("init", "-q")
        self.git("add", ".")
        self.git(
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-qm",
            "initial",
        )
        (self.root / "notes.txt").write_bytes(b"unrelated local notes\n")
        (self.root / "staged.txt").write_bytes(b"unrelated staged notes\n")
        self.git("add", "staged.txt")
        (self.root / "untracked.txt").write_bytes(b"unrelated untracked notes\n")
        self.before = self.state()
        self.fake = self.directory / "fake command.py"
        self.fake.write_text(FAKE_COMMAND)
        self.record = self.directory / "install-observations.jsonl"
        for name in ("BackgroundProcess", "BrowserProbe", "wait_for_http"):
            patcher = mock.patch.object(wasm, name)
            patcher.start()
            self.addCleanup(patcher.stop)
        patcher = mock.patch.object(common, "tool_versions", return_value={})
        patcher.start()
        self.addCleanup(patcher.stop)

    def git(self, *args):
        return subprocess.run(
            ["git", "-C", str(self.root), *args],
            capture_output=True,
            text=True,
            check=True,
        ).stdout

    def state(self):
        return (
            self.git("status", "--porcelain"),
            self.git("diff", "--binary"),
            self.git("diff", "--cached", "--binary"),
        )

    def options(self, restore_status=0):
        command = [sys.executable, str(self.fake), str(self.root), str(self.record)]
        return wasm.WasmOptions(
            checkout=self.root,
            label="restoration",
            edit_name="no-change",
            edit=None,
            build=shlex.join([*command, "build", str(restore_status)]),
            install=shlex.join([*command, "install", str(restore_status)]),
            restart=wasm.ServerRestart.NEVER,
            target="voting",
            port=12345,
            samples=1,
            warmup=0,
            timeout=10,
            output_dir=self.directory / "results",
        )

    def assert_restored(self):
        self.assertEqual(self.state(), self.before)
        for name in (
            "packages/yarn.lock",
            "packages/core.tgz",
            "packages/package.json",
        ):
            self.assertEqual((self.root / name).read_bytes(), self.original[name])
        self.assertEqual(
            (self.root / "untracked.txt").read_bytes(), b"unrelated untracked notes\n"
        )

    def test_install_normalization_is_reverted_and_the_next_benchmark_can_run(self):
        for _ in range(2):
            result = json.loads(wasm.run_wasm(self.options()).read_text())
            self.assertEqual(result["summary"]["n"], 1)
            self.assertEqual(result["summary"]["failed"], 0)
            self.assert_restored()
        self.assertEqual(
            [json.loads(line) for line in self.record.read_text().splitlines()],
            [
                ["normalized lock\n", "generated archive\n"],
                ["original lock\n", "original archive\n"],
            ]
            * 2,
        )

    def test_failed_restore_install_is_reported_after_restoring_tracked_inputs(self):
        with self.assertRaisesRegex(RuntimeError, "restore install exited 23"):
            wasm.run_wasm(self.options(restore_status=23))
        self.assert_restored()
        logs = list((self.directory / "results").rglob("restore-install.log"))
        self.assertEqual(len(logs), 1)
        self.assertIn("restoring dependencies", logs[0].read_text())

    def test_dirty_packaging_inputs_are_refused_without_running_commands(self):
        for name in ("packages/yarn.lock", "packages/core.tgz"):
            with self.subTest(path=name):
                (self.root / name).write_bytes(b"pre-existing change\n")
                before = self.state()
                with self.assertRaisesRegex(ValueError, "files with local changes"):
                    wasm.run_wasm(self.options())
                self.assertEqual(self.state(), before)
                self.assertFalse(self.record.exists())
                wasm.BackgroundProcess.assert_not_called()
                (self.root / name).write_bytes(self.original[name])


if __name__ == "__main__":
    unittest.main()
