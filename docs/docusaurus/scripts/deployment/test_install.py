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
    def test_security_mail_requires_private_tls_configuration_and_bounded_timeout(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'smtp.json'
            config=self.config();config['notifications']['smtp_config_file']=str(path)
            path.write_text(json.dumps({'server_url':'smtps://mailer:private@smtp.example.invalid:465'}));path.chmod(0o600)
            valid=json.loads(install.notification_config(config))
            self.assertEqual(valid['timeout'],5)
            for invalid in ({'server_url':'smtp://smtp.example.invalid'}, {'server_url':'smtps://smtp.example.invalid','timeout':0}, {'server_url':'smtps://smtp.example.invalid','timeout':120}, {'server_url':'smtps://smtp.example.invalid','accept_invalid_certs':True}):
                path.write_text(json.dumps(invalid))
                with self.subTest(invalid=invalid), self.assertRaises(ValueError):install.notification_config(config)
            path.write_text(json.dumps({'server_url':'smtps://smtp.example.invalid'}));path.chmod(0o644)
            with self.assertRaises(ValueError):install.notification_config(config)

    def test_security_mail_is_configured_before_application_workers_start(self):
        script=install.install_script(self.config())
        self.assertLess(script.index('create secret generic security-notifications'),script.index('source /root/deploy/installation-commands/08-005.sh'))
        self.assertNotIn('mailer:private',script)

    def config(self):
        d=json.loads(Path(__file__).with_name('site.example.json').read_text())
        d['kit']['signing_fingerprint']='SHA256:'+'a'*43
        d['toe']['format']='tar'
        d['toe']['archive_sha256']={p:'a'*64 for p in install.PARTS}
        d['toe']['manifest_sha256']='a'*64
        d['tls']['ca_sha256']=d['identity_provider']['ca_sha256']='a'*64
        d['release']['step_commit']='a'*40
        return d
    def test_installation_supplies_authenticated_ca_before_starting_workers(self):
        script=install.install_script(self.config())
        supplied=script.index('source /root/deploy/installation-commands/07-006.sh')
        ca=install.command_source('07-006',self.config())
        self.assertIn('create configmap self-test-tls-ca',ca)
        self.assertIn('--from-file=ca.pem=/root/deploy/tls/ca.pem',ca)
        started=script.index('source /root/deploy/installation-commands/08-005.sh')
        self.assertLess(supplied,started)
        self.assertNotIn('--insecure',script)

    def test_ssh_script_input_cannot_be_consumed_by_child_and_is_removed(self):
        import subprocess
        script='set -e\nprintf "%s\n" "$0"\ntest "$(stat -c %a "$0")" = 600\ncat >/dev/null\nprintf "explicit-input" | cat\nprintf "\nafter-child\n"\n'
        result=subprocess.run(['bash','-c',install.SSH_SCRIPT_WRAPPER],input=script,text=True,capture_output=True)
        self.assertEqual(result.returncode,0,result.stderr)
        lines=result.stdout.splitlines()
        self.assertEqual(lines[1:],['explicit-input','after-child'])
        self.assertFalse(Path(lines[0]).exists())
        result=subprocess.run(['bash','-c',install.SSH_SCRIPT_WRAPPER],input='printf "%s\n" "$0"; exit 23\n',text=True,capture_output=True)
        self.assertEqual(result.returncode,23)
        self.assertFalse(Path(result.stdout.strip()).exists())

    def test_discovery_retries_only_startup_failures_with_verified_https(self):
        import subprocess
        d=self.config();host=d['hosts']['administration']
        expected='https://'+d['hosts']['voting']+'/realms/'+install.R
        ok=subprocess.CompletedProcess([],0,json.dumps({'issuer':expected}).encode()+b'\n200',b'')
        for initial in [subprocess.CompletedProcess([],code,b'\n000',b'private error') for code in (7,28,52,56)]+[subprocess.CompletedProcess([],0,b'upstream unavailable\n'+status,b'') for status in (b'502',b'503',b'504')]+[subprocess.TimeoutExpired('curl',4)]:
            with self.subTest(initial=initial),patch('install.subprocess.run',side_effect=[initial,ok]) as call,patch('install.time.sleep') as sleep:
                self.assertEqual(install.verify_discovery(d,host,install.R),{'issuer':expected})
                self.assertEqual(call.call_count,2);sleep.assert_called_once_with(1)
                args=call.call_args.args[0]
                self.assertEqual(args[args.index('--cacert')+1],d['tls']['ca'])
                self.assertEqual(args[args.index('--connect-timeout')+1],'2')
                self.assertEqual(args[args.index('--max-time')+1],'3')
                self.assertEqual(call.call_args.kwargs['timeout'],4)
                self.assertNotIn('--insecure',args);self.assertNotIn('--location',args)
                self.assertEqual(args[-1],'https://'+host+'/realms/'+install.R+'/.well-known/openid-configuration')

    def test_discovery_security_or_semantic_failure_is_never_retried(self):
        import subprocess
        d=self.config();host=d['hosts']['voting']
        bad=[subprocess.CompletedProcess([],code,b'\n000',b'private error') for code in (6,35,51,60,77)]+[
            subprocess.CompletedProcess([],0,body,b'') for body in (b'{}\n500',b'{}\n404',b'{}\n302',b'{"issuer":"https://other.invalid"}\n200',b'invalid-json\n200',b'[]\n200',b'no-status')]
        for result in bad:
            with self.subTest(result=result),patch('install.subprocess.run',return_value=result) as call,patch('install.time.sleep') as sleep:
                with self.assertRaises(ValueError):install.verify_discovery(d,host,install.R)
                call.assert_called_once();sleep.assert_not_called()

    def test_discovery_startup_exhaustion_has_short_total_bound(self):
        import subprocess
        with patch('install.subprocess.run',return_value=subprocess.CompletedProcess([],0,b'unavailable\n503',b'')) as call,patch('install.time.sleep') as sleep:
            d=self.config()
            with self.assertRaisesRegex(ValueError,'did not become ready'):install.verify_discovery(d,d['hosts']['voting'],install.R)
            self.assertEqual(call.call_count,12);self.assertEqual(sleep.call_count,11)
            self.assertLessEqual(call.call_count*call.call_args.kwargs['timeout']+sleep.call_count,60)

    def test_metadata_api_waits_for_startup_without_retrying_inconsistency(self):
        with patch('install.ssh',side_effect=[ValueError('not listening'),b'{"is_consistent":true,"inconsistent_count":0}']) as ssh,patch('install.time.sleep') as sleep:
            install.verify_metadata(self.config())
            self.assertEqual(ssh.call_count,2);sleep.assert_called_once_with(2)
        with patch('install.ssh',side_effect=ValueError('not listening')) as ssh,patch('install.time.sleep') as sleep:
            with self.assertRaisesRegex(ValueError,'did not become ready'):install.verify_metadata(self.config())
            self.assertEqual(ssh.call_count,20);self.assertEqual(sleep.call_count,19)
            self.assertEqual(ssh.call_args.kwargs['timeout'],5)
        with patch('install.ssh',return_value=b'{"is_consistent":false,"inconsistent_count":196}') as ssh,patch('install.time.sleep') as sleep:
            with self.assertRaisesRegex(ValueError,'metadata is inconsistent'):install.verify_metadata(self.config())
            ssh.assert_called_once();sleep.assert_not_called()

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

    def test_check_refuses_a_workstation_without_a_required_tool(self):
        with patch('install.shutil.which',side_effect=lambda tool:None if tool=='openssl' else '/usr/bin/'+tool):
            with self.assertRaisesRegex(ValueError,'lacks these tools: openssl'):install.check_workstation()

    def test_check_refuses_an_old_openssl(self):
        with patch('install.shutil.which',return_value='/usr/bin/tool'),patch('install.run',return_value=b'OpenSSL 1.1.1w  11 Sep 2023'):
            with self.assertRaisesRegex(ValueError,'needs OpenSSL 3'):install.check_workstation()
        with patch('install.shutil.which',return_value='/usr/bin/tool'),patch('install.run',return_value=b'OpenSSL 3.0.13 30 Jan 2024'):
            install.check_workstation()

    def test_oci_profile_needs_no_user_created_archive_hashes(self):
        d=self.config();d['toe'].pop('archive_sha256');d['toe']['format']='oci'
        install.validate(d)
        d['toe']['archive_sha256']={part:'a'*64 for part in install.PARTS}
        with self.assertRaisesRegex(ValueError,'unknown field'):install.validate(d)

    HOSTS={'ADMIN_HOST':'admin.example.test','PORTAL_HOST':'voting.example.test','VERIFIER_HOST':'verify.example.test'}
    HOST_ARGS=['--arg','admin','admin.example.test','--arg','portal','voting.example.test','--arg','verifier','verify.example.test']
    FIRST_LOGIN_FLOW={'alias':'certificate-first-login-flow','topLevel':True,'providerId':'basic-flow','authenticationExecutions':[
        {'authenticator':'idp-detect-existing-broker-user','requirement':'REQUIRED','priority':0},{'authenticator':'idp-auto-link','requirement':'REQUIRED','priority':1}]}

    def host_env(self):
        import os
        return dict(os.environ,**self.HOSTS)

    def realm_fixture(self,kind):
        profile={'attributes':[{'name':name,'permissions':{'view':['admin','user'],'edit':['admin']}} for name in ('username','email','firstName')]}
        fixture={'sslRequired':'external','ssoSessionIdleTimeout':900,'bruteForceProtected':True,'failureFactor':5,'eventsEnabled':True,'registrationAllowed':False,'resetPasswordAllowed':False,'rememberMe':False,'eventsListeners':['custom-event-listener'],'identityProviders':[{'alias':'external-login','firstBrokerLoginFlowAlias':'certificate-first-login-flow','config':{} if kind=='event-template' else {'passMaxAge':'true'}}],'groups':[],
            'requiredActions':[{'alias':alias,'enabled':False,'defaultAction':False} for alias in ('UPDATE_PASSWORD','delete_credential','webauthn-register-passwordless','idp_link')],
            'defaultRole':{'name':'default-roles-synthetic'},'roles':{'realm':[{'name':'default-roles-synthetic','composites':{'realm':['uma_authorization'],'client':{'account':['view-profile']}}}]},
            'components':{'org.keycloak.userprofile.UserProfileProvider':[{'providerId':'declarative-user-profile','config':{'kc.user.profile.config':[json.dumps(profile)]}}]}}
        fixture['adminEventsEnabled']=fixture['adminEventsDetailsEnabled']=kind!='event-template'
        fixture.update(browserFlow='sequent browser flow',authenticatorConfig=[{'alias':'redirect-to-idp','config':{'defaultProvider':'external-login'}}],authenticationFlows=[
            {'alias':'sequent browser flow','authenticationExecutions':[{'authenticator':'identity-provider-redirector','requirement':'ALTERNATIVE','authenticatorConfig':'redirect-to-idp'},
                {'flowAlias':'sequent browser flow forms','requirement':'DISABLED'}]},
            {'alias':'sequent browser flow forms','authenticationExecutions':[{'authenticator':'auth-username-password-form','requirement':'REQUIRED'}]},
            {'alias':'basic / silver condition','authenticationExecutions':[{'flowAlias':'message channel OTP','requirement':'DISABLED'}]},
            copy.deepcopy(self.FIRST_LOGIN_FLOW)])
        uris={'admin-portal':['https://'+self.HOSTS['ADMIN_HOST']+'/*'],'voting-portal':['https://'+self.HOSTS['PORTAL_HOST']+'/*','https://'+self.HOSTS['VERIFIER_HOST']+'/*']}
        fixture['clients']=[{'clientId':name,'publicClient':True,'standardFlowEnabled':True,'implicitFlowEnabled':False,'directAccessGrantsEnabled':False,'redirectUris':uris[name],'webOrigins':['+'],
            'attributes':{'pkce.code.challenge.method':'S256','post.logout.redirect.uris':'+'}} for name in (['voting-portal'] if kind=='event-template' else ['admin-portal','voting-portal'])]
        if kind!='event-template':
            machine={'publicClient':False,'standardFlowEnabled':False,'implicitFlowEnabled':False,'redirectUris':[],'webOrigins':[],
                'attributes':{'oauth2.device.authorization.grant.enabled':'false','oidc.ciba.grant.enabled':'false'}}
            fixture['clients']+=[dict(copy.deepcopy(machine),clientId='api-key-client',directAccessGrantsEnabled=True,serviceAccountsEnabled=False,
                    protocolMappers=[{'name':'x-hasura-default-role','config':{'claim.value':'trustee-ceremony'}}]),
                dict(copy.deepcopy(machine),clientId='service-account',directAccessGrantsEnabled=False,serviceAccountsEnabled=True,authorizationServicesEnabled=False)]
            fixture['clients'][0]['protocolMappers']=[{'name':'board-authentication-time','protocolMapper':'oidc-usersessionmodel-note-mapper',
                'config':{'user.session.note':'AUTH_TIME','claim.name':'auth_time','jsonType.label':'long','id.token.claim':'true','access.token.claim':'true'}}]
            fixture['clients'].append({'clientId':'cli-account-admin','enabled':False,'directAccessGrantsEnabled':False})
            fixture['users']=[{'username':name,'groups':['/trustee']} for name in ('trustee1','trustee2')]
            if kind=='tenant':
                for user in fixture['users']:user['credentials']=[{'type':'password','value':'synthetic','temporary':False}]
        else:
            fixture['clients']+=[{'clientId':name,'enabled':False} for name in ('results-portal','onsite-voting-portal','vp-sso','ivr-service','ivr-voting','voting-portal-certs')]
        fixture['clients'].append({'clientId':'admin-cli','publicClient':True,'directAccessGrantsEnabled':False})
        fixture['clients']+=[{'clientId':name,'enabled':False} for name in ('security-admin-console','account','account-console')]
        return fixture

    def test_realm_assertions_respect_voter_and_administrator_scope(self):
        import subprocess
        for kind in ('tenant','tenant-template','event-template'):
            fixture=self.realm_fixture(kind)
            args=['jq','-e','--arg','kind',kind,'--arg','alias','external-login',*self.HOST_ARGS,install.REALM_CHECK]
            result=subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True)
            self.assertEqual(result.returncode,0,(kind,result.stderr))
            fixture['clients'][0]['directAccessGrantsEnabled']=True
            self.assertNotEqual(subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True).returncode,0)
            fixture['clients']=[]
            self.assertNotEqual(subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True).returncode,0)

    def test_realm_assertions_require_disabled_actions_account_and_self_service(self):
        import subprocess
        def profile_edit(name,edit):
            def change(fixture):
                component=fixture['components']['org.keycloak.userprofile.UserProfileProvider'][0]['config']['kc.user.profile.config']
                profile=json.loads(component[0])
                for attribute in profile['attributes']:
                    if attribute['name']==name:attribute['permissions']['edit']=edit
                component[0]=json.dumps(profile)
            return change
        mutations={
            'enabled required action':lambda f:f['requiredActions'][0].update(enabled=True),
            'default required action':lambda f:f['requiredActions'][1].update(defaultAction=True),
            'required action without enabled flag':lambda f:f['requiredActions'][2].pop('enabled'),
            'no required-action list':lambda f:f.pop('requiredActions'),
            'enabled account client':lambda f:f['clients'][-2].update(enabled=True),
            'missing account console':lambda f:f['clients'].pop(),
            'manage-account default role':lambda f:f['roles']['realm'][0]['composites']['client']['account'].append('manage-account'),
            'missing default role':lambda f:f['roles'].update(realm=[]),
            'user-editable username':profile_edit('username',['admin','user']),
            'user-editable email':profile_edit('email',['admin','user']),
            'unrestricted email':profile_edit('email',None),
            'missing user profile':lambda f:f.pop('components'),
            'registration allowed':lambda f:f.update(registrationAllowed=True),
            'password reset allowed':lambda f:f.update(resetPasswordAllowed=True),
        }
        for kind in ('tenant','tenant-template','event-template'):
            args=['jq','-e','--arg','kind',kind,'--arg','alias','external-login',*self.HOST_ARGS,install.REALM_CHECK]
            self.assertEqual(subprocess.run(args,input=json.dumps(self.realm_fixture(kind)),text=True,capture_output=True).returncode,0,kind)
            for name,mutate in mutations.items():
                fixture=self.realm_fixture(kind);mutate(fixture)
                with self.subTest(kind=kind,mutation=name):
                    self.assertNotEqual(subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True).returncode,0)

    def test_realm_assertions_restrict_machine_clients_console_and_offline_access(self):
        import subprocess
        def client(fixture,name):
            return next(c for c in fixture['clients'] if c['clientId']==name)
        def without(name):
            return lambda f:f.update(clients=[c for c in f['clients'] if c['clientId']!=name])
        def setting(name,**values):
            return lambda f:client(f,name).update(values)
        def attribute(name,key,value):
            return lambda f:client(f,name)['attributes'].update({key:value})
        every={
            'enabled administration console':setting('security-admin-console',enabled=True),
            'missing administration console':without('security-admin-console'),
            'missing identity-provider link action':lambda f:f.update(requiredActions=[a for a in f['requiredActions'] if a['alias']!='idp_link']),
            'offline default role':lambda f:f['roles']['realm'][0]['composites']['realm'].append('offline_access'),
            'offline composite role':lambda f:f['roles']['realm'].append({'name':'synthetic','composites':{'realm':['offline_access']}}),
            'offline client role':lambda f:f['roles'].update(client={'voting-portal':[{'name':'synthetic','composites':{'realm':['offline_access']}}]}),
            'offline realm optional scope':lambda f:f.update(defaultOptionalClientScopes=['address','offline_access']),
            'offline realm default scope':lambda f:f.update(defaultDefaultClientScopes=['offline_access']),
            'offline client optional scope':lambda f:f['clients'][0].update(optionalClientScopes=['offline_access']),
            'offline client default scope':lambda f:f['clients'][0].update(defaultClientScopes=['offline_access']),
            'offline group':lambda f:f.update(groups=[{'name':'voter','realmRoles':['offline_access']}]),
            'offline user':lambda f:f.setdefault('users',[]).append({'username':'synthetic','realmRoles':['offline_access']}),
        }
        staff={
            'trustee client standard flow':setting('api-key-client',standardFlowEnabled=True),
            'trustee client implicit flow':setting('api-key-client',implicitFlowEnabled=True),
            'trustee client credentials':setting('api-key-client',serviceAccountsEnabled=True),
            'trustee client without password grant':setting('api-key-client',directAccessGrantsEnabled=False),
            'trustee client public':setting('api-key-client',publicClient=True),
            'trustee client redirect':setting('api-key-client',redirectUris=['*']),
            'trustee client origin':setting('api-key-client',webOrigins=['*']),
            'trustee client logout redirect':attribute('api-key-client','post.logout.redirect.uris','+'),
            'trustee client device grant':attribute('api-key-client','oauth2.device.authorization.grant.enabled','true'),
            'trustee client administrator role':lambda f:client(f,'api-key-client')['protocolMappers'][0]['config'].update({'claim.value':'admin-user'}),
            'trustee client without role claim':setting('api-key-client',protocolMappers=[]),
            'trustee client service user':lambda f:f.setdefault('users',[]).append({'username':'service-account-api-key-client','serviceAccountClientId':'api-key-client'}),
            'missing trustee client':without('api-key-client'),
            'service client standard flow':setting('service-account',standardFlowEnabled=True),
            'service client password grant':setting('service-account',directAccessGrantsEnabled=True),
            'service client without client credentials':setting('service-account',serviceAccountsEnabled=False),
            'service client redirect':setting('service-account',redirectUris=['/*']),
            'service client authorization services':setting('service-account',authorizationServicesEnabled=True),
            'service client backchannel grant':attribute('service-account','oidc.ciba.grant.enabled','true'),
            'service client token exchange':attribute('service-account','standard.token.exchange.enabled','true'),
            'missing service client':without('service-account'),
        }
        for kind in ('tenant','tenant-template','event-template'):
            args=['jq','-e','--arg','kind',kind,'--arg','alias','external-login',*self.HOST_ARGS,install.REALM_CHECK]
            self.assertEqual(subprocess.run(args,input=json.dumps(self.realm_fixture(kind)),text=True,capture_output=True).returncode,0,kind)
            for name,mutate in (every if kind=='event-template' else dict(every,**staff)).items():
                fixture=self.realm_fixture(kind);mutate(fixture)
                with self.subTest(kind=kind,mutation=name):
                    self.assertNotEqual(subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True).returncode,0)

    def test_realm_filter_restricts_machine_clients_console_and_offline_access(self):
        import shutil,subprocess
        commands=Path(__file__).resolve().parent/'commands'
        with tempfile.TemporaryDirectory() as folder:
            folder=Path(folder)
            shutil.copy(commands/'07-015.jq',folder/'board.jq')
            (folder/'idp.json').write_text(json.dumps({'alias':'external-login','providerId':'oidc','enabled':True,'firstBrokerLoginFlowAlias':'certificate-first-login-flow','config':{}}))
            (folder/'flow.json').write_text(json.dumps(self.FIRST_LOGIN_FLOW))
            def checked(kind,realm):
                return subprocess.run(['jq','-e','--arg','kind',kind,'--arg','alias','external-login',*self.HOST_ARGS,install.REALM_CHECK],input=json.dumps(realm),text=True,capture_output=True).returncode
            def filtered(kind,realm):
                return json.loads(subprocess.run(['jq','-e','-L',str(folder),'--arg','doc',kind,'--slurpfile','idp',str(folder/'idp.json'),'--slurpfile','flow',str(folder/'flow.json'),'-f',str(commands/'07-016.jq')],input=json.dumps(realm),env=self.host_env(),text=True,capture_output=True,check=True).stdout)
            unrestricted={'publicClient':False,'standardFlowEnabled':True,'implicitFlowEnabled':False,'directAccessGrantsEnabled':True,'serviceAccountsEnabled':True,
                'redirectUris':['*'],'webOrigins':['*'],'attributes':{'post.logout.redirect.uris':'+'},'optionalClientScopes':['offline_access','address']}
            for kind in ('tenant','tenant-template','event-template'):
                delivered=self.realm_fixture(kind);delivered.update(defaultOptionalClientScopes=['offline_access','address'])
                delivered['requiredActions']=[a for a in delivered['requiredActions'] if a['alias']!='idp_link']
                delivered['roles']['realm'][0]['composites']['realm'].append('offline_access')
                for c in delivered['clients']:
                    if c['clientId'] in ('admin-portal','voting-portal'):c['optionalClientScopes']=['offline_access','address']
                    if c['clientId']=='security-admin-console':c['enabled']=True
                if kind!='event-template':
                    delivered['groups']=[{'name':'admin-light','realmRoles':['offline_access','monitoring-view']}]
                    delivered['clients']=[c for c in delivered['clients'] if c['clientId'] not in ('api-key-client','service-account')]+[
                        dict(copy.deepcopy(unrestricted),clientId='api-key-client',authenticationFlowBindingOverrides={'browser':'synthetic'},
                            protocolMappers=[{'name':'x-hasura-default-role','config':{'claim.value':'admin-user'}}]),
                        dict(copy.deepcopy(unrestricted),clientId='service-account',redirectUris=['/*'],authorizationServicesEnabled=True,authorizationSettings={'resources':[]})]
                    delivered['users']+=[{'username':'service-account-'+name,'serviceAccountClientId':name} for name in ('api-key-client','service-account')]
                with self.subTest(kind=kind):
                    self.assertNotEqual(checked(kind,delivered),0)
                    installed=filtered(kind,delivered)
                    self.assertEqual(checked(kind,installed),0)
                    self.assertEqual([c.get('optionalClientScopes') for c in installed['clients'] if c['clientId']=='voting-portal'],[['address']])
                    if kind!='event-template':
                        self.assertEqual([u['username'] for u in installed['users']],['trustee1','trustee2','service-account-service-account'])

    def test_realm_assertions_cover_the_sign_in_settings_of_the_transformation(self):
        import subprocess
        def client(fixture,name):
            return next(c for c in fixture['clients'] if c['clientId']==name)
        def execution(fixture,alias,key,value):
            flow=next(f for f in fixture['authenticationFlows'] if f['alias']==alias)
            return next(e for e in flow['authenticationExecutions'] if e.get(key)==value)
        every={
            'personal account with a password':lambda f:f.setdefault('users',[]).append({'username':'synthetic','credentials':[{'type':'password','value':'synthetic'}]}),
            'prohibited permission in another group':lambda f:f.update(groups=[{'name':'voter','realmRoles':['voter-manually-verify']}]),
            'prohibited permission in a subgroup':lambda f:f.update(groups=[{'name':'staff','realmRoles':[],'subGroups':[{'name':'synthetic','realmRoles':['datafix-account']}]}]),
            'prohibited permission of a user':lambda f:f.setdefault('users',[]).append({'username':'synthetic','realmRoles':['publish-results-write']}),
            'local forms enabled':lambda f:execution(f,'sequent browser flow','flowAlias','sequent browser flow forms').update(requirement='ALTERNATIVE'),
            'another browser flow':lambda f:f.update(browserFlow='browser'),
            'messaging code step enabled':lambda f:execution(f,'basic / silver condition','flowAlias','message channel OTP').update(requirement='CONDITIONAL'),
            'redirect to another provider':lambda f:f['authenticatorConfig'][0]['config'].update(defaultProvider='synthetic'),
            'redirector without the provider setting':lambda f:execution(f,'sequent browser flow','authenticator','identity-provider-redirector').pop('authenticatorConfig'),
            'provider prompt set':lambda f:f['identityProviders'][0]['config'].update(prompt='consent'),
            'admin-cli password grant':lambda f:client(f,'admin-cli').update(directAccessGrantsEnabled=True),
            'missing admin-cli':lambda f:f.update(clients=[c for c in f['clients'] if c['clientId']!='admin-cli']),
            'enabled cli-account-admin':lambda f:f['clients'].append({'clientId':'cli-account-admin','enabled':True,'directAccessGrantsEnabled':False}),
            'cli-account-admin password grant':lambda f:f['clients'].append({'clientId':'cli-account-admin','enabled':False,'directAccessGrantsEnabled':True}),
            'enabled results client':lambda f:f['clients'].append({'clientId':'results-portal','enabled':True}),
            'enabled telephone client':lambda f:f['clients'].append({'clientId':'ivr-voting','enabled':True}),
            'smart-link flow':lambda f:f['authenticationFlows'].append({'alias':'smart-link','authenticationExecutions':[]}),
            'smart-link execution':lambda f:f['authenticationFlows'].append({'alias':'synthetic','authenticationExecutions':[{'authenticator':'smart-link','requirement':'REQUIRED'}]}),
        }
        staff={
            'no maximum authentication age':lambda f:f['identityProviders'][0]['config'].pop('passMaxAge'),
            'missing authentication-time claim':lambda f:client(f,'admin-portal').update(protocolMappers=[]),
            'authentication time from another note':lambda f:client(f,'admin-portal')['protocolMappers'][0]['config'].update({'user.session.note':'synthetic'}),
        }
        event={'messaging channels configured':lambda f:f.update(attributes={'sequent.messaging':'synthetic'})}
        for kind in ('tenant','tenant-template','event-template'):
            args=['jq','-e','--arg','kind',kind,'--arg','alias','external-login',*self.HOST_ARGS,install.REALM_CHECK]
            self.assertEqual(subprocess.run(args,input=json.dumps(self.realm_fixture(kind)),text=True,capture_output=True).returncode,0,kind)
            mutations=dict(every,**(event if kind=='event-template' else staff))
            if kind=='tenant':mutations['missing trustee password']=lambda f:f['users'][1].pop('credentials')
            for name,mutate in mutations.items():
                fixture=self.realm_fixture(kind);mutate(fixture)
                with self.subTest(kind=kind,mutation=name):
                    self.assertNotEqual(subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True).returncode,0)

    def test_realm_assertions_cover_first_sign_in_and_portal_addresses(self):
        import shlex,subprocess
        def client(fixture,name):
            return next(c for c in fixture['clients'] if c['clientId']==name)
        def first_login(fixture):
            return next(f for f in fixture['authenticationFlows'] if f['alias']=='certificate-first-login-flow')
        every={
            'provider without the first-login flow':lambda f:f['identityProviders'][0].pop('firstBrokerLoginFlowAlias'),
            'provider with the default first-login flow':lambda f:f['identityProviders'][0].update(firstBrokerLoginFlowAlias='first broker login'),
            'missing first-login flow':lambda f:f.update(authenticationFlows=[x for x in f['authenticationFlows'] if x['alias']!='certificate-first-login-flow']),
            'second first-login flow':lambda f:f['authenticationFlows'].append(copy.deepcopy(first_login(f))),
            'first-login flow creates accounts':lambda f:first_login(f)['authenticationExecutions'].append({'authenticator':'idp-create-user-if-unique','requirement':'ALTERNATIVE','priority':2}),
            'first-login flow without the account lookup':lambda f:first_login(f)['authenticationExecutions'].pop(0),
            'optional account lookup':lambda f:first_login(f)['authenticationExecutions'][0].update(requirement='ALTERNATIVE'),
            'link before the account lookup':lambda f:first_login(f)['authenticationExecutions'][1].update(priority=-1),
            'first-login flow not top level':lambda f:first_login(f).update(topLevel=False),
            'voting portal implicit flow':lambda f:client(f,'voting-portal').update(implicitFlowEnabled=True),
            'voting portal without the code flow':lambda f:client(f,'voting-portal').update(standardFlowEnabled=False),
            'confidential voting portal':lambda f:client(f,'voting-portal').update(publicClient=False),
            'voting portal redirect to any address':lambda f:client(f,'voting-portal').update(redirectUris=['*']),
            'voting portal redirect to another host':lambda f:client(f,'voting-portal')['redirectUris'].append('https://other.example.test/*'),
            'voting portal redirect without TLS':lambda f:client(f,'voting-portal').update(redirectUris=['http://voting.example.test/*','https://verify.example.test/*']),
            'voting portal any origin':lambda f:client(f,'voting-portal').update(webOrigins=['*']),
            'voting portal any logout address':lambda f:client(f,'voting-portal')['attributes'].update({'post.logout.redirect.uris':'*'}),
            'voting portal device grant':lambda f:client(f,'voting-portal')['attributes'].update({'oauth2.device.authorization.grant.enabled':'true'}),
            'other client redirecting elsewhere':lambda f:f['clients'].append({'clientId':'synthetic','redirectUris':['https://other.example.test/*']}),
        }
        staff={
            'administration portal redirect to the voting host':lambda f:client(f,'admin-portal').update(redirectUris=['https://voting.example.test/*']),
            'administration portal implicit flow':lambda f:client(f,'admin-portal').update(implicitFlowEnabled=True),
        }
        event={'event client redirecting to the administration host':lambda f:f['clients'].append({'clientId':'synthetic','redirectUris':['https://admin.example.test/*']})}
        def run(kind,fixture,hosts=None):
            args=['jq','-e','--arg','kind',kind,'--arg','alias','external-login',*(self.HOST_ARGS if hosts is None else hosts),install.REALM_CHECK]
            return subprocess.run(args,input=json.dumps(fixture),text=True,capture_output=True).returncode
        def hosts(admin='admin.example.test',portal='voting.example.test',verifier='verify.example.test'):
            return ['--arg','admin',admin,'--arg','portal',portal,'--arg','verifier',verifier]
        for kind in ('tenant','tenant-template','event-template'):
            self.assertEqual(run(kind,self.realm_fixture(kind)),0,kind)
            disabled=self.realm_fixture(kind);disabled['clients'].append({'clientId':'synthetic','enabled':False,'redirectUris':['https://other.example.test/*']})
            self.assertEqual(run(kind,disabled),0,kind)
            for name,mutate in dict(every,**(event if kind=='event-template' else staff)).items():
                fixture=self.realm_fixture(kind);mutate(fixture)
                with self.subTest(kind=kind,mutation=name):
                    self.assertNotEqual(run(kind,fixture),0)
            other=[hosts(portal='other.example.test'),hosts(verifier='other.example.test'),hosts(admin=''),hosts(portal=''),hosts(verifier='')]
            if kind!='event-template':other.append(hosts(admin='other.example.test'))
            for site in other:
                with self.subTest(kind=kind,hosts=site):
                    self.assertNotEqual(run(kind,self.realm_fixture(kind),site),0)
        self.assertIn('--arg admin "$ADMIN_HOST" --arg portal "$PORTAL_HOST" --arg verifier "$VERIFIER_HOST" '+shlex.quote(install.REALM_CHECK),install.install_script(self.config()))

    def test_board_role_check_covers_administrator_groups(self):
        import shlex,shutil,subprocess
        with tempfile.TemporaryDirectory() as folder:
            shutil.copy(Path(__file__).resolve().parent/'commands'/'07-015.jq',Path(folder)/'board.jq')
            def checked(groups):
                return subprocess.run(['jq','-e','-L',folder,install.BOARD_GROUP_CHECK],input=json.dumps({'groups':groups}),text=True,capture_output=True).returncode
            self.assertEqual(checked([]),0)
            self.assertEqual(checked([{'name':'admin','realmRoles':['user-read','tenant-write']},{'name':'election-board','realmRoles':['publish-write']}]),0)
            for groups in ([{'name':'admin','realmRoles':['publish-write']}],[{'name':'admin-light','realmRoles':['sign-open-voting']}],
                    [{'name':'admin','realmRoles':[],'subGroups':[{'name':'nested','realmRoles':['election-event-create']}]}]):
                with self.subTest(groups=groups):
                    self.assertNotEqual(checked(groups),0)
        self.assertIn('jq -e -L /root/deploy '+shlex.quote(install.BOARD_GROUP_CHECK),install.install_script(self.config()))

    def test_realm_filter_sets_the_checked_sign_in_settings(self):
        import shutil,subprocess
        commands=Path(__file__).resolve().parent/'commands'
        with tempfile.TemporaryDirectory() as folder:
            folder=Path(folder)
            shutil.copy(commands/'07-015.jq',folder/'board.jq')
            (folder/'idp.json').write_text(json.dumps({'alias':'external-login','providerId':'oidc','enabled':True,'firstBrokerLoginFlowAlias':'certificate-first-login-flow','config':{}}))
            (folder/'flow.json').write_text(json.dumps(self.FIRST_LOGIN_FLOW))
            def checked(kind,realm):
                return subprocess.run(['jq','-e','--arg','kind',kind,'--arg','alias','external-login',*self.HOST_ARGS,install.REALM_CHECK],input=json.dumps(realm),text=True,capture_output=True).returncode
            def board(realm):
                return subprocess.run(['jq','-e','-L',str(folder),install.BOARD_GROUP_CHECK],input=json.dumps(realm),text=True,capture_output=True).returncode
            def filtered(kind,realm):
                return json.loads(subprocess.run(['jq','-e','-L',str(folder),'--arg','doc',kind,'--slurpfile','idp',str(folder/'idp.json'),'--slurpfile','flow',str(folder/'flow.json'),'-f',str(commands/'07-016.jq')],input=json.dumps(realm),env=self.host_env(),text=True,capture_output=True,check=True).stdout)
            for kind in ('tenant','tenant-template','event-template'):
                delivered=self.realm_fixture(kind)
                delivered['identityProviders']=[{'alias':'digital-certificates','firstBrokerLoginFlowAlias':'first broker login','config':{'prompt':'consent'}}]
                delivered['authenticatorConfig']=[]
                for c in delivered['clients']:
                    if c['clientId'] in ('admin-portal','voting-portal'):
                        c.update(implicitFlowEnabled=True,redirectUris=['/*','http://127.0.0.1:3000/*'],webOrigins=['*']);c['attributes']['post.logout.redirect.uris']='*'
                if kind!='event-template':
                    delivered['authenticationFlows']=[f for f in delivered['authenticationFlows'] if f['alias']!='certificate-first-login-flow']
                for flow in delivered['authenticationFlows']:
                    for e in flow['authenticationExecutions']:
                        e.pop('authenticatorConfig',None)
                        if e.get('flowAlias'):e['requirement']='ALTERNATIVE'
                delivered['authenticationFlows']+=[{'alias':name,'authenticationExecutions':[{'authenticator':'smart-link','requirement':'REQUIRED'}]} for name in ('smart-link','smart-link forms')]
                for c in delivered['clients']:
                    if c['clientId'] in ('admin-cli','cli-account-admin'):c['directAccessGrantsEnabled']=True
                    if c['clientId'] in ('cli-account-admin','results-portal','ivr-service'):c['enabled']=True
                if kind=='event-template':
                    delivered['attributes']={'sequent.messaging':'synthetic'}
                else:
                    delivered['clients'][0]['protocolMappers']=[]
                    delivered['groups']=[{'name':'admin','realmRoles':['user-read','publish-write','datafix-account','voter-manually-verify','publish-results-write']}]
                    delivered['users'].append({'username':'admin','groups':['/admin'],'credentials':[{'type':'password','value':'synthetic'}]})
                with self.subTest(kind=kind):
                    self.assertNotEqual(checked(kind,delivered),0)
                    installed=filtered(kind,delivered)
                    self.assertEqual(checked(kind,installed),0)
                    self.assertEqual(board(installed),0)
                    if kind!='event-template':
                        self.assertNotEqual(board(delivered),0)
                        self.assertEqual(installed['groups'][0]['realmRoles'],['user-read'])
            event=self.realm_fixture('event-template')
            next(f for f in event['authenticationFlows'] if f['alias']=='certificate-first-login-flow')['authenticationExecutions'].append(
                {'authenticator':'idp-create-user-if-unique','requirement':'ALTERNATIVE','priority':2})
            self.assertNotEqual(checked('event-template',filtered('event-template',event)),0)

    def test_realm_filter_leaves_registration_and_password_reset_to_the_delivered_realm(self):
        # The delivered realm files hold both settings; the installation only checks them.
        import shutil,subprocess
        commands=Path(__file__).resolve().parent/'commands'
        with tempfile.TemporaryDirectory() as folder:
            folder=Path(folder)
            shutil.copy(commands/'07-015.jq',folder/'board.jq')
            (folder/'idp.json').write_text(json.dumps({'alias':'external-login','providerId':'oidc','enabled':True,'firstBrokerLoginFlowAlias':'certificate-first-login-flow','config':{}}))
            (folder/'flow.json').write_text(json.dumps(self.FIRST_LOGIN_FLOW))
            def installed(kind,realm):
                filtered=subprocess.run(['jq','-e','-L',str(folder),'--arg','doc',kind,'--slurpfile','idp',str(folder/'idp.json'),'--slurpfile','flow',str(folder/'flow.json'),'-f',str(commands/'07-016.jq')],input=json.dumps(realm),env=self.host_env(),text=True,capture_output=True,check=True)
                return subprocess.run(['jq','-e','--arg','kind',kind,'--arg','alias','external-login',*self.HOST_ARGS,install.REALM_CHECK],input=filtered.stdout,text=True,capture_output=True).returncode
            for kind in ('tenant','tenant-template','event-template'):
                delivered=self.realm_fixture(kind)
                self.assertEqual(installed(kind,delivered),0,kind)
                for name in ('registrationAllowed','resetPasswordAllowed'):
                    with self.subTest(kind=kind,setting=name):
                        self.assertNotEqual(installed(kind,dict(delivered,**{name:True})),0)

    def test_config_rejects_unknown_keys_and_shell_host(self):
        d=self.config();install.validate(d)
        for mutation in [lambda x:x.update(secret='wrong'),lambda x:x['nodes'].update(c1='host;id'),lambda x:x['hosts'].update(voting='x$(id).test')]:
            candidate=copy.deepcopy(d);mutation(candidate)
            with self.assertRaises(ValueError):install.validate(candidate)
    def test_config_requires_every_archive_digest_distinct_nodes_and_public_inputs(self):
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

    def test_identity_provider_anchor_must_be_one_ecdsa_p384_certificate(self):
        import shutil,subprocess
        if not shutil.which('openssl'):self.skipTest('openssl is not installed')
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            def certificate(name,*key):
                subprocess.run(['openssl','req','-x509','-nodes','-days','1','-subj','/CN=synthetic test CA','-keyout',str(root/(name+'.key')),'-out',str(root/(name+'.pem'))]+list(key),check=True,capture_output=True)
                return root/(name+'.pem')
            p384=certificate('p384','-newkey','ec','-pkeyopt','ec_paramgen_curve:secp384r1')
            install.verify_auth_server_anchor(p384)
            for name,key,reason in [('p256',['-newkey','ec','-pkeyopt','ec_paramgen_curve:prime256v1'],'ECDSA P-384'),('rsa',['-newkey','rsa:3072'],'ECDSA P-384')]:
                with self.assertRaisesRegex(ValueError,reason):install.verify_auth_server_anchor(certificate(name,*key))
            bundle=root/'bundle.pem';bundle.write_bytes(p384.read_bytes()+certificate('second','-newkey','ec','-pkeyopt','ec_paramgen_curve:secp384r1').read_bytes())
            with self.assertRaisesRegex(ValueError,'exactly one certificate'):install.verify_auth_server_anchor(bundle)
            with_key=root/'with-key.pem';with_key.write_bytes(p384.read_bytes()+(root/'p384.key').read_bytes())
            with self.assertRaisesRegex(ValueError,'exactly one certificate'):install.verify_auth_server_anchor(with_key)

    SERVER_PARTS=['admin-portal','b4','ballot-verifier','braid','graphql-engine','harvest','keycloak','tls-endpoint','voting-portal','windmill']

    def test_the_digest_list_names_eleven_parts_and_ten_run_on_the_servers(self):
        self.assertEqual(list(install.PARTS),self.SERVER_PARTS)
        self.assertEqual(sorted(install.TOE_PARTS),sorted(self.SERVER_PARTS+['election-verifier']))

    def test_every_part_list_of_the_commands_and_helpers_is_the_ten_server_parts(self):
        import re
        parts=set(self.SERVER_PARTS)
        source=lambda name:re.sub(r'\\\n\s*',' ',(install.ROOT/name).read_text())
        groups=lambda text:[set(g.split('|')) for g in re.findall(r'\(((?:[a-z0-9-]+\|)+[a-z0-9-]+)\)',text) if 'harvest' in g.split('|')]
        loop=lambda text:set(re.search(r'for p in ([a-z0-9 -]+); do',text)[1].split())
        self.assertEqual(loop(source('commands/08-002.sh')),parts)
        for name in ('commands/08-002.sh','commands/10-001.sh','verification.py','integrity.py'):
            found=groups(source(name))
            self.assertTrue(found,name)
            for group in found:
                # The unpinned-image check matches every ballot image by its prefix.
                self.assertEqual(group-{'ballot','trustee'},parts-({'ballot-verifier'} if 'ballot' in group else set()),name)
        pinning=source('commands/08-003.sh')
        self.assertEqual(loop(pinning)|set(re.findall(r'\$\(d ([a-z0-9-]+)\)',pinning)),parts)
        self.assertEqual(groups(pinning),[parts-{'ballot-verifier'}|{'ballot','trustee'}])
        self.assertEqual(re.findall(r'-eq ([0-9]+)',source('commands/10-001.sh')),['10','10'])

    def test_installed_platform_check_and_image_readback_cover_the_ten_parts(self):
        scripts=[]
        with patch('install.ssh',side_effect=lambda d,node,script,label,timeout=0:scripts.append(script) or b''),patch('install.verify_metadata'),patch('install.verify_discovery'),patch('install.run',return_value=b'403'),patch('install.verify_login'),patch('builtins.print'):
            install.verify(self.config())
        self.assertIn('for part in '+' '.join(self.SERVER_PARTS)+'; do\n',scripts[0])
        self.assertNotIn('__PARTS__',scripts[0])
        with patch('install.ssh',side_effect=lambda d,node,script,label,timeout=0:scripts.append(script) or b''),patch('install.upload'),patch('install.run'),patch('install.install_script',return_value=''),patch('install.verify'),patch('install.cleanup_import'),patch('install.notification_config',return_value=b'{}'),patch('install.command_source',return_value=''),patch('builtins.print'),patch('install.Path.read_bytes',return_value=b''):
            install.install_prepared(self.config(),{p:Path('/nonexistent/'+p+'.tar') for p in self.SERVER_PARTS},{p:'a'*64 for p in self.SERVER_PARTS})
        readback=[s for s in scripts if 'images ls -q' in s]
        self.assertEqual(len(readback),2)
        for script in readback:
            self.assertIn('case "$part" in '+'|'.join(self.SERVER_PARTS)+')',script)
            for part in self.SERVER_PARTS:self.assertIn('  '+part+'.tar\n',script)

    def test_preflight_requires_port_443_free_and_runs_no_host_tls_endpoint(self):
        script=install.preflight_script(self.config(),True)
        self.assertIn("AGD_CHECK=TLS_ENDPOINT\ntest -z \"$(ss -H -ltn 'sport = :443')\"\n",script)
        self.assertNotIn('nginx',script)
        self.assertNotIn('TLS_ENDPOINT',install.preflight_script(self.config(),True,node='c2'))
        import subprocess
        with patch('install.subprocess.run',return_value=subprocess.CompletedProcess([],1,b'',b'AGD_CHECK_FAILED TLS_ENDPOINT\n')):
            with self.assertRaisesRegex(ValueError,'Port 443 of C1 must be free for the TLS endpoint'):install.run(['false'],'Node c1 preflight')

    def test_tls_inputs_are_installed_for_the_endpoint_workload_without_a_host_service(self):
        script=install.command_source('05-005',self.config())
        self.assertNotIn('nginx',script);self.assertNotIn('systemctl',script)
        self.assertIn('install -m 600 /root/deploy/tls/tls.key /var/lib/sequent-site/tls/tls.key',script)

    def test_a_delivery_without_every_server_part_is_refused_before_any_transfer(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);key=root/'key';key.write_text('private');key.chmod(0o600)
            d=self.config();d['ssh_identity']=str(key);d['toe']={'directory':str(root),'format':'oci','manifest_sha256':''}
            earlier=['b4','ballot-verifier','braid','election-verifier','harvest','keycloak','windmill']
            manifest=root/'image-digests.txt'
            manifest.write_text('tag: v'+d['release']['version']+'\ncommit: '+d['release']['step_commit']+'\n'+''.join(part+' sha256:'+'b'*64+'\n' for part in earlier))
            d['toe']['manifest_sha256']=install.digest(manifest)
            with patch('install.verify_kit'),patch('install.run') as run:
                with self.assertRaisesRegex(ValueError,'^Image digest manifest must contain exactly the eleven TOE parts$'):install.verify_inputs(d)
                run.assert_not_called()

    def master_secret_check(self,folder):
        # The early check of the installation script, with the deployment directory moved to a test folder.
        lines=install.install_script(self.config()).splitlines()
        i=lines.index('AGD_CHECK=MASTER_SECRET')
        head=[line for line in lines[:lines.index('AGD_STAGE=03-006')] if not line.startswith('. ')]
        return '\n'.join(head+['AGD_STAGE=07-005']+lines[i:i+2]+['echo checked',''])

    def test_generated_master_secret_is_64_hexadecimal_characters_and_new_each_time(self):
        import shutil,subprocess
        if not shutil.which('openssl'):self.skipTest('openssl is not installed')
        with tempfile.TemporaryDirectory() as folder:
            for name in ('a','b'):
                (Path(folder)/name).mkdir()
                r=subprocess.run(['bash','-e'],input=install.command_source('07-005',self.config()).replace('/root/deploy',folder+'/'+name),text=True,capture_output=True)
                self.assertEqual(r.returncode,0)
                self.assertTrue(r.stdout.startswith('600 '))
            # Compared in the shell, so no value is shown.
            check='. a/secrets.env; [[ $MASTER_SECRET =~ ^[0-9a-f]{64}$ ]]; first=$MASTER_SECRET; . b/secrets.env; [[ $MASTER_SECRET =~ ^[0-9a-f]{64}$ && $MASTER_SECRET != "$first" ]]'
            self.assertEqual(subprocess.run(['bash','-ec',check],cwd=folder,capture_output=True).returncode,0)

    def test_installation_stops_after_secret_generation_without_a_well_formed_master_secret(self):
        import os,shlex,subprocess
        lines=install.install_script(self.config()).splitlines()
        i=lines.index('AGD_CHECK=MASTER_SECRET')
        self.assertEqual(lines[i-1],'source /root/deploy/installation-commands/07-005.sh')
        self.assertEqual(lines[i+2],'AGD_STAGE=07-006')
        well_formed=['0123456789abcdef'*4,'0123456789ABCDEF'*4]
        malformed=[None,'','demonstration_master_secret','a'*63,'a'*65,'g'*64,' '+'a'*63,'missing file']
        expected='Hardened platform deployment failed: The generated master secret is missing or not 64 hexadecimal characters; stop and inspect this stage privately; see If something fails in the Deployment Guide'
        with tempfile.TemporaryDirectory() as folder:
            secrets=Path(folder)/'secrets.env'
            for value in well_formed+malformed:
                if secrets.exists():secrets.unlink()
                if value!='missing file':secrets.write_text('export POSTGRES_PASSWORD=0\n'+('' if value is None else 'export MASTER_SECRET='+shlex.quote(value)+'\n'))
                r=subprocess.run(['bash'],input=self.master_secret_check(folder).replace('/root/deploy',folder),text=True,capture_output=True,env={'PATH':os.environ['PATH']})
                if value in well_formed:
                    self.assertEqual((r.returncode,r.stdout,r.stderr),(0,'checked\n',''))
                    continue
                self.assertNotEqual(r.returncode,0,value)
                self.assertEqual(r.stdout,'')
                self.assertIn('AGD_CHECK_FAILED MASTER_SECRET\n',r.stderr)
                if value:self.assertNotIn(value,r.stderr)
                with patch('install.subprocess.run',return_value=subprocess.CompletedProcess([],r.returncode,b'',r.stderr.encode())):
                    with self.assertRaises(ValueError) as error:install.run(['false'],'Hardened platform deployment')
                self.assertEqual(str(error.exception),expected)

