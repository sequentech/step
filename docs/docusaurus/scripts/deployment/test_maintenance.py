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
    # A fake k3s for the maintenance scripts. Each wait of the scripts sleeps
    # two seconds; this sleep moves the shell's clock by SLEEP_SECONDS instead.
    FAKE_K3S=r"""
k3s(){
  printf '%s\n' "$*" >> "$CALLS"
  polls=$(grep -c 'get pods' "$CALLS" || true)
  case "$*" in
    'kubectl get deployments -n step-apps -o json') echo '{"items":[{"metadata":{"namespace":"step-apps","name":"windmill"},"spec":{"replicas":2}}]}';;
    'kubectl get deployments -n step-infra -o json') echo '{"items":[{"metadata":{"namespace":"step-infra","name":"postgres"},"spec":{"replicas":1}}]}';;
    'kubectl get pods -A -o json')
      if grep -q 'scale deployment/postgres' "$CALLS"; then echo '{"items":[]}'
      elif test "$polls" -le "$BUSY_POLLS"; then echo '{"items":[{"metadata":{"namespace":"step-apps","labels":{"app":"windmill"}},"status":{"phase":"Running"}},{"metadata":{"namespace":"step-infra","labels":{"app":"postgres"}},"status":{"phase":"Running"}}]}'
      else echo '{"items":[{"metadata":{"namespace":"step-infra","labels":{"app":"postgres"}},"status":{"phase":"Running"}}]}'; fi;;
    *'get pods -l '*)
      if test "$polls" -le "$BUSY_POLLS"; then echo '{"items":[{"metadata":{"deletionTimestamp":"2026-11-10T00:00:00Z"}},{"metadata":{}}]}'
      else echo '{"items":[{"metadata":{}},{"metadata":{}}]}'; fi;;
    *' exec '*) echo '[]';;
  esac
}
sleep(){ SECONDS=$((SECONDS+$1*SLEEP_SECONDS/2)); }
"""
    def run_script(self,folder,script,busy_polls):
        import os,subprocess
        root=Path(folder);(root/'deploy').mkdir(exist_ok=True)
        script=script.replace(m.STATE,str(root/'state')).replace('/root/deploy',str(root/'deploy'))
        environment=dict(os.environ,CALLS=str(root/'calls'),BUSY_POLLS=str(busy_polls),SLEEP_SECONDS='110')
        return subprocess.run(['bash','-c','set -Eeuo pipefail\n'+self.FAKE_K3S+script],capture_output=True,text=True,env=environment)
    @unittest.skipUnless(shutil.which('jq'),'jq required')
    def test_stop_lets_running_tasks_finish_but_not_forever(self):
        # Six polls, 110 seconds apart, are the task workers' 660-second grace.
        for busy,stops in ((6,True),(100,False)):
            with self.subTest(busy=busy),tempfile.TemporaryDirectory() as folder:
                def execute(data,node,script,label,timeout):
                    self.assertGreater(timeout,m.STOP_SECONDS+m.DATABASE_STOP_SECONDS)
                    result=self.run_script(folder,script,busy)
                    if result.returncode:raise ValueError('stopped with '+str(result.returncode))
                    return result.stdout.encode()
                with patch.object(m.install,'ssh',side_effect=execute),patch('builtins.print'):
                    if stops:m.stop({})
                    else:self.assertRaises(ValueError,m.stop,{})
                calls=(Path(folder)/'calls').read_text()
                self.assertEqual('scale deployment/postgres' in calls,stops)
                self.assertEqual(json.loads((Path(folder)/'state/state.json').read_text())['phase'],'stopped' if stops else 'stopping')
    @unittest.skipUnless(shutil.which('jq'),'jq required')
    def test_restart_holds_the_claim_until_the_replaced_pods_have_stopped(self):
        for busy,restarts in ((6,True),(100,False)):
            with self.subTest(busy=busy),tempfile.TemporaryDirectory() as folder:
                def execute(data,node,script,label,timeout):
                    self.assertGreater(timeout,m.READY_SECONDS+m.STOP_SECONDS)
                    result=self.run_script(folder,script,busy)
                    self.assertFalse((Path(folder)/'state/claim').exists())
                    if result.returncode:raise ValueError('restart failed')
                    return b''
                with patch.object(m,'load',return_value={}),patch.object(m,'confirm'),patch.object(m.install,'ssh',side_effect=execute),patch('builtins.print') as printed:
                    if restarts:m.main('services','site.json','restart','windmill')
                    else:self.assertRaises(ValueError,m.main,'services','site.json','restart','windmill')
                calls=(Path(folder)/'calls').read_text().splitlines()
                self.assertEqual(calls[:2],['kubectl -n step-apps rollout restart deployment/windmill','kubectl -n step-apps rollout status deployment/windmill --timeout='+str(m.READY_SECONDS)+'s'])
                self.assertEqual(len(calls)-2,7 if restarts else 8)
                self.assertEqual(any('COMPONENT READY' in str(c) for c in printed.call_args_list),restarts)
    def test_logs_show_every_pod_of_a_component(self):
        for component in ('harvest','windmill','keycloak','tls-endpoint'):
            with patch.object(m,'load',return_value={}),patch.object(m.install,'ssh',return_value=b'') as ssh,patch('builtins.print'):
                m.main('services','site.json','logs',component)
            self.assertEqual(ssh.call_args.args[2],'k3s kubectl -n '+m.SERVICES[component]+' logs -l app='+component+' --prefix --tail=20')
    def test_the_tls_endpoint_restarts_in_its_namespace_and_says_that_port_443_is_closed_meanwhile(self):
        self.assertEqual(m.SERVICES['tls-endpoint'],'step-infra')
        with patch.object(m,'load',return_value={}),patch.object(m,'confirm'),patch.object(m.install,'ssh',return_value=b'') as ssh,patch('builtins.print') as printed:
            m.main('services','site.json','restart','tls-endpoint')
        script=ssh.call_args.args[2]
        self.assertIn('k3s kubectl -n step-infra rollout restart deployment/tls-endpoint\n',script)
        self.assertIn('test ! -e '+m.STATE+'/state.json\n',script)
        self.assertIn('nothing answers on port 443 meanwhile',str(printed.call_args_list[0]))
    def test_a_trustee_restart_says_that_it_stops_before_it_starts_again(self):
        for component in ('trustee1','trustee2'):
            with patch.object(m,'load',return_value={}),patch.object(m,'confirm'),patch.object(m.install,'ssh',return_value=b'') as ssh,patch('builtins.print') as printed:
                m.main('services','site.json','restart',component)
            self.assertIn('k3s kubectl -n step-infra rollout restart deployment/'+component+'\n',ssh.call_args.args[2])
            self.assertIn('it finishes its current cycle and stops, then starts again',str(printed.call_args_list[0]))
