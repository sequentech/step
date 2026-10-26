# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Synthetic failure cases; no platform or network operations."""
import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
import runner
import build

class RunnerTests(unittest.TestCase):
    def test_public_values_refuse_shell_injection(self):
        for value in ["$(id)", "x;id", "x'", 'x"', "x`id`", "x\ny"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                runner.validate_value("issuer", value)
        self.assertEqual(runner.validate_value("issuer", "https://idp.example.test/realms/site"), "https://idp.example.test/realms/site")

    def test_wrong_machine_role_refused(self):
        with self.assertRaises(ValueError):
            runner.require_role({"roles": ["c1"]}, "admin")
        runner.require_role({"roles": ["c1", "c2"]}, "c2")

    def test_changed_or_symlink_script_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder); p = root / "one.sh"; p.write_text("true\n")
            digest = hashlib.sha256(p.read_bytes()).hexdigest()
            self.assertEqual(runner.checked_file(root, "one.sh", digest), p)
            p.write_text("false\n")
            with self.assertRaises(ValueError): runner.checked_file(root, "one.sh", digest)
            p.unlink(); p.symlink_to("/etc/hostname")
            with self.assertRaises(ValueError): runner.checked_file(root, "one.sh", digest)
            with self.assertRaises(ValueError): runner.checked_file(root, "../outside", digest)

    def test_unresolved_values_refused(self):
        with self.assertRaises(ValueError): runner.substitute("echo '<issuer>'\n", {})
        self.assertEqual(runner.substitute("echo '<issuer>'\n", {"issuer": "https://idp.test"}), "echo 'https://idp.test'\n")
        self.assertEqual(runner.substitute('cat < file > output\n', {}), 'cat < file > output\n')

    def test_indented_heredoc_and_data_are_preserved(self):
        text = "<!-- deployment-step: 08-007 -->\n   ```bash\n   cat <<'EOF'\n   hello\n   EOF\n   ```\n"
        rows = build.extract(text)
        self.assertEqual(rows[0]["body"], "cat <<'EOF'\nhello\nEOF\n")
        with self.assertRaises(ValueError): build.extract(text.replace("   ```\n", ""))

    def test_step_error_stops_following_command_and_session_survives(self):
        with tempfile.TemporaryDirectory() as folder:
            p = Path(folder) / "body.sh"; p.write_text("false\nprintf should-not-run\\n\n")
            script = "source \"$1\"\n_legacy_test(){ :; }\nagd_source \"$2\"\nprintf 'session-survives\\n'\n"
            result = subprocess.run(["bash", "--noprofile", "--norc", "-c", script, "test", str(Path(__file__).with_name("session.sh")), str(p)], capture_output=True, text=True)
            self.assertNotIn("should-not-run", result.stdout)
            self.assertIn("session-survives", result.stdout)
            self.assertIn("stopped", result.stderr)

    def test_pipeline_failure_stops_and_restores_options(self):
        for body in ["false | cat\nprintf BAD\n", "f(){ false; printf BAD; }; f\nprintf BAD\n", "(set -e; trap 'printf cleaned' EXIT; false)\nprintf BAD\n"]:
            with self.subTest(body=body), tempfile.TemporaryDirectory() as folder:
                f = Path(folder) / 'body.sh'; f.write_text(body)
                command = '''source "$1"; agd_source "$2"; printf "survived\\n"; [[ $- != *e* ]]; test "$(set -o | awk '/pipefail/ {print $2}')" = off'''
                result = subprocess.run(['bash','-c',command,'test',str(Path(__file__).with_name('session.sh')),str(f)],capture_output=True,text=True)
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertNotIn('BAD',result.stdout)
                self.assertIn('survived',result.stdout)
                if 'trap' in body: self.assertIn('cleaned',result.stdout)

    def test_success_preserves_variables_and_directory(self):
        with tempfile.TemporaryDirectory() as folder:
            f=Path(folder)/'body.sh';f.write_text('export EXAMPLE=kept\ncd /tmp\nset -o noclobber\n')
            result=subprocess.run(['bash','-c','''source "$1"; agd_source "$2"; test "$EXAMPLE" = kept; test "$PWD" = /tmp; test "$(set -o | awk '/noclobber/ {print $2}')" = off''','test',str(Path(__file__).with_name('session.sh')),str(f)],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)

    def test_restore_corrupt_backup_never_calls_ssh(self):
        original=(Path(__file__).parent/'commands/11-003.sh').read_text()
        with tempfile.TemporaryDirectory() as folder:
            backup=Path(folder)/'bad.tar.gpg';backup.write_bytes(b'bad')
            body=original.replace('. /root/deploy/hosts.env','C1=example.invalid').replace('<backup-medium>/<file-of-the-backup-to-restore>.tar.gpg',str(backup))
            f=Path(folder)/'body.sh';f.write_text(body)
            command='source "$1"; gpg(){ return 2; }; ssh(){ printf DESTRUCTIVE; }; agd_source "$2"; printf survived'
            result=subprocess.run(['bash','-c',command,'test',str(Path(__file__).with_name('session.sh')),str(f)],capture_output=True,text=True)
            self.assertNotIn('DESTRUCTIVE',result.stdout)
            self.assertIn('survived',result.stdout)

    def test_restore_unrelated_archive_refused_before_delete(self):
        import tarfile
        import io
        original=(Path(__file__).parent/'commands/11-003.sh').read_text()
        with tempfile.TemporaryDirectory() as folder:
            backup=Path(folder)/'unrelated.tar.gpg'
            with tarfile.open(backup,'w') as archive:
                item=tarfile.TarInfo('unrelated.txt');item.size=1;archive.addfile(item,io.BytesIO(b'x'))
            body=original.replace('. /root/deploy/hosts.env','C1=example.invalid').replace('<backup-medium>/<file-of-the-backup-to-restore>.tar.gpg',str(backup))
            f=Path(folder)/'body.sh';f.write_text(body)
            command='source "$1"; gpg(){ cat "$2"; }; ssh(){ printf DESTRUCTIVE; }; agd_source "$2"; printf survived'
            result=subprocess.run(['bash','-c',command,'test',str(Path(__file__).with_name('session.sh')),str(f)],capture_output=True,text=True)
            self.assertNotIn('DESTRUCTIVE',result.stdout)
            self.assertIn('survived',result.stdout)

    def test_native_trustee_number_and_c2_mapping(self):
        for value in ['0','3','one']:
            with self.assertRaises(ValueError): runner.validate_value('native-trustee-number',value)
        self.assertEqual(build.HOSTS['06-004'],['c2'])

    def test_bundle_relocatable_hashcheck_and_test_execution_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            repo=Path(__file__).resolve().parents[4]
            out=Path(folder)/'with spaces'/'deployment'
            m=build.build(repo,out,'0'*40,True)
            result=subprocess.run([str(out/'deploy'),'check'],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            result=subprocess.run([str(out/'deploy'),'configure','c1'],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('Test-only',result.stderr)
            (out/m['steps'][0]['file']).write_text('changed')
            result=subprocess.run([str(out/'deploy'),'check'],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)


if __name__ == "__main__": unittest.main()
