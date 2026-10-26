#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Guided customer/trustee commissioning; physical custody stays with the trustees."""
import copy
import hashlib
import json
from pathlib import Path
import re
import shlex
import install


def validate(c):
    install.keys(c,('customer_slug','administrator','trustees'),'commissioning')
    install.require(isinstance(c['customer_slug'],str) and re.fullmatch(r'[A-Za-z0-9_.-]+',c['customer_slug']),'Use a literal customer slug')
    install.keys(c['administrator'],('username','email'),'customer administrator')
    for value in c['administrator'].values():
        install.require(isinstance(value,str) and re.fullmatch(r'[A-Za-z0-9_@.+-]+',value),'Invalid customer administrator identifier')
    install.require('@' in c['administrator']['email'],'Customer administrator email required')
    install.require(isinstance(c['trustees'],list) and len(c['trustees'])==2,'Exactly two native trustee inputs required')
    names=[]
    for t in c['trustees']:
        install.keys(t,('name','keys_file'),'trustee input')
        install.require(isinstance(t['name'],str) and re.fullmatch(r'[A-Za-z0-9_.@ -]+',t['name']),'Use the exact registered trustee name without shell punctuation')
        install.require(isinstance(t['keys_file'],str) and Path(t['keys_file']).is_absolute(),'Trustee medium path must be absolute')
        names.append(t['name'])
    install.require(len(set(names))==2,'Trustee names must be distinct')
    return c


def ask(message):
    # /dev/tty prevents a pipe or captured stdin from silently authorizing custody.
    with open('/dev/tty','w') as output, open('/dev/tty','r') as response:
        output.write(message+'\nType CONTINUE after completing this action: ');output.flush()
        install.require(response.readline().strip()=='CONTINUE','Commissioning paused; no automatic continuation')


def sql(d,query,values):
    args=' '.join('-v '+shlex.quote(k+'='+v) for k,v in values.items())
    s='k3s kubectl -n step-infra exec -i deploy/postgres -- psql -U postgres -tA -d hasura -v ON_ERROR_STOP=1 '+args+" <<'AGD_SQL'\n"+query+'\nAGD_SQL\n'
    return install.ssh(d,'c1',s,'Commissioning database check').decode().strip()


def state(d):
    s='''if test -f /root/deploy/.commissioned; then printf complete;
elif test -f /root/deploy/.commission-imports-started; then printf partial;
elif test -f /root/deploy/.commission-accounts-ready; then printf accounts;
elif test -f /root/deploy/.commission-started; then printf partial;
else printf new; fi
'''
    return install.ssh(d,'c1',s,'Commissioning state').decode().strip()


def customer(d,c):
    rows=json.loads(sql(d,"SELECT coalesce(json_agg(json_build_object('id',id,'slug',slug)), '[]'::json) FROM sequent_backend.tenant;",{}))
    install.require(len(rows)==2,'Create exactly one customer tenant in the portal before commissioning')
    matches=[r for r in rows if r['slug']==c['customer_slug'] and r['id']!=install.R.removeprefix('tenant-')]
    install.require(len(matches)==1,'Configured customer slug is absent or ambiguous; create/select it in the portal')
    ident=matches[0]['id'];install.require(re.fullmatch(r'[0-9a-f-]{36}',ident) is not None,'Invalid customer tenant identity')
    return 'tenant-'+ident


def accounts(d,c,realm):
    values=copy.deepcopy(d);values['operator'].update(c['administrator'])
    body=install.command_source('08-008',values)
    config_hash=hashlib.sha256(json.dumps(c,sort_keys=True).encode()).hexdigest()
    script='test ! -e /root/deploy/.commission-started\nprintf started > /root/deploy/.commission-started\n'
    script+='printf %s '+shlex.quote(config_hash)+' > /root/deploy/.commission-config.sha256\n'
    script+='printf \'export CR=%s\\n\' '+shlex.quote(realm)+' >> /root/deploy/hosts.env\n'
    script+=body+'\nprintf ready > /root/deploy/.commission-accounts-ready\n'
    install.ssh(d,'c1',script,'Customer account creation',timeout=600)
    recorded=install.ssh(d,'c1','test -f /root/deploy/.commission-accounts-ready\ncat /root/deploy/.commission-config.sha256\n','Customer account completion readback').decode()
    install.require(recorded==config_hash,'Customer account creation did not complete with the configured identity; inspect partial state before continuing')


def public_keys(d,number,remote):
    image_hash=next(line.split()[1] for line in (Path(d['toe']['directory'])/'image-digests.txt').read_text().splitlines() if line.startswith('braid '))
    script='k3s ctr -n k8s.io run --rm --mount '+shlex.quote('type=bind,src='+remote+',dst=/keys,options=rbind:ro')+' '+shlex.quote('sequentech.local/braid@'+image_hash)+' '+shlex.quote('agd-public-key-check-'+str(number))+' /usr/bin/trustee public-keys --trustee-config /keys\n'
    return json.loads(install.ssh(d,'c1',script,'Trustee public-key derivation',timeout=120))


