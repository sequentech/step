#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Bounded, operator-facing checks of installed policy. Never a full ST self-test."""
import json
import re
from urllib.parse import urlencode
import commission
import install


def lines(value):
    return [line.strip() for line in value.splitlines() if line.strip()]


def remote(d, script, label, timeout=120):
    return install.ssh(d, 'c1', script, label, timeout=timeout).decode()


def check_event_rows(value, expected):
    rows=[line.split('|') for line in lines(value)]
    install.require(len(rows)==len(expected) and {r[0] for r in rows}==set(expected), 'Event realm policy result is absent, duplicated or unexpected')
    for row in rows:
        install.require(len(row)==7 and row[1:5]==['EXTERNAL','900','t','true'] and row[5].isdigit() and int(row[5])>0 and row[6]=='custom-event-listener', 'An event realm TLS, session, brute-force or audit policy differs')


def validate_event_name(name):
    install.require(isinstance(name,str) and re.fullmatch(r'tenant-[A-Za-z0-9-]+-event-[A-Za-z0-9-]+',name) is not None, 'Use an exact election-event realm name from the inspection result')


def check_signing_keys(published, realms):
    known={}
    fields=('kty','n','e','crv','x','y','alg','use')
    for realm in realms:
        install.require(isinstance(realm,dict) and isinstance(realm.get('keys'),list), 'Realm signing-key response is invalid')
        for key in realm['keys']:
            install.require(isinstance(key,dict) and isinstance(key.get('kid'),str) and key['kid'] and key.get('kty') in ('RSA','EC'), 'Realm signing key lacks its public identity')
            material={field:key[field] for field in fields if field in key}
            install.require(key['kid'] not in known or known[key['kid']]==material, 'Different realm keys share a signing-key identifier')
            known[key['kid']]=material
    install.require(isinstance(published,dict) and isinstance(published.get('keys'),list) and published['keys'], 'Published signing-key set is empty or invalid')
    seen=set()
    for key in published['keys']:
        install.require(isinstance(key,dict) and key.get('kid') not in seen and key.get('kid') in known, 'Published signing key is unknown or duplicated')
        material={field:key[field] for field in fields if field in key}
        install.require(material==known[key['kid']], 'Published key material differs from its realm key')
        seen.add(key['kid'])


def check_metadata_response(status, body):
    # Exact-path ingress may return its portal HTML fallback for an unknown path.
    refused=status==404 or (status==403 and not body.lstrip().startswith(('{','['))) or (status in (200,405) and body.lstrip().lower().startswith(('<!doctype html','<html')))
    install.require(refused, 'Metadata endpoint did not return a verified refusal or portal fallback')


def http(d, host, path, label, form=None, json_body=None):
    argv=['curl','--silent','--show-error','--cacert',d['tls']['ca'],'--max-time','15','-w','\n%{http_code}']
    if form is not None:argv+=['--data',urlencode(form)]
    if json_body is not None:argv+=['-H','Content-Type: application/json','--data',json.dumps(json_body)]
    raw=install.run(argv+['https://'+host+path],label,timeout=20).decode()
    body,status=raw.rsplit('\n',1)
    install.require(re.fullmatch(r'[1-5][0-9]{2}',status) is not None,'HTTPS check has no valid response status')
    return int(status),body


def document(d, host, path, label):
    status,body=http(d,host,path,label)
    install.require(status==200,label+' did not return HTTP 200')
    try:return json.loads(body)
    except (TypeError,ValueError):raise ValueError(label+' returned invalid JSON') from None


def external(d, realm):
    for host in (d['hosts'][name] for name in ('voting','administration','verifier')):
        status,_=http(d,host,'/realms/'+realm+'/election/x/public/login','Smart-link endpoint refusal')
        install.require(status==404,'Smart-link endpoint must return HTTP 404')
        status,body=http(d,host,'/hasura/v1/metadata','Metadata endpoint refusal',json_body={'type':'export_metadata','args':{}})
        check_metadata_response(status,body)
        headers=install.run(['curl','--silent','--show-error','--cacert',d['tls']['ca'],'--max-time','15','-o','/dev/null','-D','-','-H','Origin: https://other.invalid','https://'+host+'/hasura/v1/graphql'],'Foreign-origin policy',timeout=20).decode()
        install.require(re.search(r'^HTTP/\S+ [1-5][0-9]{2}',headers,re.M) is not None and re.search(r'^access-control-allow-origin:',headers,re.I|re.M) is None,'A foreign origin received cross-origin permission or no HTTP response')
    print('PASS: smart-link endpoint blocked; metadata hidden; foreign-origin access refused',flush=True)


def check_restrictions(value, customer):
    rows=lines(value);tenant=customer.removeprefix('tenant-');initial=install.R.removeprefix('tenant-')
    install.require(len(rows)==9 and rows[:3]==['0']*3 and set(rows[3:5])=={tenant,initial} and rows[5:7]==['0','0'] and set(rows[7:])=={install.R+'|t|t',customer+'|t|t'},'Tenant, prohibited-role, unused-channel or administration-audit restrictions differ')


def storage_script(d):
    body=install.command_source('10-015',d)
    body=body.replace('k3s kubectl apply -f /root/deploy/storage-check.yaml','k3s kubectl create -f /root/deploy/storage-check.yaml\ncreated=1')
    body=body.replace('k3s kubectl -n step-infra delete pod storage-check','')
    return '''existing=$(k3s kubectl -n step-infra get pod storage-check --ignore-not-found -o name)
test -z "$existing"
created=0
cleanup() { result=$?; trap - EXIT; if test "$created" = 1; then k3s kubectl -n step-infra delete pod storage-check --wait=true --timeout=30s >/dev/null || result=1; fi; exit "$result"; }
trap cleanup EXIT
'''+body