class TlsRenewalTests(unittest.TestCase):
    # Fakes for the renewal script on C1: the TLS endpoint serves its
    # certificate file once reloaded, Traefik the Secret once replaced; a
    # fingerprint is a SHA-256 of the certificate; each wait moves the shell's
    # clock by 30 seconds.
    FAKES=r"""
k3s(){
  printf '%s\n' "$*" >> "$CALLS"
  case "$*" in
    *'nginx -t -q') return "$NGINX_TEST";;
    *'nginx -s reload') if test "$RELOADS" = 1; then cp "$SITE/tls.crt" "$SERVED/443"; fi;;
    *' create secret tls step-tls-cert '*) for a in "$@"; do case "$a" in --cert=*) printf '%s\n' "${a#--cert=}";; esac; done;;
    *' replace -f -') read -r cert; cp "$cert" "$SERVED/10443";;
  esac
}
openssl(){
  case "$1" in
    x509) sha256sum | cut -d' ' -f1;;
    s_client) cat "$SERVED/${3##*:}" 2>/dev/null || true;;
  esac
}
sleep(){ SECONDS=$((SECONDS+30)); }
"""
    OLD={'tls.crt':b'OLD CERTIFICATE\n','tls.key':b'OLD KEY\n','ca.pem':b'SITE CA\n'}
    def setUp(self):
        import tempfile as t
        self.folder=t.TemporaryDirectory();self.root=Path(self.folder.name)
        self.site=self.root/'site';self.deploy=self.root/'deploy';self.served=self.root/'served';self.state=self.root/'state'
        for directory in (self.site,self.deploy/'tls',self.served):directory.mkdir(parents=True)
        for directory in (self.site,self.deploy/'tls'):
            for name,body in self.OLD.items():(directory/name).write_bytes(body)
        # The modes that 05-005 and the installer's transfer give them.
        for name in self.OLD:(self.site/name).chmod(0o600 if name=='tls.key' else 0o644);(self.deploy/'tls'/name).chmod(0o600)
        (self.deploy/'hosts.env').write_text('export PORTAL_HOST=vote.example\nexport ADMIN_HOST=admin.example\nexport VERIFIER_HOST=verify.example\n')
        for port in ('443','10443'):(self.served/port).write_bytes(self.OLD['tls.crt'])
        (self.root/'new.crt').write_bytes(b'NEW CERTIFICATE\n');(self.root/'new.key').write_bytes(b'NEW KEY\n')
        self.calls=self.root/'calls';self.transfers=[]
    def tearDown(self):self.folder.cleanup()
    def renew(self,ca=b'SITE CA\n',nginx_test=0,reloads=1):
        import shlex
        (self.root/'ca.pem').write_bytes(ca)
        data={'ssh_identity':'/id','nodes':{'c1':'192.0.2.1'},'tls':{'certificate':str(self.root/'new.crt'),'private_key':str(self.root/'new.key'),'ca':str(self.root/'ca.pem')}}
        real=m.install.run
        def run(argv,label,stdin=None,timeout=120):
            if argv[0]=='openssl':return b'notAfter=May 30 12:00:00 2027 GMT\n'
            self.transfers.append((argv,stdin))
            body=shlex.split(argv[-1])[0].replace(m.STATE,str(self.state)).replace(m.SITE_TLS,str(self.site)).replace('/root/deploy',str(self.deploy))
            values={'CALLS':self.calls,'SITE':self.site,'SERVED':self.served,'NGINX_TEST':nginx_test,'RELOADS':reloads}
            return real(['bash','-c',''.join(k+'='+shlex.quote(str(v))+'\n' for k,v in values.items())+self.FAKES+body],label,stdin,timeout)
        with patch.object(m.install,'run',side_effect=run),patch.object(m,'confirm') as confirm,patch('builtins.print') as printed:
            try:m.renew_tls(data)
            finally:self.printed=' '.join(str(c) for c in printed.call_args_list)
        confirm.assert_called_once_with('RENEW TLS CERTIFICATE')
    def calls_made(self):
        return self.calls.read_text().splitlines() if self.calls.exists() else []
    def files(self,directory):
        return {name:((directory/name).read_bytes(),(directory/name).stat().st_mode&0o777) for name in ('tls.crt','tls.key','ca.pem')}
    def assert_released(self):
        self.assertFalse((self.state/'claim').exists())
        self.assertFalse((self.deploy/'.tls-renewal').exists())
    def test_the_endpoint_reloads_the_new_files_before_both_secrets_are_replaced(self):
        self.renew()
        self.assertEqual(self.files(self.site),{'tls.crt':(b'NEW CERTIFICATE\n',0o644),'tls.key':(b'NEW KEY\n',0o600),'ca.pem':(b'SITE CA\n',0o644)})
        self.assertEqual({n:v for n,v in self.files(self.deploy/'tls').items() if n!='ca.pem'},{'tls.crt':(b'NEW CERTIFICATE\n',0o600),'tls.key':(b'NEW KEY\n',0o600)})
        calls=self.calls_made();renewal=str(self.deploy/'.tls-renewal')
        self.assertEqual(calls[:3],['kubectl -n step-infra rollout status deployment/tls-endpoint --timeout=10s','kubectl -n step-infra exec deploy/tls-endpoint -- nginx -t -q','kubectl -n step-infra exec deploy/tls-endpoint -- nginx -s reload'])
        self.assertEqual(sorted(calls[3:]),sorted(['kubectl -n '+ns+' '+call for ns in ('step-apps','step-infra') for call in ('create secret tls step-tls-cert --cert='+renewal+'/tls.crt --key='+renewal+'/tls.key --dry-run=client -o yaml','replace -f -')]))
        self.assertNotIn('delete',' '.join(calls))
        self.assert_released()
        self.assertIn('TLS CERTIFICATE RENEWED',self.printed);self.assertIn('May 30 12:00:00 2027 GMT',self.printed)
    def test_the_key_travels_on_standard_input_only_and_the_ca_file_stays(self):
        self.renew()
        [(argv,stdin)]=self.transfers
        self.assertNotIn('NEW KEY',' '.join(argv))
        with tarfile.open(fileobj=io.BytesIO(stdin)) as archive:
            self.assertEqual(archive.getnames(),['tls.crt','tls.key'])
            self.assertEqual(archive.extractfile('tls.key').read(),b'NEW KEY\n')
    def test_a_different_ca_file_changes_nothing(self):
        with self.assertRaises(ValueError) as refused:self.renew(ca=b'OTHER CA\n')
        self.assertIn('renews only a certificate from the same CA file; nothing was changed',str(refused.exception))
        self.assertEqual(self.calls_made(),[])
        for directory in (self.site,self.deploy/'tls'):self.assertEqual({n:v[0] for n,v in self.files(directory).items()},self.OLD)
        self.assert_released()
    def test_files_the_endpoint_refuses_are_put_back_and_nothing_else_changes(self):
        with self.assertRaises(ValueError) as refused:self.renew(nginx_test=1)
        self.assertIn('the previous files are back in place',str(refused.exception))
        self.assertEqual(self.files(self.site),{'tls.crt':(b'OLD CERTIFICATE\n',0o644),'tls.key':(b'OLD KEY\n',0o600),'ca.pem':(b'SITE CA\n',0o644)})
        self.assertEqual({n:v[0] for n,v in self.files(self.deploy/'tls').items()},self.OLD)
        self.assertEqual(self.calls_made()[-1],'kubectl -n step-infra exec deploy/tls-endpoint -- nginx -t -q')
        self.assert_released()
    def test_until_every_host_gets_the_new_certificate_the_installation_copies_stay(self):
        with self.assertRaises(ValueError) as refused:self.renew(reloads=0)
        self.assertIn('within '+str(m.SERVED_SECONDS)+' seconds; the installation copies are unchanged and the command can be repeated',str(refused.exception))
        self.assertEqual({n:v[0] for n,v in self.files(self.deploy/'tls').items()},self.OLD)
        self.assert_released()
        self.renew()
        self.assertEqual((self.deploy/'tls'/'tls.crt').read_bytes(),b'NEW CERTIFICATE\n')
    def test_no_renewal_during_other_maintenance(self):
        for blocker in ('claim','stopped','trustee backup'):
            with self.subTest(blocker=blocker):
                self.tearDown();self.setUp()
                if blocker=='claim':(self.state/'claim').mkdir(parents=True);(self.state/'claim'/'owner').write_text('system')
                if blocker=='stopped':self.state.mkdir();(self.state/'state.json').write_text('{"phase":"stopped"}\n')
                if blocker=='trustee backup':(self.deploy/'.trustee-backup-in-progress').write_text('')
                with self.assertRaises(ValueError):self.renew()
                self.assertEqual(self.calls_made(),[])
                self.assertEqual((self.site/'tls.crt').read_bytes(),self.OLD['tls.crt'])
                if blocker=='claim':self.assertEqual((self.state/'claim'/'owner').read_text(),'system')
                else:self.assertFalse((self.state/'claim').exists())
    def test_renew_tls_is_a_deploy_command_that_checks_the_inputs_first(self):
        import runner
        self.assertIn('renew-tls',runner.OPERATIONS)
        with patch('sys.argv',['deploy','renew-tls','site.json']),patch('runner.manifest',return_value={}),patch.object(m,'main') as main:
            runner.main()
        main.assert_called_once_with('renew-tls','site.json',None,None)
        order=[]
        with patch.object(m,'load',return_value={}),patch.object(m.install,'verify_inputs',side_effect=lambda data:order.append('inputs')),patch.object(m,'renew_tls',side_effect=lambda data:order.append('renewal')):
            m.main('renew-tls','site.json')
        self.assertEqual(order,['inputs','renewal'])
if __name__=='__main__':unittest.main()