# A stand-in for k3s on C1: it records its arguments and runs the shell that
# `kubectl exec` would start in the Keycloak container, with a kcadm that
# records its own arguments and environment.
MOCK_EXEC_K3S = r"""#!/usr/bin/env python3
import json, os, subprocess, sys
args = sys.argv[1:]
with open(os.environ['MOCK_ARGS'], 'a') as record: record.write(json.dumps(args) + '\n')
shell = args.index('bash')
script = args[shell + 2].replace('/opt/keycloak/bin/kcadm.sh', os.environ['MOCK_KCADM'])
sys.exit(subprocess.run(['bash', '-c', script] + args[shell + 3:]).returncode)
"""
MOCK_KCADM = r"""#!/usr/bin/env python3
import json, os, sys
with open(os.environ['MOCK_ARGS'], 'a') as record: record.write(json.dumps(sys.argv[1:]) + '\n')
with open(os.environ['MOCK_PASSWORD'], 'w') as record: record.write(os.environ.get('KC_CLI_PASSWORD', ''))
"""


class KeycloakAdministratorPasswordTests(unittest.TestCase):
    def test_the_administrator_password_reaches_kcadm_on_no_command_line(self):
        import os, re, subprocess
        commands = Path(__file__).resolve().parent / 'commands'
        for ident, config in (('08-005', '/tmp/kcadm-admin-api.config'), ('08-008', '/tmp/kcadm.config'), ('08-009', '/tmp/kcadm-trustee-map.config')):
            text = (commands / (ident + '.sh')).read_text()
            with self.subTest(step=ident), tempfile.TemporaryDirectory() as folder:
                self.assertNotIn('--password', text)
                root = Path(folder)
                login = re.search(r"""printf '%s\\n' "\$KC_ADMIN_PASSWORD" \| .*?kcadm "\$KC_ADMIN_USER"( \\\n  > "\$W/auth\.log" 2>&1)?\n""", text, re.S)
                self.assertIsNotNone(login)
                (root / 'k3s').write_text(MOCK_EXEC_K3S); (root / 'k3s').chmod(0o755)
                (root / 'kcadm').write_text(MOCK_KCADM); (root / 'kcadm').chmod(0o755)
                password = 'synthetic administrator password 7'
                environment = dict(os.environ, PATH=folder + os.pathsep + os.environ['PATH'], MOCK_ARGS=str(root / 'args'), MOCK_KCADM=str(root / 'kcadm'), MOCK_PASSWORD=str(root / 'password'), KC_ADMIN_PASSWORD=password, KC_ADMIN_USER='service-administrator', W=folder)
                result = subprocess.run(['bash', '-c', 'set -Eeuo pipefail\n' + login[0]], env=environment, capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)
                calls = [json.loads(line) for line in (root / 'args').read_text().splitlines()]
                self.assertEqual(len(calls), 2)
                self.assertFalse([call for call in calls if any(password in argument for argument in call)])
                self.assertEqual(calls[1], ['config', 'credentials', '--server', 'http://localhost:8090', '--realm', 'master', '--user', 'service-administrator', '--config', config])
                self.assertEqual((root / 'password').read_text(), password)


