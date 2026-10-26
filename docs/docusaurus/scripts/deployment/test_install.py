# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Installer boundaries: no network or platform mutation in these tests."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import install

class InstallerTests(unittest.TestCase):
    def config(self):
        d=json.loads(Path(__file__).with_name('site.example.json').read_text())
        d['kit']['signing_fingerprint']='SHA256:'+'a'*43
        d['toe']['format']='tar'
        d['toe']['archive_sha256']={p:'a'*64 for p in install.PARTS}
        d['toe']['manifest_sha256']='a'*64
        d['tls']['ca_sha256']=d['identity_provider']['ca_sha256']='a'*64
        d['release']['step_commit']='a'*40
        return d
    def test_metadata_consistency_rejects_unavailable_actions_before_handover(self):
        with patch('install.ssh',return_value=b'{"is_consistent":true,"inconsistent_count":0}'):
            install.verify_metadata(self.config())
        for result in ({'is_consistent':False,'inconsistent_count':196},{'is_consistent':True,'inconsistent_count':1},{'inconsistent_count':0}):
            with patch('install.ssh',return_value=json.dumps(result).encode()):
                with self.assertRaisesRegex(ValueError,'metadata is inconsistent'):install.verify_metadata(self.config())

    def test_runtime_identity_uses_spec_container_name_not_normalized_status_tag(self):
        import subprocess
        expected='sequentech.local/harvest@sha256:'+'a'*64
        fixture={'items':[{'spec':{'containers':[{'name':'harvest','image':expected},{'name':'other','image':'other:latest'}],'initContainers':[{'name':'setup','image':expected}]},'status':{'containerStatuses':[{'name':'harvest','image':'sequentech.local/harvest:v10.0.0','imageID':expected},{'name':'other','image':expected,'imageID':'unrelated'}],'initContainerStatuses':[{'name':'setup','image':'sha256:normalized','imageID':expected}]}}]}
        args=['jq','-r','--arg','image',expected,install.RUNTIME_IDS]
        result=subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(result.stdout.splitlines(),[expected,expected])
        fixture['items'][0]['spec']['containers'][0]['image']='unapproved:latest'
        result=subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True)
        self.assertEqual(result.stdout.splitlines(),[expected])

    def test_supporting_payload_requires_signed_complete_and_bound_metadata(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'release').mkdir()
            revision='a'*40
            assets={'schema_version':1,'step_revision':revision,'bucket':'public','prefix':'public-assets','file_count':1,'files':[{'path':'report.hbs'}]}
            asset_path=root/'release/public-assets.json';asset_path.write_text(json.dumps(assets))
            inputs={'schemaVersion':1,'stepRevision':revision,'publicAssetsInventorySha256':install.digest(asset_path)}
            input_path=root/'release/setup-image-inputs.json';input_path.write_text(json.dumps(inputs))
            entry={'ref':'sequentech.local/trustee:latest','kind':'sequent','dockerfile':'beyond:airgap/docker/trustee-setup.Dockerfile','imageId':'sha256:'+'b'*64}
            images={'schemaVersion':1,'stepRevision':revision,'images':[entry]}
            image_path=root/'release/images.json';image_path.write_text(json.dumps(images))
            signed={'release/provenance.json','release/images.json','release/public-assets.json','release/setup-image-inputs.json'}
            self.assertEqual(install.verify_setup_payload(root,signed),entry['imageId'])
            for missing in signed:
                with self.assertRaisesRegex(ValueError,'missing signed'):install.verify_setup_payload(root,signed-{missing})
            assets['step_revision']='c'*40;asset_path.write_text(json.dumps(assets))
            with self.assertRaisesRegex(ValueError,'revisions differ'):install.verify_setup_payload(root,signed)
            assets['step_revision']=revision;assets['files'][0]['path']='altered.hbs';asset_path.write_text(json.dumps(assets))
            with self.assertRaisesRegex(ValueError,'different public-asset'):install.verify_setup_payload(root,signed)
            inputs['publicAssetsInventorySha256']=install.digest(asset_path);input_path.write_text(json.dumps(inputs))
            images['images'].append(entry);image_path.write_text(json.dumps(images))
            with self.assertRaisesRegex(ValueError,'exactly one'):install.verify_setup_payload(root,signed)

    def test_supporting_image_identity_is_checked_on_each_node_without_writes(self):
        d=self.config();d['_setup_image_id']='sha256:'+'b'*64
        for node in ('c1','c2'):
            with patch('install.ssh',return_value=json.dumps({'status':{'id':d['_setup_image_id']}}).encode()) as ssh:
                install.verify_setup_image(d,node)
                self.assertEqual(ssh.call_args.args[1],node)
                self.assertEqual(ssh.call_args.args[2],'k3s crictl inspecti sequentech.local/trustee:latest\n')
            with patch('install.ssh',return_value=json.dumps({'status':{'id':'sha256:'+'c'*64,'repoDigests':['sequentech.local/trustee@'+d['_setup_image_id']]}}).encode()):
                install.verify_setup_image(d,node)
            with patch('install.ssh',return_value=json.dumps({'status':{'id':'sha256:'+'c'*64,'repoDigests':['unrelated.invalid/image@'+d['_setup_image_id']]}}).encode()):
                with self.assertRaisesRegex(ValueError,'differs from the accepted kit'):install.verify_setup_image(d,node)
            with patch('install.ssh',return_value=json.dumps({'status':{'id':'sha256:'+'c'*64}}).encode()):
                with self.assertRaisesRegex(ValueError,'differs from the accepted kit'):install.verify_setup_image(d,node)

    def test_oci_profile_needs_no_user_created_archive_hashes(self):
        d=self.config();d['toe'].pop('archive_sha256');d['toe']['format']='oci'
        install.validate(d)
        d['toe']['archive_sha256']={part:'a'*64 for part in install.PARTS}
        with self.assertRaisesRegex(ValueError,'unknown field'):install.validate(d)

    def test_realm_assertions_respect_voter_and_administrator_scope(self):
        import subprocess
        base={'sslRequired':'external','ssoSessionIdleTimeout':900,'bruteForceProtected':True,'failureFactor':5,'eventsEnabled':True,'registrationAllowed':False,'resetPasswordAllowed':False,'rememberMe':False,'eventsListeners':['custom-event-listener'],'identityProviders':[{'alias':'external-login'}],'groups':[]}
        for kind in ('tenant','tenant-template','event-template'):
            fixture=copy.deepcopy(base);fixture['adminEventsEnabled']=fixture['adminEventsDetailsEnabled']=kind!='event-template'
            fixture['clients']=[{'clientId':name,'directAccessGrantsEnabled':False,'attributes':{'pkce.code.challenge.method':'S256'}} for name in (['voting-portal'] if kind=='event-template' else ['admin-portal','voting-portal'])]
            args=['jq','-e','--arg','kind',kind,'--arg','alias','external-login',install.REALM_CHECK]
            result=subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True)
            self.assertEqual(result.returncode,0,(kind,result.stderr))
            fixture['clients'][0]['directAccessGrantsEnabled']=True
            self.assertNotEqual(subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True).returncode,0)
            fixture['clients']=[]
            self.assertNotEqual(subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True).returncode,0)

    def test_config_rejects_unknown_keys_and_shell_host(self):
        d=self.config();install.validate(d)
        for mutation in [lambda x:x.update(secret='wrong'),lambda x:x['nodes'].update(c1='host;id'),lambda x:x['hosts'].update(voting='x$(id).test')]:
            candidate=copy.deepcopy(d);mutation(candidate)
            with self.assertRaises(ValueError):install.validate(candidate)
    def test_config_requires_all_six_digests_distinct_nodes_and_public_inputs(self):
        for mutation in [lambda x:x['toe']['archive_sha256'].pop('braid'),lambda x:x['nodes'].update(c2=x['nodes']['c1']),lambda x:x['operator'].update(username='$(cat /root/secret)')]:
            d=self.config();mutation(d)
            with self.assertRaises(ValueError):install.validate(d)
    def test_private_input_permissions_and_symlinks(self):
        with tempfile.TemporaryDirectory() as folder:
            p=Path(folder)/'secret';p.write_text('secret');p.chmod(0o600)
            install.checked_path(str(p),private=True)
            p.chmod(0o644)
            with self.assertRaises(ValueError):install.checked_path(str(p),private=True)
            link=Path(folder)/'link';link.symlink_to(p)
            with self.assertRaises(ValueError):install.checked_path(str(link))
    def test_stages_exclude_custody_backup_and_unchecked_interactive_commands(self):
        d=self.config();s=install.install_script(d)
        self.assertNotIn('11-003',s);self.assertNotIn('08-014',s)
        self.assertNotIn('<', '\n'.join(line for line in s.splitlines() if line.startswith('source ')))
        self.assertIn('set -Eeuo pipefail',s)
        self.assertIn('verify_realms',s)
        self.assertIn('test "$POSTGRES_PASSWORD"',s)
    def test_failures_do_not_expose_subprocess_output(self):
        import subprocess
        with patch('install.subprocess.run',return_value=subprocess.CompletedProcess([],1,b'PASSWORD',b'SECRET')):
            with self.assertRaisesRegex(ValueError,'stage failed') as e:install.run(['false'],'stage')
            self.assertNotIn('PASSWORD',str(e.exception));self.assertNotIn('SECRET',str(e.exception))
    def test_preflight_refuses_existing_platform_before_upload(self):
        s=install.preflight_script(self.config(),fresh=True)
        self.assertIn('step-apps',s);self.assertIn('test ! -e /root/deploy',s)
        self.assertNotIn('mkdir',s);self.assertNotIn('kubectl apply',s)
    def test_shell_public_values_are_quoted(self):
        d=self.config();s=install.environment(d)
        self.assertIn('export PORTAL_HOST=vote.example.org',s)
        self.assertNotIn('secret',s.lower())
    def test_archive_member_paths_cannot_escape_kit(self):
        with tempfile.TemporaryDirectory() as folder:
            p=Path(folder);(p/'ok').write_bytes(b'ok')
            with self.assertRaises(ValueError):install.verify_checksums(p,'a'*64+'  ../escape\n')
    def test_agent_preflight_does_not_run_control_plane_commands(self):
        s=install.preflight_script(self.config(),True,node='c2')
        self.assertNotIn('secrets-encrypt',s);self.assertNotIn('kubectl get namespace',s)
        self.assertIn('chronyc',s);self.assertIn('test ! -e /root/deploy',s)
    def test_modified_manifest_is_rejected_before_any_certificate_or_network_check(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'images').mkdir();key=root/'key';key.write_text('private');key.chmod(0o600)
            d=self.config();d['ssh_identity']=str(key);d['toe']['directory']=str(root)
            for part in install.PARTS:
                p=root/'images'/(part+'.tar');p.write_bytes(part.encode());d['toe']['archive_sha256'][part]=install.digest(p)
            (root/'image-digests.txt').write_text('tampered')
            with patch('install.verify_kit'),patch('install.run') as run:
                with self.assertRaisesRegex(ValueError,'manifest differs'):install.verify_inputs(d)
                run.assert_not_called()
    def test_source_identity_comes_from_toe_release_not_os_kit(self):
        d=self.config();d['_kit_identity']={'version':'1.2.3','step_commit':'b'*40}
        script=install.command_source('07-004',d)
        self.assertIn('a'*40,script);self.assertNotIn('b'*40,script)
        import subprocess
        self.assertEqual(subprocess.run(['bash','-n'],input=install.install_script(d),text=True,capture_output=True).returncode,0)
    def test_only_allowlisted_failure_reason_is_shown(self):
        import subprocess
        with patch('install.subprocess.run',return_value=subprocess.CompletedProcess([],1,b'SECRET',b'SECRET\nAGD_CHECK_FAILED TIME_AUTHENTICATION\n')):
            with self.assertRaisesRegex(ValueError,'NTS-authenticated time') as error:install.run(['false'],'Node c1')
            self.assertNotIn('SECRET',str(error.exception))

    def test_idp_policy_cannot_disable_signature_or_pkce(self):
        d={'alias':'external-login','providerId':'oidc','enabled':True,'trustEmail':False,'storeToken':False,'linkOnly':False,'firstBrokerLoginFlowAlias':'certificate-first-login-flow','config':{'issuer':'https://login.example.org','authorizationUrl':'https://login.example.org/auth','tokenUrl':'https://login.example.org/token','jwksUrl':'https://login.example.org/jwks','logoutUrl':'https://login.example.org/logout','clientId':'platform','clientSecret':'not-printed','clientAuthMethod':'client_secret_post','validateSignature':'true','useJwksUrl':'true','pkceEnabled':'true','pkceMethod':'S256'}}
        install.validate_idp(d,'external-login')
        d['config']['validateSignature']='false'
        with self.assertRaises(ValueError):install.validate_idp(d,'external-login')

if __name__=='__main__':unittest.main()
