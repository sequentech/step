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
    def test_descriptive_names_and_numeric_aliases_resolve_identically(self):
        m = {'steps': [{'id': '04-check-secret-encryption', 'legacy_id': '04-005', 'roles': ['c1']}]}
        names = runner.step_entries(m)
        self.assertIs(names['04-check-secret-encryption'], names['04-005'])
        self.assertEqual(runner.numeric_id(names['04-check-secret-encryption']), '04-005')
        with self.assertRaises(ValueError):
            runner.step_entries({'steps': m['steps'] + [{'id': '04-005'}]})

    def test_destructive_confirmations_survive_readable_names(self):
        self.assertEqual(runner.confirmation_for({'id': '08-remove-trustee-keys', 'legacy_id': '08-014'}), 'REMOVE TRUSTEE KEYS')
        self.assertEqual(runner.confirmation_for({'id': '11-restore-authorized-backup', 'legacy_id': '11-003'}), 'RESTORE AUTHORIZED BACKUP')
        self.assertIsNone(runner.confirmation_for({'id': '04-check-secret-encryption', 'legacy_id': '04-005'}))


    def test_service_isolation_zero_public_routes_continues(self):
        command = (Path(__file__).parent/'commands/10-009.sh').read_text()
        with tempfile.TemporaryDirectory() as folder:
            f=Path(folder)/'body.sh';f.write_text(command)
            stub = r'''source "$1"
k3s(){
 case "$*" in
 'secrets-encrypt status') printf 'Encryption Status: Enabled\n';;
 *'get service b4'*) printf 'ClusterIP\n';;
 *'get ingress'*) printf '{"items": []}\n';;
 *'get service -A'*) printf '{"items": []}\n'; printf reached-service-check >&2;;
 esac
}
agd_source "$2"
'''
            result=subprocess.run(['bash','-c',stub,'test',str(Path(__file__).with_name('session.sh')),str(f)],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertIn('reached-service-check',result.stderr)
            self.assertIn('0',result.stdout)

    def test_excluded_portal_absence_is_readable_but_missing_file_stops(self):
        command = (Path(__file__).parent/'commands/07-014.sh').read_text()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'config').mkdir()
            f=root/'body.sh';f.write_text(command)
            config=root/'config/step-env.yaml';config.write_text('OTHER_SETTING: value\n')
            args=['bash','-c','source "$1"; agd_source "$2"','test',str(Path(__file__).with_name('session.sh').resolve()),str(f)]
            result=subprocess.run(args,cwd=root,capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertIn('Neither excluded-portal setting is present.',result.stdout)
            config.unlink()
            result=subprocess.run(args,cwd=root,capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertNotIn('Neither excluded-portal',result.stdout)

    def test_trustee_import_failure_preserves_temporary_key(self):
        import re
        source=(Path(__file__).parent/'commands/08-010.sh').read_text()
        remote=re.search(r'ssh root@\$C1 "(.*)"\s*$',source,re.S).group(1)
        for failure in [True,False]:
            code=('k3s(){ return '+('1' if failure else '0')+'; }; '
                  'shred(){ printf DELETED; }; N=1; '+remote)
            result=subprocess.run(['bash','-c',code],capture_output=True,text=True)
            if failure:
                self.assertNotEqual(result.returncode,0)
                self.assertNotIn('DELETED',result.stdout)
            else:
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertIn('Trustee key import complete; temporary key file removed.',result.stdout)

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

    def test_tenant_selection_prompts_after_listing_and_rejects_bad_uuid(self):
        original=(Path(__file__).parent/'commands/08-007.sh').read_text()
        tenant='01234567-89ab-cdef-0123-456789abcdef'
        for value in [tenant, 'not-a-uuid', '$(id)']:
            with self.subTest(value=value), tempfile.TemporaryDirectory() as folder:
                hosts=Path(folder)/'hosts.env';hosts.write_text('')
                f=Path(folder)/'body.sh';f.write_text(original.replace('/root/deploy/hosts.env',str(hosts)).replace('cd /root/deploy','cd '+folder))
                code='''source "$1"; k3s(){ printf "LISTED\\n"; }; agd_source "$2"; result=$?; printf "CR=%s\\n" "$CR"; exit "$result"'''
                result=subprocess.run(['bash','-c',code,'test',str(Path(__file__).with_name('session.sh')),str(f)],input=value+'\n',capture_output=True,text=True)
                self.assertIn('LISTED',result.stdout)
                if value == tenant:
                    self.assertEqual(result.returncode,0,result.stderr)
                    self.assertIn('export CR=tenant-'+tenant,hosts.read_text())
                    self.assertIn('CR=tenant-'+tenant,result.stdout)
                else:
                    self.assertNotEqual(result.returncode,0)
                    self.assertEqual(hosts.read_text(),'')

    def test_discovery_and_redirect_checks_select_realm_explicitly(self):
        for name in ['10-002.sh','10-003.sh']:
            original=(Path(__file__).parent/'commands'/name).read_text()
            for choice in ['', 'customer', 'invalid']:
                with self.subTest(name=name,choice=choice), tempfile.TemporaryDirectory() as folder:
                    f=Path(folder)/'body.sh'
                    setup='R=tenant-installation; CR=tenant-customer; PORTAL_HOST=portal.test; ADMIN_HOST=admin.test; VERIFIER_HOST=verifier.test'
                    f.write_text(original.replace('. /root/deploy/hosts.env',setup))
                    code='''source "$1"; curl(){ printf '%s\\n' "$*" >&2; printf '{}'; }; agd_source "$2"'''
                    result=subprocess.run(['bash','-c',code,'test',str(Path(__file__).with_name('session.sh')),str(f)],input=choice+'\n',capture_output=True,text=True)
                    if choice == 'invalid':
                        self.assertNotEqual(result.returncode,0)
                        self.assertNotIn('https://',result.stderr)
                    else:
                        self.assertEqual(result.returncode,0,result.stderr)
                        self.assertIn('/realms/tenant-'+('customer' if choice == 'customer' else 'installation'),result.stderr)

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
            self.assertEqual(len(m['steps']), 78)
            for e in m['steps']:
                self.assertRegex(e['id'], r'^[0-9]{2}-[a-z]+(?:-[a-z0-9]+)+$')
                modern=subprocess.run([str(out/'deploy'),'show',e['id']],capture_output=True,text=True)
                legacy=subprocess.run([str(out/'deploy'),'show',e['legacy_id']],capture_output=True,text=True)
                self.assertEqual(modern.returncode,0,modern.stderr)
                self.assertEqual(modern.stdout,legacy.stdout)
            result=subprocess.run([str(out/'deploy'),'configure','c1'],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('Test-only',result.stderr)
            (out/m['steps'][0]['file']).write_text('changed')
            result=subprocess.run([str(out/'deploy'),'check'],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)


if __name__ == "__main__": unittest.main()
