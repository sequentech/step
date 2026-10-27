# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Real controlling-terminal checks for operator decisions; no platform actions."""
import errno
import os
from pathlib import Path
import pty
import select
import signal
import subprocess
import sys
import time
import unittest


class TerminalTests(unittest.TestCase):
    def terminal(self, expression, prompt, response):
        pid, descriptor = pty.fork()
        if pid == 0:
            # The decision must come from the controlling terminal, not stdin.
            with open(os.devnull, 'rb') as null:
                os.dup2(null.fileno(), 0)
            code = 'import sys; sys.path.insert(0, ' + repr(str(Path(__file__).parent)) + '); ' + expression
            os.execv(sys.executable, [sys.executable, '-c', code])
        output = bytearray(); sent = False; status = None
        try:
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                ready, _, _ = select.select([descriptor], [], [], 0.1)
                if ready:
                    try:
                        chunk = os.read(descriptor, 4096)
                    except OSError as error:
                        if error.errno != errno.EIO:
                            raise
                        chunk = b''
                    if chunk:
                        output.extend(chunk)
                    if not sent and prompt.encode() in output:
                        os.write(descriptor, response.encode() + b'\n'); sent = True
                ended, value = os.waitpid(pid, os.WNOHANG)
                if ended:
                    status = value
                    break
            self.assertIsNotNone(status, 'Operator prompt did not finish within five seconds')
            return os.waitstatus_to_exitcode(status), output.decode(errors='replace'), sent
        finally:
            os.close(descriptor)
            if status is None:
                os.kill(pid, signal.SIGKILL)
                os.waitpid(pid, 0)

    def test_trustee_confirmation_accepts_exact_phrase_on_real_terminal(self):
        code, text, sent = self.terminal("import trustee; trustee.confirm('REMOVE TRUSTEE KEYS', 'Retirement fixture'); print('ACCEPTED')", 'Type REMOVE TRUSTEE KEYS: ', 'REMOVE TRUSTEE KEYS')
        self.assertEqual(code, 0, text)
        self.assertTrue(sent)
        self.assertIn('ACCEPTED', text)

    def test_trustee_confirmation_rejects_wrong_phrase_on_real_terminal(self):
        code, text, sent = self.terminal("import trustee; trustee.confirm('REMOVE TRUSTEE KEYS', 'Retirement fixture'); print('ACCEPTED')", 'Type REMOVE TRUSTEE KEYS: ', 'yes')
        self.assertTrue(sent, text)
        self.assertNotEqual(code, 0)
        self.assertIn('Operation cancelled; no automatic confirmation', text)
        self.assertNotIn('ACCEPTED\r\n', text)

    def test_runner_prompt_reads_terminal_with_redirected_stdin(self):
        code, text, sent = self.terminal("import runner; assert runner.prompt('Host', 'default') == 'chosen'; print('ACCEPTED')", 'Host [default]: ', 'chosen')
        self.assertEqual(code, 0, text)
        self.assertTrue(sent)
        self.assertIn('ACCEPTED', text)

    def test_runner_prompt_keeps_explicit_default_on_empty_terminal_response(self):
        code, text, sent = self.terminal("import runner; assert runner.prompt('Host', 'default') == 'default'; print('ACCEPTED')", 'Host [default]: ', '')
        self.assertEqual(code, 0, text)
        self.assertTrue(sent)

    def test_piped_phrase_cannot_authorize_without_controlling_terminal(self):
        result = subprocess.run([sys.executable, '-c', "import trustee; trustee.confirm('REMOVE TRUSTEE KEYS', 'Retirement fixture'); print('ACCEPTED')"], cwd=Path(__file__).parent, input='REMOVE TRUSTEE KEYS\n', text=True, capture_output=True, start_new_session=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('ACCEPTED', result.stdout)


if __name__ == '__main__': unittest.main()