def inspect_storage(d):
    output=lines(remote(d,storage_script(d),'Storage policy inspection',timeout=180))
    # kubectl may prepend creation/wait messages; the five data lines are exact.
    values=[line.strip('"') for line in output if not line.startswith(('pod/','pod "'))]
    install.require(values==['0','AccessDenied','NoSuchBucketPolicy','1','Enabled'],'Plugin, public/private storage or checkpoint Object Lock policy differs')
    print('PASS: no plugins; private documents restricted; public policy exact; checkpoint Object Lock enabled',flush=True)


def event_list(d):
    output=remote(d,install.command_source('10-011',d),'Election realm listing')
    realms=lines(output)
    for realm in realms:validate_event_name(realm)
    install.require(len(realms)==len(set(realms)),'Election realm list contains duplicates')
    rows=remote(d,install.command_source('10-012',d),'Election realm policy')
    check_event_rows(rows,realms)
    return realms


def event(d,realm):
    validate_event_name(realm)
    customer=commission.customer(d,commission.validate(d.get('commissioning')))
    install.require(realm.startswith(customer+'-event-'),'Event realm is not in the configured customer tenant')
    install.require(realm in event_list(d),'Selected election realm does not exist')
    host=d['hosts']['voting'];prefix='/realms/'+realm
    channels=document(d,host,prefix+'/messaging-channels','Event code-channel policy')
    install.require(channels.get('code_channels')==[],'Event offers a sign-in code or message channel')
    discovery=document(d,host,prefix+'/.well-known/openid-configuration','Event issuer')
    install.require(discovery.get('issuer')=='https://'+host+prefix,'Event issuer differs from its configured realm')
    status,raw=http(d,host,prefix+'/protocol/openid-connect/token','Event password-grant refusal',{'grant_type':'password','client_id':'admin-cli','username':'agd-negative-check','password':'invalid-negative-check'})
    response=json.loads(raw)
    install.require(status in (400,401) and response.get('error')=='unauthorized_client' and 'access_token' not in response,'Event password grant is not refused')
    print('PASS: '+realm+' TLS/session/audit policy, issuer, absent code channels and disabled password grant',flush=True)


def platform(d):
    install.verify(d)
    customer=commission.customer(d,commission.validate(d.get('commissioning')))
    commission.verify(d,customer)
    images=lines(remote(d,install.command_source('10-001',d),'Running image content verification',timeout=180))
    observed={}
    for row in images:
        match=re.fullmatch(r'(b4|ballot-verifier|braid|harvest|keycloak|windmill) ([0-9]+) verified',row)
        install.require(match is not None and match[1] not in observed,'Running image check returned missing, repeated or unexpected output')
        observed[match[1]]=int(match[2])
    install.require(set(observed)==set(install.PARTS) and all(count >= (2 if part in ('braid','windmill') else 1) for part,count in observed.items()),'A required running TOE image is absent')
    print('PASS: all six server image contents match the accepted delivery; both trustees ready',flush=True)
    files=lines(remote(d,install.command_source('10-004',d),'Identity provider file inspection'))
    install.require(files and files[-1]=='0' and not any('smart-link-hmac' in value for value in files[:-1]),'Prohibited smart-link extension is installed')
    flows=lines(remote(d,install.command_source('10-005',d),'Used authentication flow inspection'))
    install.require(flows==['0'],'A used authentication flow includes smart-link')
    print('PASS: no smart-link extension or used authentication flow',flush=True)
    graphql=set(lines(remote(d,install.command_source('10-007',d),'GraphQL ingress policy')))
    install.require(graphql=={'/hasura/v1/graphql Exact','HASURA_GRAPHQL_ENABLE_CONSOLE false','HASURA_GRAPHQL_DEV_MODE false'},'GraphQL console, development mode or ingress path differs')
    cluster=lines(remote(d,install.command_source('10-009',d),'Cluster secret and service exposure policy'))
    install.require(cluster==['Encryption Status: Enabled','ClusterIP','0'],'Cluster secret encryption or service exposure differs; no external Service is permitted')
    print('PASS: exact GraphQL route; console disabled; encrypted Secrets; internal-only services',flush=True)
    external(d,customer)
    check_restrictions(remote(d,install.command_source('10-014',d),'Tenant and unused-function inspection'),customer)
    print('PASS: exactly two tenants; no installation personal login, prohibited roles or telephone channel',flush=True)
    inspect_storage(d)
    realms=event_list(d)
    install.require(all(name.startswith(customer+'-event-') for name in realms),'An event realm belongs to an unexpected tenant')
    host=d['hosts']['voting']
    known=[document(d,host,'/realms/'+realm+'/protocol/openid-connect/certs','Realm public signing keys') for realm in [install.R,customer]+realms]
    published=document(d,host,'/storage/public/certs.json','Published public signing keys')
    check_signing_keys(published,known)
    print('PASS: published signing keys match the public keys of this installation',flush=True)
    if not realms:print('PENDING: no election event exists; repeat inspection and event checks after each import',flush=True)
    for realm in realms:print('EVENT REALM: '+realm,flush=True)
    print('PLATFORM INSPECTION COMPLETE — browser/MFA, client-network, event checks and board handover remain separate.',flush=True)
