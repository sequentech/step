# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import io,json,shutil,tarfile,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import maintenance as m

class MaintenanceTests(unittest.TestCase):
    def row(self,phase='execution',safety='recoverable',generation=4):
        return dict(tenant_id='11111111-1111-4111-8111-111111111111',election_event_id='22222222-2222-4222-8222-222222222222',phase=phase,safety=safety,generation=generation)
    def test_restore_cannot_reopen_ended_voting_or_remove_event(self):
        old={'installation':{},'lifecycle':[self.row()]}
        for current in ([self.row('evaluation')],[self.row('post_processing')],[]):
            with self.assertRaises(ValueError):m.restore_compatible(old,current,{})
    def test_restore_cannot_clear_unrecoverable_state(self):
        with self.assertRaisesRegex(ValueError,'unrecoverable'):
            m.restore_compatible({'installation':{},'lifecycle':[self.row()]},[self.row(safety='unrecoverable')],{})
    def test_same_phase_restore_allows_old_generation_but_not_wrong_installation(self):
        old={'installation':{'release':1},'lifecycle':[self.row(phase='preparation',generation=1)]}
        self.assertEqual(len(m.restore_compatible(old,[self.row(phase='preparation')],{'release':1})),1)
        with self.assertRaises(ValueError):m.restore_compatible(old,[self.row(phase='preparation')],{'release':2})
    def test_generic_restore_cannot_discard_current_execution_records(self):
        with self.assertRaisesRegex(ValueError,'Execution'):
            m.restore_compatible({'installation':{},'lifecycle':[self.row()]},[self.row()],{})
    def test_duplicate_or_invalid_safety_record_refused(self):
        for rows in ([self.row(),self.row()],[self.row(generation=-1)],[self.row(phase='other')]):
            with self.assertRaises(ValueError):m.floors(rows)
    def test_encryption_failure_never_resumes_services(self):
        data={'nodes':{},'hosts':{},'release':{},'toe':{'manifest_sha256':'abc'}}
        with patch.object(m,'backup_destination',return_value=Path('/tmp/unused')),patch.object(m,'password',return_value='a secure example password'),patch.object(m,'confirm'),patch.object(m,'stop'),patch.object(m,'safety_record',return_value=[]),patch.object(m.install,'ssh'),patch.object(m,'encrypt_remote',side_effect=ValueError('transfer failed')),patch.object(m,'resume') as resume:
            with self.assertRaises(ValueError):m.backup(data)
            resume.assert_not_called()
    def test_resume_unchanged_excludes_restore_states(self):
        with patch.object(m.install,'ssh',side_effect=ValueError('refused')) as ssh:
            with self.assertRaises(ValueError):m.resume({})
        command=ssh.call_args.args[2]
        self.assertIn('stopped|stopping',command);self.assertNotIn('restored',command);self.assertIn('/claim/owner',command)
    def test_stop_api_failure_never_starts_backup(self):
        with patch.object(m.install,'ssh',side_effect=ValueError('API unavailable')) as ssh:
            with self.assertRaises(ValueError):m.stop({})
        command=ssh.call_args.args[2]
        self.assertLess(command.index('mkdir /var/lib/sequent-maintenance/claim'),command.index('scale "deployment/$name"'))
        self.assertLess(command.index('json_agg'),command.index('scale deployment/postgres'));self.assertNotIn('tar -',command)
    @unittest.skipUnless(shutil.which('gpg'),'GPG required')
    def test_real_encryption_roundtrip_and_tampering_refusal(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);plain=root/'fixture.tar';encrypted=root/'backup.tar.gpg';metadata={'installation':{'schema':1},'lifecycle':[self.row()]}
            with tarfile.open(plain,'w') as archive:
                for name in ['root/deploy','var/lib/sequent-data']:
                    item=tarfile.TarInfo(name);item.type=tarfile.DIRTYPE;archive.addfile(item)
                body=json.dumps(metadata).encode();item=tarfile.TarInfo('root/deploy/.backup-identity.json');item.size=len(body);archive.addfile(item,io.BytesIO(body))
            secret='distinct test backup passphrase'
            with patch.object(m,'ssh_command',return_value=['cat',str(plain)]):digest=m.encrypt_remote({},'',encrypted,passphrase=secret)
            self.assertEqual(digest,m.install.digest(encrypted));self.assertEqual(m.verify_archive(encrypted,'system',secret),metadata)
            self.assertNotIn(secret.encode(),encrypted.read_bytes())
            corrupt=bytearray(encrypted.read_bytes());corrupt[-25]^=1;encrypted.write_bytes(corrupt)
            with self.assertRaises((ValueError,tarfile.TarError)):m.verify_archive(encrypted,'system',secret)
    def test_restart_claim_prevents_overlap_with_backup(self):
        import subprocess
        with tempfile.TemporaryDirectory() as folder:
            state=Path(folder)/'state';claim=state/'claim';claim.mkdir(parents=True);(claim/'owner').write_text('system')
            def execute(data,node,script,label,timeout):
                script=script.replace(m.STATE,str(state)).replace('/root/deploy',folder)
                result=subprocess.run(['bash','-c','set -Eeuo pipefail\nk3s(){ echo UNSAFE_RESTART; };\n'+script],capture_output=True,text=True)
                self.assertNotEqual(result.returncode,0);self.assertNotIn('UNSAFE_RESTART',result.stdout)
                self.assertEqual((claim/'owner').read_text(),'system');raise ValueError('Existing maintenance owner preserved')
            with patch.object(m,'load',return_value={}),patch.object(m,'confirm'),patch.object(m.install,'ssh',side_effect=execute):
                with self.assertRaises(ValueError):m.main('services','site.json','restart','keycloak')
    def test_resume_refuses_backup_stream_in_progress(self):
        import subprocess
        with tempfile.TemporaryDirectory() as folder:
            state=Path(folder);(state/'claim').mkdir();(state/'claim/owner').write_text('system');(state/'state.json').write_text('{"phase":"backing-up"}')
            def execute(data,node,script,label,timeout):
                script=script.replace(m.STATE,folder).replace('/root/deploy',folder)
                result=subprocess.run(['bash','-c','set -Eeuo pipefail\nk3s(){ echo UNSAFE_RESUME; };\n'+script],capture_output=True,text=True)
                self.assertNotEqual(result.returncode,0);self.assertNotIn('UNSAFE_RESUME',result.stdout);raise ValueError('Active encrypted transfer preserved')
            with patch.object(m.install,'ssh',side_effect=execute):
                with self.assertRaises(ValueError):m.resume({})
    def test_cipher_start_failure_terminates_remote_reader(self):
        from unittest.mock import MagicMock
        remote=MagicMock()
        with tempfile.TemporaryDirectory() as folder,patch.object(m,'ssh_command',return_value=['ssh']),patch.object(m.subprocess,'Popen',return_value=remote),patch.object(m,'gpg_process',side_effect=OSError('GPG start refused')):
            with self.assertRaises(OSError):m.encrypt_remote({},'',Path(folder)/'backup.tar.gpg',passphrase='test backup passphrase')
            remote.kill.assert_called_once();remote.wait.assert_called_once();remote.stdout.close.assert_called_once()
    def test_named_stop_typo_cannot_stop_all_services(self):
        with patch.object(m,'load',return_value={}),patch.object(m,'confirm'),patch.object(m,'stop') as stop:
            with self.assertRaisesRegex(ValueError,'component'):m.main('services','site.json','stop','keycloak')
            stop.assert_not_called()
    def test_passphrase_never_process_argument(self):
        with patch.object(m.subprocess,'Popen') as spawn:m.gpg_process(['--decrypt'],'private test passphrase',stdout=None)
        self.assertNotIn('private test passphrase',' '.join(spawn.call_args.args[0]));self.assertIn('--passphrase-fd',spawn.call_args.args[0])
if __name__=='__main__':unittest.main()
