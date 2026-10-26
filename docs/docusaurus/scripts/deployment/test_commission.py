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
            with patch('commission.ask'),patch('install.ssh_args',return_value=['ssh']),patch('install.run'),patch('commission.public_keys',return_value={'signing_public_key':'wrong','share_encryption_public_key':'wrong'}),patch('commission.sql',return_value=json.dumps([{'signing_public_key':'expected','share_encryption_public_key':'expected'}])),patch('install.ssh',return_value=b'/root/deploy/.commission-key.a1b2c3d4\n') as ssh:
                with self.assertRaisesRegex(ValueError,'do not match'):commission.import_trustee({},c,'tenant-test',1)
                self.assertEqual(key.read_text(),'private-key-fixture')
                self.assertEqual(ssh.call_count,2)
                self.assertIn('shred -u',ssh.call_args.args[2])
                self.assertNotIn('create secret',ssh.call_args.args[2])
    def test_failed_transfer_attempts_owned_temporary_cleanup(self):
        with tempfile.TemporaryDirectory() as folder:
            key=Path(folder)/'keys';key.write_text('private-key-fixture');key.chmod(0o600)
            c=self.config();c['trustees'][0]['keys_file']=str(key)
            with patch('commission.ask'),patch('install.ssh_args',return_value=['ssh']),patch('install.run',side_effect=ValueError('transfer failed')),patch('install.ssh',return_value=b'/root/deploy/.commission-key.a1b2c3d4\n') as ssh:
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

if __name__=='__main__':unittest.main()
