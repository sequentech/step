# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
from pathlib import Path
import re
import unittest
from unittest.mock import patch
import install
import verification

class VerificationTests(unittest.TestCase):
    def test_event_policy_requires_every_imported_realm(self):
        row='tenant-a-event-b|EXTERNAL|900|t|true|5|custom-event-listener'
        verification.check_event_rows(row, ['tenant-a-event-b'])
        for broken in ['',row.replace('|900|','|3600|'),row+'\n'+row]:
            with self.assertRaises(ValueError):
                verification.check_event_rows(broken,['tenant-a-event-b'])

    def test_event_realm_keeps_the_template_failure_factor(self):
        jq=(Path(__file__).parent/'commands'/'07-016.jq').read_text()
        self.assertEqual(re.findall(r'\.failureFactor = (\d+)',jq),[verification.EVENT_FAILURE_FACTOR])
        self.assertEqual(re.findall(r'\.failureFactor == (\d+)',install.REALM_CHECK),[verification.EVENT_FAILURE_FACTOR])
        row='tenant-a-event-b|EXTERNAL|900|t|true|{}|custom-event-listener'
        verification.check_event_rows(row.format(verification.EVENT_FAILURE_FACTOR),['tenant-a-event-b'])
        for value in ['1','4','6','30','05','','-5']:
            with self.subTest(failure_factor=value), self.assertRaises(ValueError):
                verification.check_event_rows(row.format(value),['tenant-a-event-b'])

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

    def test_storage_check_pod_runs_on_the_first_server(self):
        # Its image is built on the trustee image; the second server runs none.
        source=(Path(__file__).resolve().parent/'commands'/'10-015.sh').read_text()
        pod=source.split('\nspec:\n',1)[1].split('\n  containers:\n',1)[0]+'\n'
        self.assertIn('  nodeSelector:\n    node-role.kubernetes.io/control-plane: "true"\n',pod)

    def test_storage_inspection_requires_exactly_the_four_policy_lines(self):
        output='pod/storage-check created\nAccessDenied\nNoSuchBucketPolicy\n1\nEnabled\n'
        with patch('verification.storage_script',return_value=''):
            with patch('verification.remote',return_value=output):verification.inspect_storage({})
            for broken in ['0\n'+output,output.replace('\n1\n','\n0\n')]:
                with patch('verification.remote',return_value=broken),self.assertRaises(ValueError):verification.inspect_storage({})

    def test_platform_requires_two_ready_instances_of_harvest_windmill_and_braid(self):
        class Reached(Exception):pass
        def run(counts):
            rows='\n'.join(part+' '+str(counts.get(part,1))+' verified' for part in verification.install.PARTS)
            with patch('install.verify'),patch('commission.validate'),patch('commission.customer'),patch('commission.verify'),patch('install.command_source',return_value=''),patch('verification.remote',side_effect=[rows,Reached()]):
                verification.platform({})
        two={'braid':2,'harvest':2,'windmill':2}
        with self.assertRaises(Reached):run(two)
        for part in two:
            with self.subTest(part=part),self.assertRaises(ValueError):run({**two,part:1})

    def test_platform_refuses_smart_link_and_sign_in_link_providers(self):
        class Reached(Exception):pass
        rows='\n'.join(part+' '+str(2 if part in ('braid','harvest','windmill') else 1)+' verified' for part in verification.install.PARTS)
        def run(files):
            with patch('install.verify'),patch('commission.validate'),patch('commission.customer'),patch('commission.verify'),patch('install.command_source',return_value=''),patch('verification.remote',side_effect=[rows,files,Reached()]):
                verification.platform({})
        clean='sequent.conditional-authenticators.jar\nsequent.custom-event-listener.jar\n0\n0\n'
        with self.assertRaises(Reached):run(clean)
        # A named provider jar, handler classes in another jar, a listed name, no listing, the earlier output.
        for files in (clean.replace('0\n0\n','1\n0\n'),clean.replace('0\n0\n','0\n2\n'),'sequent.sign-in-link-handlers.jar\n'+clean,'0\n0\n','sequent.conditional-authenticators.jar\n0\n'):
            with self.subTest(files=files),self.assertRaises(ValueError):run(files)

    def test_realm_argument_cannot_escape_into_url_or_sql(self):
        for realm in ['tenant-x-event-y/../master',"tenant-x-event-y'",'master','']:
            with self.assertRaises(ValueError):verification.validate_event_name(realm)
        verification.validate_event_name('tenant-123-event-456')

    def test_wrong_restriction_tenant_or_audit_flags_fail(self):
        a='90505c8a-23a9-4cdf-a26b-4e19f6a097d5';b='11111111-1111-4111-8111-111111111111'
        rows='\n'.join(['0','0','0',a,b,'0','0','tenant-'+a+'|t|t','tenant-'+b+'|t|t','0','0|2','0|0|0'])
        verification.check_restrictions(rows,'tenant-'+b)
        with self.assertRaises(ValueError):verification.check_restrictions(rows.replace('|t|t','|t|f'),'tenant-'+b)
        with self.assertRaises(ValueError):verification.check_restrictions(rows+'\nextra','tenant-'+b)
        # Account self-service, required actions, profile or client findings; old or missing credentials;
        # a client without a required token mapper, offered offline access, or a missing row.
        for tail in (['1','0|2','0|0|0'],['0','2|2','0|0|0'],['0','0|1','0|0|0'],['0','0|2','1|0|0'],['0','0|2','0|1|0'],['0','0|2','0|0|1'],['0','0|2'],['0']):
            with self.subTest(tail=tail), self.assertRaises(ValueError):
                verification.check_restrictions('\n'.join(rows.split('\n')[:9]+tail),'tenant-'+b)

if __name__=='__main__':unittest.main()