SERVICE_MANIFEST = """spec:
  template:
    spec:
      initContainers:
        - name: wait-for-task-queues
          envFrom:
            - configMapRef:
                name: step-env
      containers:
        - name: application
          envFrom:
            - configMapRef:
                name: step-env
          env:
            - name: MASTER_SECRET
              valueFrom:
                secretKeyRef:
                  name: step-secrets
                  key: MASTER_SECRET
"""


class ServiceCredentialTests(unittest.TestCase):
    def test_the_service_credentials_reach_only_the_application_containers(self):
        import subprocess
        source = (Path(__file__).resolve().parent / 'commands' / '07-007.sh').read_text()
        with tempfile.TemporaryDirectory() as folder:
            names = ('harvest/deployment.yaml', 'windmill/deployment.yaml', 'windmill/beat.yaml')
            for name in names:
                path = Path(folder) / 'kubernetes' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(SERVICE_MANIFEST)
            script = source.replace('/root/deploy', folder)
            result = subprocess.run(['bash', '-c', 'set -Eeuo pipefail\n' + script], capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            for name in names:
                with self.subTest(name=name):
                    self.assertIn(name + ':1\n', result.stdout)
                    text = (Path(folder) / 'kubernetes' / name).read_text()
                    init, application = text.split('      containers:\n')
                    self.assertNotIn('keycloak-service-credentials', init)
                    self.assertIn('                name: step-env\n            - secretRef:\n                name: keycloak-service-credentials\n          env:\n', application)

    def test_the_operation_stops_when_a_manifest_offers_no_place_for_them(self):
        import subprocess
        source = (Path(__file__).resolve().parent / 'commands' / '07-007.sh').read_text()
        with tempfile.TemporaryDirectory() as folder:
            for name in ('harvest/deployment.yaml', 'windmill/deployment.yaml', 'windmill/beat.yaml'):
                path = Path(folder) / 'kubernetes' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(SERVICE_MANIFEST.split('      containers:\n')[0])
            result = subprocess.run(['bash', '-c', 'set -Eeuo pipefail\n' + source.replace('/root/deploy', folder)], capture_output=True, text=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)

if __name__=='__main__':unittest.main()
