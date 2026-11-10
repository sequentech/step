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
    def test_service_isolation_zero_public_routes_continues(self):
        import install
        command = install.command_source('10-009', {'operator': {'service_administrator': '', 'username': '', 'email': ''}})
        # The platform inspection runs the operation as install.ssh does: one bash script with this prefix.
        stub = r'''set -Eeuo pipefail
set +x
umask 077
k3s(){
 case "$*" in
 'secrets-encrypt status') printf 'Encryption Status: Enabled\n';;
 *'get service b4'*) printf 'ClusterIP\n';;
 *'get ingress'*) printf '{"items": []}\n';;
 *'get service -A'*) printf '{"items": []}\n'; printf reached-service-check >&2;;
 esac
}
''' + command
        result=subprocess.run(['bash','-c',stub],capture_output=True,text=True)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertIn('reached-service-check',result.stderr)
        self.assertIn('0',result.stdout)

    def test_excluded_portal_absence_is_readable_but_missing_file_stops(self):
        command = (Path(__file__).parent/'commands/07-014.sh').read_text()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'config').mkdir()
            f=root/'body.sh';f.write_text(command)
            config=root/'config/step-env.yaml';config.write_text('OTHER_SETTING: value\n')
            # The installation script sources each operation in one shell with these options.
            args=['bash','-c','set -Eeuo pipefail; source "$1"; printf after-operation','test',str(f)]
            result=subprocess.run(args,cwd=root,capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertIn('Neither excluded-portal setting is present.',result.stdout)
            self.assertIn('after-operation',result.stdout)
            config.unlink()
            result=subprocess.run(args,cwd=root,capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertNotIn('Neither excluded-portal',result.stdout)
            self.assertNotIn('after-operation',result.stdout)

    def test_backup_quiescence_propagates_remote_failures(self):
        from unittest.mock import patch
        import maintenance
        for failed in ['scale', 'get pods']:
            with self.subTest(failed=failed), tempfile.TemporaryDirectory() as folder:
                root=Path(folder);deploy=root/'deploy';deploy.mkdir();state=root/'maintenance'
                def execute(data,node,script,label,timeout):
                    body=script.replace('/root/deploy',str(deploy)).replace('/var/lib/sequent-maintenance',str(state))
                    stub='''set -Eeuo pipefail
k3s(){
 case "$*" in
 'kubectl get deployments -n step-apps -o json') printf '{"items":[{"metadata":{"name":"harvest","namespace":"step-apps"},"spec":{"replicas":2}}]}' ;;
 'kubectl get deployments -n step-infra -o json') printf '{"items":[{"metadata":{"name":"postgres","namespace":"step-infra"},"spec":{"replicas":1}}]}' ;;
 *"$FAILED"*) printf 'REACHED EXPECTED FAILURE: %s\\n' "$FAILED" >&2; return 3;;
 *"scale deployment"*) :;;
 *) printf 'Unexpected mock call: %s\\n' "$*" >&2; return 42;;
 esac
}
export FAILED="$1"
''' + body
                    result=subprocess.run(['bash','-c',stub,'test',failed],capture_output=True,text=True)
                    self.assertNotEqual(result.returncode,0)
                    self.assertIn('REACHED EXPECTED FAILURE: '+failed,result.stderr)
                    self.assertNotIn('APPLICATIONS STOPPED',result.stdout)
                    self.assertTrue((deploy/'.backup-in-progress').exists())
                    recorded=json.loads((state/'replicas.json').read_text())
                    self.assertEqual({item['name']:item['replicas'] for item in recorded},{'harvest':2,'postgres':1})
                    raise ValueError('Actual generated shutdown script refused unsafe continuation')
                with patch('maintenance.install.ssh',side_effect=execute):
                    with self.assertRaises(ValueError):maintenance.stop({})

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

    def test_indented_heredoc_and_data_are_preserved(self):
        text = "<!-- deployment-step: 08-007 -->\n   ```bash\n   cat <<'EOF'\n   hello\n   EOF\n   ```\n"
        rows = build.extract(text)
        self.assertEqual(rows[0]["body"], "cat <<'EOF'\nhello\nEOF\n")
        with self.assertRaises(ValueError): build.extract(text.replace("   ```\n", ""))

    def _restore_rejection(self, backup_content, encrypt=False):
        from unittest.mock import patch
        import maintenance
        import os,time,tarfile
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);selected=root/'bad.tar.gpg';selected.write_bytes(backup_content)
            secret='temporary real encryption test passphrase'
            if encrypt:
                with selected.open('wb') as output:
                    process=maintenance.gpg_process(['--symmetric','--cipher-algo','AES256','--force-mdc'],secret,stdin=subprocess.PIPE,stdout=output,stderr=subprocess.PIPE)
                    _,errors=process.communicate(backup_content,timeout=10)
                    self.assertEqual(process.returncode,0,errors)
            current=root/'current.tar.gpg';current.write_bytes(b'current backup fixture')
            data={'nodes':{},'hosts':{},'release':{},'toe':{'manifest_sha256':'a'*64}}
            (root/'.backup-receipt.json').write_text(json.dumps({'archive':str(current),'sha256':maintenance.install.digest(current),'installation':maintenance.identity(data),'created_at':int(time.time())}))
            previous=Path.cwd();os.chdir(root)
            try:
                with patch('builtins.input',return_value=str(selected)),patch('maintenance.password',return_value=secret),patch('maintenance.confirm'),patch('maintenance.stop') as stop,patch('maintenance.install.ssh') as ssh:
                    with self.assertRaises((ValueError,tarfile.TarError)):maintenance.restore(data)
                    stop.assert_not_called();ssh.assert_not_called()
            finally:os.chdir(previous)

    def test_restore_corrupt_backup_never_calls_ssh(self):
        self._restore_rejection(b'bad')

    def test_restore_unrelated_archive_refused_before_delete(self):
        import tarfile
        import io
        content=io.BytesIO()
        with tarfile.open(fileobj=content,mode='w') as archive:
            item=tarfile.TarInfo('unrelated.txt');item.size=1;archive.addfile(item,io.BytesIO(b'x'))
        self._restore_rejection(content.getvalue(),encrypt=True)

    def test_bundle_relocatable_hashcheck_and_test_execution_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            repo=Path(__file__).resolve().parents[4]
            out=Path(folder)/'with spaces'/'deployment'
            m=build.build(repo,out,'0'*40,True)
            result=subprocess.run([str(out/'deploy'),'check'],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual({e['legacy_id'] for e in m['steps']}, build.HELPER_STEPS)
            self.assertEqual(sorted(p.stem for p in (out/'steps').iterdir()), sorted(build.HELPER_STEPS))
            self.assertFalse((out/'session.sh').exists())
            for e in m['steps']:
                self.assertRegex(e['id'], r'^[0-9]{2}-[a-z]+(?:-[a-z0-9]+)+$')
            result=subprocess.run([str(out/'deploy'),'install','site.json'],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('Test-only',result.stderr)
            (out/m['steps'][0]['file']).write_text('changed')
            result=subprocess.run([str(out/'deploy'),'check'],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)

    def test_the_helpers_offer_only_the_operations_of_the_guide(self):
        with tempfile.TemporaryDirectory() as folder:
            out=Path(folder)/'deployment'
            build.build(Path(__file__).resolve().parents[4],out,'0'*40,True)
            for argv in (['show','08-015'],['list'],['session','admin'],['configure','c1'],['prepare','08-015'],['08-015']):
                with self.subTest(argv=argv):
                    result=subprocess.run([str(out/'deploy')]+argv,capture_output=True,text=True)
                    self.assertEqual(result.returncode,2,result.stderr)
                    self.assertIn('invalid choice',result.stderr)
            result=subprocess.run([str(out/'deploy'),'backup'],capture_output=True,text=True)
            self.assertEqual((result.returncode,result.stderr),(1,'Stopped: Provide the configuration path: ./deploy backup site.json\n'))


class HelperOperationTests(unittest.TestCase):
    HERE = Path(__file__).resolve().parent

    def used_by_the_helpers(self):
        import re
        used=set()
        for name in ('install.py','commission.py','verification.py','inspection.py','maintenance.py','trustee.py'):
            text=(self.HERE/name).read_text()
            used|=set(re.findall(r"(?:command_source|source)\('([0-9]{2}-[0-9]{3})'",text))
            for group in re.findall(r"for ident in \(((?:'[0-9]{2}-[0-9]{3}',?)+)\)",text):
                used|=set(re.findall(r"[0-9]{2}-[0-9]{3}",group))
        return used

    def test_the_bundle_carries_exactly_the_operations_the_helpers_run(self):
        self.assertEqual(self.used_by_the_helpers(), build.HELPER_STEPS)
        self.assertEqual(sorted(p.stem for p in (self.HERE/'commands').iterdir()), sorted(build.HELPER_STEPS))

    def test_the_guide_marks_each_helper_operation_once(self):
        import re
        guide=self.HERE.parents[2]/build.GUIDE.relative_to('docs')
        marked=re.findall(r'<!-- deployment-step: ([0-9]{2}-[0-9]{3}) -->',''.join(p.read_text() for p in sorted(guide.glob('*.md'))))
        for ident in sorted(build.HELPER_STEPS):
            with self.subTest(ident=ident):
                self.assertEqual(marked.count(ident),1)


if __name__ == "__main__": unittest.main()
