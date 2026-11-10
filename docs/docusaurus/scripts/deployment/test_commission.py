# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import commission
import install

class CommissionTests(unittest.TestCase):
    def test_accounts_require_separate_completion_readback(self):
        c=self.config();d={'operator':{'username':'bootstrap','email':'b@example.org'}}
        with patch('install.command_source',return_value='true'),patch('install.ssh',side_effect=[b'',b'wrong-identity']) as ssh:
            with self.assertRaisesRegex(ValueError,'did not complete'):commission.accounts(d,c,'tenant-test')
            self.assertEqual(ssh.call_count,2)
            self.assertIn('test -f /root/deploy/.commission-accounts-ready',ssh.call_args.args[2])

    def test_confirmation_uses_a_real_controlling_terminal(self):
        import os
        import pty
        import select
        import signal
        import sys
        import time
        for answer, expected in [('CONTINUE',0),('PAUSE',1)]:
            pid,fd=pty.fork()
            if pid==0:
                code='import sys;sys.path.insert(0,'+repr(str(Path(__file__).parent))+');import commission;commission.ask("Actual terminal confirmation")'
                os.execl(sys.executable,sys.executable,'-c',code)
            output=b'';status=None;deadline=time.monotonic()+10;answered=False
            try:
                while time.monotonic()<deadline:
                    ready,_,_=select.select([fd],[],[],0.1)
                    if ready:
                        try:chunk=os.read(fd,4096)
                        except OSError:chunk=b''
                        output+=chunk
                        if b'Type CONTINUE' in output and not answered:
                            os.write(fd,(answer+'\n').encode());answered=True
                    done,state=os.waitpid(pid,os.WNOHANG)
                    if done:status=state;break
                self.assertIsNotNone(status,'Terminal confirmation timed out')
                self.assertTrue(answered,output.decode(errors='replace'))
                self.assertEqual(os.waitstatus_to_exitcode(status),expected,output.decode(errors='replace'))
            finally:
                os.close(fd)
                if status is None:
                    os.kill(pid,signal.SIGKILL);os.waitpid(pid,0)

    def config(self):
        return {'customer_slug':'customer','administrator':{'username':'customer-admin','email':'a@example.org'},'trustees':[{'name':'one','keys_file':'/media/one.toml'},{'name':'two','keys_file':'/media/two.toml'}]}
    def test_exact_two_distinct_trustees_and_safe_identifiers(self):
        c=self.config();commission.validate(c)
        for mutate in [lambda x:x['trustees'].pop(),lambda x:x['trustees'][1].update(name='one'),lambda x:x.update(customer_slug="x';id"),lambda x:x['administrator'].update(password='not allowed')]:
            bad=copy.deepcopy(c);mutate(bad)
            with self.assertRaises(ValueError):commission.validate(bad)
    def test_partial_state_prevents_any_write(self):
        with patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='partial'),patch('commission.accounts') as accounts,patch('commission.import_trustee') as imported:
            with self.assertRaisesRegex(ValueError,'automatic replay'):commission.main({'commissioning':self.config()})
            accounts.assert_not_called();imported.assert_not_called()
    def test_changed_config_after_accounts_refuses_continuation(self):
        with patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='accounts'),patch('install.ssh',return_value=b'wrong-digest'),patch('commission.ask') as ask,patch('commission.import_trustee') as imported:
            with self.assertRaisesRegex(ValueError,'configuration changed'):commission.main({'commissioning':self.config()})
            ask.assert_not_called();imported.assert_not_called()
    def test_wrong_registered_key_pair_cleans_transfer_without_import(self):
        with tempfile.TemporaryDirectory() as folder:
            key=Path(folder)/'keys';key.write_text('private-key-fixture');key.chmod(0o600)
            c=self.config();c['trustees'][0]['keys_file']=str(key)
            with patch('commission.ask'),patch('install.ssh_args',return_value=['ssh']),patch('install.run'),patch('commission.public_keys',return_value={'signing_public_key':'wrong','share_encryption_public_key':'wrong'}),patch('commission.sql',return_value=json.dumps([{'signing_public_key':'expected','share_encryption_public_key':'expected'}])),patch('install.ssh',return_value=b'/run/sequent-commission-key.a1b2c3d4\n') as ssh:
                with self.assertRaisesRegex(ValueError,'do not match'):commission.import_trustee({},c,'tenant-test',1)
                self.assertEqual(key.read_text(),'private-key-fixture')
                self.assertEqual(ssh.call_count,2)
                self.assertIn('shred -u',ssh.call_args.args[2])
                self.assertNotIn('create secret',ssh.call_args.args[2])
    def test_the_transfer_copy_is_kept_in_memory(self):
        with tempfile.TemporaryDirectory() as folder:
            key=Path(folder)/'keys';key.write_text('private-key-fixture');key.chmod(0o600)
            c=self.config();c['trustees'][0]['keys_file']=str(key)
            for returned in (b'/root/deploy/.commission-key.a1b2c3d4\n',b'/run/sequent-commission-key.a1b2c3d4/../x\n'):
                with patch('commission.ask'),patch('install.run') as run,patch('install.ssh',return_value=returned) as ssh:
                    with self.assertRaisesRegex(ValueError,'Unexpected private transfer directory'):commission.import_trustee({},c,'tenant-test',1)
                    run.assert_not_called();self.assertEqual(ssh.call_count,1)
            with patch('commission.ask'),patch('install.ssh_args',return_value=['ssh']),patch('install.run') as run,patch('commission.public_keys',return_value={'signing_public_key':'wrong','share_encryption_public_key':'wrong'}),patch('commission.sql',return_value=json.dumps([{'signing_public_key':'expected','share_encryption_public_key':'expected'}])),patch('install.ssh',return_value=b'/run/sequent-commission-key.a1b2c3d4\n') as ssh:
                with self.assertRaises(ValueError):commission.import_trustee({},c,'tenant-test',1)
            self.assertEqual(ssh.call_args_list[0].args[2],commission.TRANSFER_DIRECTORY)
            self.assertIn('cat > /run/sequent-commission-key.a1b2c3d4/keys.toml',run.call_args.args[0][-1])
            self.assertIn('shred -u /run/sequent-commission-key.a1b2c3d4/keys.toml',ssh.call_args.args[2])
    def test_the_transfer_directory_is_made_only_on_a_memory_file_system(self):
        import os
        import subprocess
        for kind,made in (('tmpfs',True),('ext2/ext3',False)):
            with tempfile.TemporaryDirectory() as root:
                script=commission.TRANSFER_DIRECTORY.replace('/run/',root+'/')
                stat='stat() { echo '+kind+'; }\n'
                result=subprocess.run(['bash','-c',stat+script],capture_output=True,text=True,timeout=10,env={'PATH':os.environ['PATH']})
                left=list(Path(root).iterdir())
                self.assertEqual(result.returncode==0,made,result.stderr)
                self.assertEqual(len(left),1 if made else 0)
                if made:
                    self.assertEqual(result.stdout,str(left[0])+'\n')
                    self.assertRegex(left[0].name,r'^sequent-commission-key\.[A-Za-z0-9]{8}$')
                    self.assertEqual(left[0].stat().st_mode&0o777,0o700)
                else:
                    self.assertEqual(result.stdout,'')
                    self.assertIn('is not in memory',result.stderr)
        self.assertEqual(subprocess.run(['stat','-f','-c','%T','/dev/shm'],capture_output=True,text=True).stdout.strip(),'tmpfs')
    def test_failed_transfer_attempts_owned_temporary_cleanup(self):
        with tempfile.TemporaryDirectory() as folder:
            key=Path(folder)/'keys';key.write_text('private-key-fixture');key.chmod(0o600)
            c=self.config();c['trustees'][0]['keys_file']=str(key)
            with patch('commission.ask'),patch('install.ssh_args',return_value=['ssh']),patch('install.run',side_effect=ValueError('transfer failed')),patch('install.ssh',return_value=b'/run/sequent-commission-key.a1b2c3d4\n') as ssh:
                with self.assertRaisesRegex(ValueError,'transfer failed'):commission.import_trustee({},c,'tenant-test',1)
                self.assertTrue(any('shred -u' in call.args[2] for call in ssh.call_args_list))
                self.assertEqual(key.read_text(),'private-key-fixture')
    def test_precheck_failure_does_not_claim_imports_started(self):
        c=self.config()
        import hashlib
        identity=hashlib.sha256(json.dumps(c,sort_keys=True).encode()).hexdigest().encode()
        with patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='accounts'),patch('install.ssh',return_value=identity) as ssh,patch('commission.ask'),patch('commission.import_trustee',side_effect=ValueError('wrong media')):
            with self.assertRaisesRegex(ValueError,'wrong media'):commission.main({'commissioning':c})
            self.assertFalse(any('printf started > /root/deploy/.commission-imports-started' in call.args[2] for call in ssh.call_args_list))

    def test_media_change_after_precheck_refuses_transfer(self):
        with tempfile.TemporaryDirectory() as folder:
            key=Path(folder)/'keys';key.write_text('changed');key.chmod(0o600)
            c=self.config();c['trustees'][0]['keys_file']=str(key)
            with patch('install.run') as run:
                with self.assertRaisesRegex(ValueError,'medium changed'):commission.import_trustee({},c,'tenant-test',1,apply=True,expected_hash='a'*64)
                run.assert_not_called()
    def test_both_pairs_checked_before_first_secret_import(self):
        c=self.config();calls=[]
        def imported(*args,**kwargs):calls.append((args[3],kwargs.get('apply',False)));return 'a'*64
        import hashlib
        identity=hashlib.sha256(json.dumps(c,sort_keys=True).encode()).hexdigest().encode()
        with patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='accounts'),patch('install.ssh',return_value=identity),patch('commission.ask'),patch('commission.import_trustee',side_effect=imported),patch('commission.verify'):
            commission.main({'commissioning':c})
        self.assertEqual(calls,[(1,False),(2,False),(1,True),(2,True)])

    def identity(self,c):
        import hashlib
        return hashlib.sha256(json.dumps(c,sort_keys=True).encode()).hexdigest()

    def test_new_keys_after_a_removal_reuse_the_trustee_import(self):
        c=self.config();calls=[]
        def remote(d,node,script,label,timeout=120):
            calls.append(label)
            return {'Commissioned customer':b'tenant-test'}.get(label,b'')
        def imported(*args,**kwargs):calls.append(('import',args[3],kwargs.get('apply',False),kwargs.get('expected_hash')));return 'h'+str(args[3])
        with patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='complete'),patch('commission.trustee_keys',return_value='removed'),patch('install.ssh',side_effect=remote) as ssh,patch('commission.ask',side_effect=lambda message:calls.append('ask')),patch('commission.import_trustee',side_effect=imported),patch('commission.verify',side_effect=lambda d,realm:calls.append('verify')),patch('builtins.print'):
            commission.main({'commissioning':c})
        self.assertEqual(calls,['Commissioned customer','ask',('import',1,False,None),('import',2,False,None),'New trustee keys import boundary',('import',1,True,'h1'),('import',2,True,'h2'),'verify','Commissioning record'])
        self.assertEqual(ssh.call_args_list[1].args[2],commission.NEW_KEYS_STARTED)
        record=ssh.call_args.args[2]
        self.assertIn('printf %s '+self.identity(c)+' > /root/deploy/.commissioned',record)
        self.assertIn('rm /root/deploy/.trustee-keys-import-started /root/deploy/.trustee-keys-destroyed',record)

    def test_new_keys_need_the_commissioned_customer(self):
        with patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='complete'),patch('commission.trustee_keys',return_value='removed'),patch('install.ssh',return_value=b'tenant-other') as ssh,patch('commission.ask') as ask,patch('commission.import_trustee') as imported:
            with self.assertRaisesRegex(ValueError,'differs from the commissioned one'):commission.main({'commissioning':self.config()})
        ask.assert_not_called();imported.assert_not_called();self.assertEqual(ssh.call_count,1)

    def test_an_unfinished_removal_or_import_refuses_new_keys(self):
        for keys,message in [('removing','removal stopped part way; run ./deploy remove-trustees site.json to complete it'),('compacting','run ./deploy remove-trustees site.json again until it reports DATASTORE COMPACTED'),('importing','run ./deploy remove-trustees site.json to remove what it installed, then ./deploy commission site.json again')]:
            with self.subTest(keys=keys),patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='complete'),patch('commission.trustee_keys',return_value=keys),patch('install.ssh') as ssh,patch('commission.ask') as ask,patch('commission.import_trustee') as imported,patch('commission.verify') as verified:
                with self.assertRaises(ValueError) as caught:commission.main({'commissioning':self.config()})
                self.assertIn(message,str(caught.exception))
                ssh.assert_not_called();ask.assert_not_called();imported.assert_not_called();verified.assert_not_called()

    def test_kept_keys_keep_the_completed_record_check(self):
        c=self.config()
        with patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='complete'),patch('commission.trustee_keys',return_value='kept'),patch('install.ssh',return_value=self.identity(c).encode()),patch('commission.import_trustee') as imported,patch('commission.verify') as verified:
            commission.main({'commissioning':c})
        imported.assert_not_called();verified.assert_called_once()
        with patch('install.verify'),patch('commission.customer',return_value='tenant-test'),patch('commission.state',return_value='complete'),patch('commission.trustee_keys',return_value='kept'),patch('install.ssh',return_value=b'other'),patch('commission.import_trustee') as imported:
            with self.assertRaisesRegex(ValueError,'differs from the completed record'):commission.main({'commissioning':c})

    def test_an_imported_trustee_is_started_once_both_its_secrets_exist(self):
        with tempfile.TemporaryDirectory() as folder:
            key=Path(folder)/'keys';key.write_text('private-key-fixture');key.chmod(0o600)
            c=self.config();c['trustees'][1]['keys_file']=str(key)
            import hashlib
            expected=hashlib.sha256(b'private-key-fixture').hexdigest()
            pair={'signing_public_key':'s','share_encryption_public_key':'e'}
            with patch('install.ssh_args',return_value=['ssh']),patch('install.run'),patch('commission.public_keys',return_value=pair),patch('commission.sql',return_value=json.dumps([pair])),patch('install.command_source',side_effect=lambda ident,values:'# step '+ident),patch('install.ssh',return_value=b'/run/sequent-commission-key.a1b2c3d4\n') as ssh,patch('builtins.print'):
                commission.import_trustee({},c,'tenant-test',2,apply=True,expected_hash=expected)
            script=next(call.args[2] for call in ssh.call_args_list if call.args[3]=='Native trustee 2 binding and startup')
            lines=script.splitlines()
            order=[lines.index(line) for line in ('# step 08-009','k3s kubectl -n step-infra create secret generic trustee2-keys --from-file=trustee.toml=/run/sequent-commission-key.a1b2c3d4/keys.toml','# step 08-011','k3s kubectl -n step-infra scale deployment/trustee2 --replicas=1','k3s kubectl -n step-infra rollout status deployment/trustee2 --timeout=5m')]
            self.assertEqual(order,sorted(order))

    def remote(self,root,script,fail=False):
        import os
        import subprocess
        script=script.replace('/root/deploy',str(root/'deploy')).replace('/var/lib/sequent-data',str(root/'data')).replace('/var/lib/sequent-maintenance',str(root/'maintenance'))
        mock='k3s() { if test -e '+str(root/'unreachable')+'; then echo "error: the server is unreachable" >&2; return 1; fi; for n in trustee1-keys trustee2-keys trustee1-auth trustee2-auth; do if test -e '+str(root/'secrets')+'/$n; then echo secret/$n; fi; done; }\n'
        return subprocess.run(['bash','-c','set -Eeuo pipefail\nset +x\numask 077\n'+mock+script],capture_output=True,text=True,timeout=30,env={'PATH':os.environ['PATH']})

    def layout(self,folder):
        root=Path(folder)
        for path in ('deploy','secrets','data/trustee1','data/trustee2'):(root/path).mkdir(parents=True)
        return root

    def test_the_trustee_key_state_is_read_from_the_servers(self):
        cases=[('removed',lambda root:(root/'deploy'/'.trustee-keys-destroyed').touch()),
               ('compacting',lambda root:None),
               ('kept',lambda root:((root/'deploy'/'.trustee-keys-destroyed').touch(),(root/'secrets'/'trustee2-auth').touch())),
               ('kept',lambda root:((root/'deploy'/'.trustee-keys-destroyed').touch(),(root/'data'/'trustee1'/'message_store').mkdir())),
               ('removing',lambda root:((root/'deploy'/'.trustee-keys-destroyed').touch(),(root/'deploy'/'.trustee-removal-in-progress').touch())),
               ('importing',lambda root:((root/'deploy'/'.trustee-keys-destroyed').touch(),(root/'deploy'/'.trustee-keys-import-started').touch(),(root/'secrets'/'trustee1-keys').touch()))]
        with patch('install.ssh',return_value=b'kept\n') as ssh:
            self.assertEqual(commission.trustee_keys({}),'kept')
        script=ssh.call_args.args[2]
        for expected,arrange in cases:
            with self.subTest(expected=expected),tempfile.TemporaryDirectory() as folder:
                root=self.layout(folder);arrange(root)
                result=self.remote(root,script)
                self.assertEqual((result.returncode,result.stdout),(0,expected),result.stderr)
        # An unreachable cluster is not an absent Secret.
        with tempfile.TemporaryDirectory() as folder:
            root=self.layout(folder);(root/'deploy'/'.trustee-keys-destroyed').touch();(root/'unreachable').touch()
            result=self.remote(root,script)
            self.assertNotEqual(result.returncode,0);self.assertNotIn('removed',result.stdout)

    def test_the_new_keys_boundary_needs_a_finished_removal_and_no_other_operation(self):
        def arranged(*paths):
            folder=tempfile.TemporaryDirectory();root=self.layout(folder.name)
            (root/'deploy'/'.trustee-keys-destroyed').touch()
            for path in paths:
                target=root/path;target.parent.mkdir(parents=True,exist_ok=True)
                if path.endswith('claim'):target.mkdir()
                else:target.touch()
            return folder,root
        folder,root=arranged()
        with folder:
            result=self.remote(root,commission.NEW_KEYS_STARTED)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertTrue((root/'deploy'/'.trustee-keys-import-started').exists())
            self.assertNotEqual(self.remote(root,commission.NEW_KEYS_STARTED).returncode,0)
        for paths in (['maintenance/claim'],['deploy/.backup-in-progress'],['maintenance/state.json']):
            folder,root=arranged(*paths)
            with self.subTest(paths=paths),folder:
                self.assertNotEqual(self.remote(root,commission.NEW_KEYS_STARTED).returncode,0)
                self.assertFalse((root/'deploy'/'.trustee-keys-import-started').exists())
        with tempfile.TemporaryDirectory() as name:
            root=self.layout(name)
            self.assertNotEqual(self.remote(root,commission.NEW_KEYS_STARTED).returncode,0)
            self.assertFalse((root/'deploy'/'.trustee-keys-import-started').exists())

if __name__=='__main__':unittest.main()