def import_trustee(d,c,realm,number,apply=False,expected_hash=None):
    trustee=c['trustees'][number-1]
    key=install.checked_path(trustee['keys_file'],private=True)
    if not apply:ask('Trustee '+str(number)+': be present and confirm this mounted medium contains your registered private keys. Keep your original medium in your custody.')
    body=key.read_bytes();key_hash=hashlib.sha256(body).hexdigest()
    install.require(expected_hash is None or key_hash==expected_hash,'Trustee medium changed after the two-key precheck')
    folder=install.ssh(d,'c1','mktemp -d /root/deploy/.commission-key.XXXXXXXX\n','Private trustee transfer directory').decode().strip()
    install.require(re.fullmatch(r'/root/deploy/\.commission-key\.[A-Za-z0-9]{8}',folder) is not None,'Unexpected private transfer directory')
    remote=folder+'/keys.toml'
    transfer='set -e; set +x; umask 077; set -C; cat > '+shlex.quote(remote)
    try:
        install.run(install.ssh_args(d,'c1')+['bash','-c',shlex.quote(transfer)],'Protected trustee key transfer',body)
        actual=public_keys(d,number,remote)
        registered=json.loads(sql(d,"SELECT coalesce(json_agg(json_build_object('signing_public_key',public_key,'share_encryption_public_key',share_encryption_public_key)), '[]'::json) FROM sequent_backend.trustee WHERE tenant_id = :'tenant' AND name = :'name';",{'tenant':realm.removeprefix('tenant-'),'name':trustee['name']}))
        install.require(len(registered)==1 and actual==registered[0],'Trustee private keys do not match the exact registered public-key pair; import refused')
        if not apply:return key_hash
        values=copy.deepcopy(d);values['_trustee']={'name':trustee['name'],'number':str(number)}
        script=install.command_source('08-009',values)
        script+='\nk3s kubectl -n step-infra create secret generic trustee'+str(number)+'-keys --from-file=trustee.toml='+shlex.quote(remote)+'\n'
        script+=install.command_source('08-011',values)
        install.ssh(d,'c1',script,'Native trustee '+str(number)+' binding and startup',timeout=600)
    finally:
        # The private original is never changed. Remove this transfer copy even on refusal.
        install.ssh(d,'c1','test ! -L '+shlex.quote(remote)+'\nif test -f '+shlex.quote(remote)+'; then shred -u '+shlex.quote(remote)+'; fi\nrmdir '+shlex.quote(folder)+'\n','Temporary trustee key cleanup')
    print('PASS: trustee '+str(number)+' public-key pair, native binding and startup; transfer copy removed',flush=True)


def main(d):
    c=validate(d.get('commissioning'))
    install.verify(d)
    realm=customer(d,c)
    digest=hashlib.sha256(json.dumps(c,sort_keys=True).encode()).hexdigest()
    current=state(d)
    install.require(current!='partial','A previous commissioning attempt stopped during writes. Inspect its actual state; automatic replay is refused')
    if current=='complete':
        recorded=install.ssh(d,'c1','cat /root/deploy/.commissioned\n','Commissioning identity').decode()
        install.require(recorded==digest,'Commissioning configuration differs from the completed record')
        verify(d,realm);return
    if current=='accounts':
        recorded=install.ssh(d,'c1','cat /root/deploy/.commission-config.sha256\n','Commissioning configuration identity').decode()
        install.require(recorded==digest,'Commissioning configuration changed after account creation; automatic continuation refused')
    if current=='new':
        uri='https://'+d['hosts']['voting']+'/realms/'+realm+'/broker/'+d['idp_alias']+'/endpoint'
        print('Register this exact customer redirect URI at the external identity provider: '+uri,flush=True)
        ask('Confirm the external identity provider registration and the customer administrator account are ready; no wildcard redirect URI is allowed.')
        accounts(d,c,realm)
        print('PASS: customer administrator and native accounts configured; installation personal accounts disabled',flush=True)
    ask('Sign in to the customer administration portal with the external administrator account. Register both trustees with their two public keys. Confirm that the registered names match site.json and both trustees have their own key media.')
    # A single attempt owns all native imports; a failed partial import is never replayed.
    hashes={number:import_trustee(d,c,realm,number) for number in (1,2)}
    print('PASS: both trustee media match their registered public-key pairs before either import',flush=True)
    install.ssh(d,'c1','test ! -e /root/deploy/.commission-imports-started\nprintf started > /root/deploy/.commission-imports-started\n','Native trustee import boundary')
    for number in (1,2):import_trustee(d,c,realm,number,apply=True,expected_hash=hashes[number])
    verify(d,realm)
    install.ssh(d,'c1','printf %s '+shlex.quote(digest)+' > /root/deploy/.commissioned\n','Commissioning record')
    print('COMMISSIONING COMMANDS COMPLETE — perform the browser, client-network and board handover checks in AGD-04.',flush=True)


def verify(d,realm):
    install.ssh(d,'c1',r'''k3s kubectl -n step-infra get deploy trustee1 trustee2 -o json | jq -e '.items | length==2 and all((.status.availableReplicas // 0)==1)' >/dev/null
''','Native trustee readiness')
    install.verify_login(d,realm)
    print('PASS: customer login restrictions and both native trustees Ready',flush=True)
