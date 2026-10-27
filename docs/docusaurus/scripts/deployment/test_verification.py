# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import unittest
from unittest.mock import patch
import verification

class VerificationTests(unittest.TestCase):
    def test_event_policy_requires_every_imported_realm(self):
        row='tenant-a-event-b|EXTERNAL|900|t|true|5|custom-event-listener'
        verification.check_event_rows(row, ['tenant-a-event-b'])
        for broken in ['',row.replace('|900|','|3600|'),row+'\n'+row]:
            with self.assertRaises(ValueError):
                verification.check_event_rows(broken,['tenant-a-event-b'])

    def test_published_key_id_alone_does_not_authorize_different_key(self):
        good={'kid':'k','kty':'RSA','n':'approved','e':'AQAB'}
        verification.check_signing_keys({'keys':[good]},[{'keys':[good]}])
        for bad in [{'keys':[]},{'keys':[{**good,'n':'other'}]},{'keys':[{**good,'kid':'unknown'}]},{'keys':[good,good]}]:
            with self.assertRaises(ValueError):verification.check_signing_keys(bad,[{'keys':[good]}])

    def test_metadata_html_fallback_is_not_a_reachable_api(self):
        verification.check_metadata_response(404,'missing')
        verification.check_metadata_response(405,'<html><head><title>405 Not Allowed</title></head></html>')
        verification.check_metadata_response(200,'<!doctype html><html>portal</html>')
        for status,body in [(0,''),(403,'{"code":"access-denied"}'),(200,'{"code":"access-denied"}'),(200,'{"sources":[]}'),(500,'oops'),(503,'<html>unavailable</html>'),(405,'{"code":"error"}')]:
            with self.assertRaises(ValueError):verification.check_metadata_response(status,body)

    def test_metadata_probe_sends_json_with_its_correct_content_type(self):
        with patch('install.run',return_value=b'<html>405</html>\n405') as run:
            verification.http({'tls':{'ca':'/ca.pem'}},'site.example','/hasura/v1/metadata','metadata',json_body={'type':'export_metadata','args':{}})
        args=run.call_args.args[0]
        self.assertIn('Content-Type: application/json',args)
        self.assertEqual(args[args.index('--data')+1],'{"type": "export_metadata", "args": {}}')

    def test_storage_check_refuses_existing_pod_before_cleanup_is_armed(self):
        with patch('install.command_source',return_value='k3s kubectl apply -f /root/deploy/storage-check.yaml\nk3s kubectl -n step-infra delete pod storage-check\n'):
            script=verification.storage_script({})
        self.assertLess(script.index('test -z "$existing"'),script.index('trap '))
        self.assertIn('kubectl create -f',script)
        self.assertIn('--timeout=30s',script)

    def test_realm_argument_cannot_escape_into_url_or_sql(self):
        for realm in ['tenant-x-event-y/../master',"tenant-x-event-y'",'master','']:
            with self.assertRaises(ValueError):verification.validate_event_name(realm)
        verification.validate_event_name('tenant-123-event-456')

    def test_wrong_restriction_tenant_or_audit_flags_fail(self):
        a='90505c8a-23a9-4cdf-a26b-4e19f6a097d5';b='11111111-1111-4111-8111-111111111111'
        rows='\n'.join(['0','0','0',a,b,'0','0','tenant-'+a+'|t|t','tenant-'+b+'|t|t'])
        verification.check_restrictions(rows,'tenant-'+b)
        with self.assertRaises(ValueError):verification.check_restrictions(rows.replace('|t|t','|t|f'),'tenant-'+b)
        with self.assertRaises(ValueError):verification.check_restrictions(rows+'\nextra','tenant-'+b)

if __name__=='__main__':unittest.main()
