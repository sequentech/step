#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Configuration-driven first installation. No upgrades, retries or acceptance claims."""
import hashlib
import io
import ipaddress
import json
import os
from pathlib import Path
import re
import shlex
import stat
import subprocess
import tarfile
import tempfile
from urllib.parse import urlencode, urlsplit, parse_qs
from step_names import STEP_NAMES

ROOT = Path(__file__).resolve().parent
PARTS = ('b4', 'ballot-verifier', 'braid', 'harvest', 'keycloak', 'windmill')
R = 'tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5'

def require(ok, message):
    if not ok: raise ValueError(message)

def keys(value, expected, label):
    require(isinstance(value, dict) and set(value) == set(expected), label + ': missing or unknown field')

def validate(d):
    keys(d, ('schema','nodes','ssh_identity','hosts','idp_alias','kit','toe','tls','identity_provider','operator','release') + (('commissioning',) if isinstance(d,dict) and 'commissioning' in d else ()), 'configuration')
    require(d['schema'] == 1, 'Unsupported configuration schema')
    keys(d['nodes'], ('c1','c2'), 'nodes')
    for address in d['nodes'].values():
        require(isinstance(address,str), 'An IP address is required'); ipaddress.IPv4Address(address)
    require(d['nodes']['c1'] != d['nodes']['c2'], 'The two nodes must have different addresses')
    keys(d['hosts'], ('voting','administration','verifier','authentication'), 'hosts')
    for host in d['hosts'].values():
        require(isinstance(host,str) and re.fullmatch(r'[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?', host), 'Invalid hostname')
    require(len(set(d['hosts'].values())) == 4, 'Use four distinct service hostnames')
    require(isinstance(d['idp_alias'],str) and re.fullmatch(r'[A-Za-z0-9_-]+',d['idp_alias']), 'Invalid identity-provider alias')
    keys(d['kit'],('directory','signing_fingerprint'),'kit')
    require(re.fullmatch(r'SHA256:[A-Za-z0-9+/]{43}',d['kit']['signing_fingerprint']) is not None, 'Independently received kit fingerprint required')
    keys(d['toe'],('directory','archive_sha256','manifest_sha256'),'toe')
    keys(d['toe']['archive_sha256'],PARTS,'TOE archive hashes')
    keys(d['tls'],('certificate','private_key','ca','ca_sha256'),'TLS inputs')
    keys(d['identity_provider'],('definition','ca','ca_sha256'),'identity-provider inputs')
    for digest in list(d['toe']['archive_sha256'].values())+[d['toe']['manifest_sha256'],d['tls']['ca_sha256'],d['identity_provider']['ca_sha256']]:
        require(isinstance(digest,str) and re.fullmatch('[0-9a-fA-F]{64}',digest), 'A SHA-256 digest must have 64 hexadecimal characters')
    keys(d['operator'],('username','email','service_administrator'),'operator')
    for value in d['operator'].values():
        require(isinstance(value,str) and re.fullmatch(r'[A-Za-z0-9_@.+-]+',value), 'Use literal operator identifiers')
    require('@' in d['operator']['email'], 'An operator email address is required')
    keys(d['release'],('version','step_commit'),'release')
    require(re.fullmatch(r'\d+\.\d+\.\d+',d['release']['version']) is not None, 'Invalid release version')
    require(re.fullmatch('[0-9a-f]{40}',d['release']['step_commit']) is not None, 'Exact release source commit required')
    for value in [d['ssh_identity'],d['kit']['directory'],d['toe']['directory'],d['tls']['certificate'],d['tls']['private_key'],d['tls']['ca'],d['identity_provider']['definition'],d['identity_provider']['ca']]:
        require(isinstance(value,str) and Path(value).is_absolute() and '\n' not in value and '\x00' not in value, 'Use absolute input paths')
    if 'commissioning' in d:
        import commission
        commission.validate(d['commissioning'])
    return d

def checked_path(name, private=False, directory=False):
    p=Path(name)
    require(not p.is_symlink() and (p.is_dir() if directory else p.is_file()), 'Missing input or symbolic link refused')
    require(p.stat().st_uid == os.geteuid(), 'Input must be owned by the installation operator')
    if private: require(not stat.S_IMODE(p.stat().st_mode) & 0o077, 'Private input must have mode 600 (directory 700)')
    return p

def digest(path):
    result=hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''):result.update(block)
    return result.hexdigest()

def run(argv, label, data=None, timeout=120):
    try:
        r=subprocess.run(argv,input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=timeout,check=False)
    except (OSError,subprocess.TimeoutExpired): raise ValueError(label + ' failed: unavailable command or timeout; no automatic retry') from None
    if r.returncode:
        stage=re.findall(rb'^AGD_STAGE_FAILED ([0-9]{2}-[0-9]{3})$',r.stderr,re.M)
        suffix=(' (operation '+STEP_NAMES.get(stage[-1].decode(),stage[-1].decode())+')') if stage else ''
        checks={b'TIME_AUTHENTICATION': 'NTS-authenticated time is required; correct the accepted site time configuration', b'SSH_POLICY': 'SSH must allow administrator keys only', b'FRESH_CLUSTER': 'A fresh cluster and empty installation workspace are required', b'SOURCE_IDENTITY': 'Installed operating-system source differs from the accepted kit', b'TOOLS': 'An offline-kit prerequisite is unavailable', b'TLS_ENDPOINT': 'The accepted server kit must provide the nginx TLS endpoint before application installation', b'PRIVATE_INGRESS': 'Traefik must use loopback-only entrypoints with ClusterIP service; public plaintext HTTP is not allowed'}
        code=re.findall(rb'^AGD_CHECK_FAILED ([A-Z_]+)$',r.stderr,re.M)
        if code and code[-1] in checks: suffix=': '+checks[code[-1]]
        raise ValueError(label + ' failed'+suffix+'; stop and inspect this stage privately before any retry')
    return r.stdout

def verify_checksums(root, text):
    seen=set()
    for line in text.splitlines():
        m=re.fullmatch(r'([0-9a-fA-F]{64}) [ *](.+)',line)
        require(m is not None,'Invalid kit checksum entry')
        rel=Path(m[2]);require(not rel.is_absolute() and '..' not in rel.parts and str(rel) not in seen,'Unsafe or duplicate kit checksum path')
        p=root/rel;require(p.resolve().is_relative_to(root.resolve()) and p.is_file() and not p.is_symlink(),'Kit checksum file is missing or unsafe')
        require(digest(p)==m[1].lower(),'Kit file checksum mismatch')
        seen.add(str(rel))
    require(bool(seen),'Empty kit checksum list')

def verify_kit(d):
    root=checked_path(d['kit']['directory'],directory=True)
    public=checked_path(str(root/'release/signing-key.pub'))
    fingerprint=run(['ssh-keygen','-l','-E','sha256','-f',str(public)],'Kit signing-key check').decode().split()
    require(len(fingerprint)>1 and fingerprint[1] == d['kit']['signing_fingerprint'],'Kit signing fingerprint differs from the independently received value')
    checksums=checked_path(str(root/'checksums.txt'));signature=checked_path(str(root/'checksums.txt.sig'))
    with tempfile.TemporaryDirectory() as folder:
        allowed=Path(folder)/'allowed';allowed.write_text('release '+public.read_text().strip()+'\n')
        run(['ssh-keygen','-Y','verify','-f',str(allowed),'-I','release','-n','sequent-release','-s',str(signature)],'Kit signature check',checksums.read_bytes())
    verify_checksums(root,checksums.read_text())

def validate_idp(d, alias):
    require(isinstance(d,dict) and d.get('alias')==alias and d.get('providerId')=='oidc','Identity-provider definition or alias differs')
    for key,value in [('enabled',True),('trustEmail',False),('storeToken',False),('linkOnly',False),('firstBrokerLoginFlowAlias','certificate-first-login-flow')]:
        require(d.get(key)==value,'Identity-provider policy differs from AGD-04')
    c=d.get('config',{})
    for key,value in [('clientAuthMethod','client_secret_post'),('validateSignature','true'),('useJwksUrl','true'),('pkceEnabled','true'),('pkceMethod','S256')]:
        require(c.get(key)==value,'Identity-provider signature or PKCE policy differs from AGD-04')
    for key in ('issuer','authorizationUrl','tokenUrl','jwksUrl','logoutUrl'):
        require(isinstance(c.get(key),str) and c[key].startswith('https://') and not any(x in c[key] for x in ('\n','\r',' ')),'Identity-provider endpoints must be HTTPS URLs')
    require(isinstance(c.get('clientSecret'),str) and bool(c['clientSecret']) and isinstance(c.get('clientId'),str) and bool(c['clientId']),'Identity-provider registration is incomplete')

def verify_inputs(d):
    checked_path(d['ssh_identity'],private=True)
    verify_kit(d)
    toe=checked_path(d['toe']['directory'],directory=True)
    for part in PARTS:
        p=checked_path(str(toe/'images'/(part+'.tar')))
        require(digest(p)==d['toe']['archive_sha256'][part].lower(),'TOE archive differs from the accepted delivery digest')
    manifest=checked_path(str(toe/'image-digests.txt'))
    require(digest(manifest)==d['toe']['manifest_sha256'].lower(),'TOE image manifest differs from accepted delivery')
    image_digests=manifest.read_text()
    require('commit: '+d['release']['step_commit'] in image_digests.splitlines() and 'tag: v'+d['release']['version'] in image_digests.splitlines(),'TOE identity differs from accepted release configuration')
    found={}
    for line in image_digests.splitlines():
        if not line.strip() or line.startswith(('toe_reference:','tag:','commit:','production_run:','generated:')):continue
        m=re.fullmatch(r'(\S+) (sha256:[0-9a-f]{64})',line)
        require(m is not None and m[1] not in found,'Invalid or duplicate image digest entry');found[m[1]]=m[2]
    require(set(PARTS).issubset(found),'Image digest manifest is incomplete')
    for group in ('tls','identity_provider'):
        ca=checked_path(d[group]['ca'])
        der=run(['openssl','x509','-in',str(ca),'-outform','DER'],'CA certificate parsing')
        require(hashlib.sha256(der).hexdigest()==d[group]['ca_sha256'].lower(),'CA fingerprint differs from independently received value')
    cert=checked_path(d['tls']['certificate']);key=checked_path(d['tls']['private_key'],private=True)
    for host in [d['hosts'][name] for name in ('voting','administration','verifier')]:
        run(['openssl','verify','-purpose','sslserver','-verify_hostname',host,'-CAfile',d['tls']['ca'],str(cert)],'TLS certificate chain and hostname check')
    pub=run(['openssl','x509','-in',str(cert),'-noout','-pubkey'],'TLS public-key check')
    require(pub==run(['openssl','pkey','-in',str(key),'-pubout'],'TLS private-key match'),'TLS certificate and key do not match')
    detail=run(['openssl','x509','-in',str(cert),'-text','-noout'],'TLS curve check')
    require(b'NIST CURVE: P-384' in detail,'TLS certificate must use P-384')
    provider=checked_path(d['identity_provider']['definition'],private=True)
    validate_idp(json.loads(provider.read_text()),d['idp_alias'])

def environment(d):
    values={'PORTAL_HOST':d['hosts']['voting'],'ADMIN_HOST':d['hosts']['administration'],'VERIFIER_HOST':d['hosts']['verifier'],'AUTH_HOST':d['hosts']['authentication'],'IDP_ALIAS':d['idp_alias'],'C1':d['nodes']['c1'],'C2':d['nodes']['c2'],'R':R}
    return ''.join('export '+k+'='+shlex.quote(v)+'\n' for k,v in values.items())

def ssh_args(d,node):
    return ['ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=15','-i',d['ssh_identity'],'root@'+d['nodes'][node]]

def ssh(d,node,script,label,timeout=120):
    return run(ssh_args(d,node)+['bash','-s'],label,('set -Eeuo pipefail\nset +x\numask 077\n'+script).encode(),timeout)

def preflight_script(d,fresh,node="c1"):
    base='''AGD_CHECK=TOOLS
trap 'printf "AGD_CHECK_FAILED %s\\n" "$AGD_CHECK" >&2' ERR
for x in bash k3s openssl jq chronyc sshd sequent tar; do command -v "$x" >/dev/null; done
AGD_CHECK=SSH_POLICY
policy=$(sshd -T)
printf '%s\\n' "$policy" | grep -qi '^passwordauthentication no$'
printf '%s\\n' "$policy" | grep -qi '^pubkeyauthentication yes$'
printf '%s\\n' "$policy" | grep -qiE '^permitrootlogin (prohibit-password|without-password)$'
AGD_CHECK=TIME_AUTHENTICATION
auth=$(chronyc -N authdata)
printf '%s\\n' "$auth" | awk 'NR>2 {n++; if ($2 != "NTS") bad=1} END {exit !(n>0 && !bad)}'
chronyc tracking | grep -qE 'Leap status *: Normal'
chronyc sources | grep -qE '^.\\*'
'''
    base+='AGD_CHECK=SOURCE_IDENTITY\n'
    base+='test "$(jq -r .release /var/lib/sequent-install/provenance.json)" = '+shlex.quote(d.get('_kit_identity',d['release'])['version'])+'\n'
    base+='test "$(jq -r .images.manifest.stepRevision /var/lib/sequent-install/provenance.json)" = '+shlex.quote(d.get('_kit_identity',d['release'])['step_commit'])+'\n'
    if fresh:base+='AGD_CHECK=FRESH_CLUSTER\ntest ! -e /root/deploy\n'
    if node=='c2':return base
    base+='''AGD_CHECK=TLS_ENDPOINT
command -v nginx >/dev/null
systemctl cat nginx.service >/dev/null
AGD_CHECK=PRIVATE_INGRESS
k3s kubectl -n kube-system get deployment traefik -o json | jq -e '
 .spec.template.spec.hostNetwork == true and
 .spec.template.spec.nodeSelector["node-role.kubernetes.io/control-plane"] == "true" and
 ([.spec.template.spec.containers[].args[] | ascii_downcase | select(startswith("--entrypoints.") and contains(".address="))] | sort) ==
 (["--entrypoints.metrics.address=127.0.0.1:9100/tcp",
   "--entrypoints.traefik.address=127.0.0.1:9000/tcp",
   "--entrypoints.web.address=127.0.0.1:10080/tcp",
   "--entrypoints.websecure.address=127.0.0.1:10443/tcp"] | sort)' >/dev/null
k3s kubectl -n kube-system get service traefik -o json | jq -e '.spec.type == "ClusterIP"' >/dev/null
test -z "$(ss -H -ltn 'sport = :80')"
if systemctl is-active --quiet k3s; then
  k3s secrets-encrypt status | grep -q '^Encryption Status: Enabled$'
'''
    if fresh:base+='''  ns=$(k3s kubectl get namespace -o json)
  printf '%s' "$ns" | jq -e '[.items[] | select(.metadata.name == "step-apps" or .metadata.name == "step-infra")] | length == 0' >/dev/null
'''
    return base+'fi\n'

def check(d,fresh=True):
    print('CHECK: verifying accepted media, archive hashes and certificate inputs',flush=True)
    verify_inputs(d)
    provenance=json.loads((Path(d['kit']['directory'])/'release/provenance.json').read_text())
    d['_kit_identity']={'version':provenance['release'],'step_commit':provenance['images']['manifest']['stepRevision']}
    print('PASS: configuration, accepted media and certificate inputs',flush=True)
    for node in ('c1','c2'):
        ssh(d,node,preflight_script(d,fresh,node=node),'Node '+node+' preflight');print('PASS: '+node+' SSH policy, authenticated time and installed source',flush=True)
    raw=ssh(d,'c1','k3s kubectl get nodes -o json\n','Cluster readiness')
    nodes=json.loads(raw)['items'];require(len(nodes)==2,'Exactly two cluster nodes are required')
    addresses=set()
    for node in nodes:
        require(any(x['type']=='Ready' and x['status']=='True' for x in node['status']['conditions']),'A cluster node is not Ready')
        addresses.update(a['address'] for a in node['status']['addresses'] if a['type']=='InternalIP')
    require(addresses==set(d['nodes'].values()),'Cluster addresses differ from site configuration')
    print('PASS: both configured cluster nodes are Ready',flush=True)
    if fresh:print('CHECK PASSED — ready to install the platform',flush=True)


def command_source(ident,d):
    suffix='.jq' if ident in ('07-008','07-015','07-016') else '.sh'
    path=ROOT/'steps'/(ident+suffix)
    if not path.exists():path=ROOT/'commands'/(ident+suffix)
    s=path.read_text()
    if ident=='07-004':
        return "cd /root/deploy/kubernetes\n! grep -qE '^  APP_(VERSION|HASH):' config/step-env.yaml\nprintf '  APP_VERSION: \"%s\"\\n  APP_HASH: \"%s\"\\n' " + shlex.quote(d['release']['version']) + ' ' + shlex.quote(d['release']['step_commit']) + " >> config/step-env.yaml\n"
    values={'name-of-the-service-administrator':d['operator']['service_administrator'],'operator-user-name-at-the-authentication-server':d['operator']['username'],'e-mail-address':d['operator']['email'],'user-name-at-the-authentication-server':d['operator']['username'],'registered-trustee-name':d.get('_trustee',{}).get('name',''),'native-trustee-number':d.get('_trustee',{}).get('number','')}
    for key,value in values.items():s=s.replace('<'+key+'>',value)
    require(not re.search(r'<[a-z][a-z0-9 -]*>',s),'Unresolved installation input in reviewed command')
    return s


def install_script(d):
    # One shell preserves functions, variables and working directories across reviewed blocks.
    lines=['set -Eeuo pipefail', 'set +x', 'umask 077', '. /root/deploy/hosts.env', 'AGD_STAGE=00-000', 'trap \'printf "AGD_STAGE_FAILED %s\\n" "$AGD_STAGE" >&2\' ERR']
    def source(ident):
        lines.append('AGD_STAGE='+ident)
        lines.append('source /root/deploy/installation-commands/'+ident+'.sh')
    source('03-006')
    for ident in ('05-004','05-005','06-002','07-001','07-002','07-003','07-004','07-005','07-006','07-007'):
        source(ident)
    lines.extend(['install -m 600 /root/deploy/installation-commands/07-008.jq /root/deploy/keycloak.jq','install -m 600 /root/deploy/installation-commands/07-015.jq /root/deploy/board.jq','install -m 600 /root/deploy/installation-commands/07-016.jq /root/deploy/realm.jq'])
    for ident in ('07-009','07-010','07-011','07-012','07-013','07-014','07-017','07-020'):
        source(ident)
    lines.append(r'''verify_realms() {
  for kind in tenant tenant-template event-template; do
    jq -e --arg alias "$IDP_ALIAS" '
      .sslRequired == "external" and .ssoSessionIdleTimeout == 900 and
      .bruteForceProtected == true and .failureFactor == 5 and
      .eventsEnabled == true and .registrationAllowed == false and
      .resetPasswordAllowed == false and .rememberMe == false and
      (.eventsListeners | index("custom-event-listener") != null) and
      (.identityProviders | length == 1) and .identityProviders[0].alias == $alias and
      ([.clients[] | select(.clientId == "admin-portal" or .clientId == "voting-portal")] | length == 2) and
      ([.groups[]? | select(.name | startswith("admin")) | .realmRoles[]?
        | select(. == "publish-results-write" or . == "datafix-account" or . == "voter-manually-verify")] | length == 0) and
      ([.clients[] | select(.clientId == "admin-portal" or .clientId == "voting-portal")
        | .directAccessGrantsEnabled == false and .attributes["pkce.code.challenge.method"] == "S256"] | all) and
      .adminEventsEnabled == true and .adminEventsDetailsEnabled == true
    ' "/root/deploy/realm-$kind.new.json" >/dev/null
  done
}
verify_realms
''')
    source('07-021');source('08-003')
    lines.extend(['test "$POSTGRES_PASSWORD"','source /root/deploy/installation-commands/08-005.sh','printf platform-installed > /root/deploy/.platform-installed'])
    return '\n'.join(lines)+'\n'


def upload(d,node,files):
    buffer=io.BytesIO()
    with tarfile.open(fileobj=buffer,mode='w') as archive:
        for name,body in files.items():
            info=tarfile.TarInfo(name);info.size=len(body);info.mode=0o600;archive.addfile(info,io.BytesIO(body))
    run(ssh_args(d,node)+['bash','-c',shlex.quote('set -e; umask 077; test ! -e /root/deploy; mkdir -m 700 /root/deploy; tar -C /root/deploy -xf -')],node+' protected input transfer',buffer.getvalue())


def install(d):
    check(d,fresh=True)
    print('INSTALL 1/4: transfer verified images to both nodes',flush=True)
    env=environment(d).encode()
    for node in ('c1','c2'):
        files={'hosts.env':env,'.installation-config.sha256':hashlib.sha256(json.dumps(d,sort_keys=True).encode()).hexdigest().encode()}
        if node=='c1':
            files['tls/hosts.json']=(json.dumps([d['hosts'][name] for name in ('voting','administration','verifier')])+'\n').encode()
            for ident in ('03-006','05-004','05-005','06-002','07-001','07-002','07-003','07-004','07-005','07-006','07-007','07-008','07-009','07-010','07-011','07-012','07-013','07-014','07-015','07-016','07-017','07-020','07-021','08-003','08-005'):
                suffix='.jq' if ident in ('07-008','07-015','07-016') else '.sh';files['installation-commands/'+ident+suffix]=command_source(ident,d).encode()
            for key,path in [('tls/tls.crt',d['tls']['certificate']),('tls/tls.key',d['tls']['private_key']),('tls/ca.pem',d['tls']['ca']),('auth-server-ca.pem',d['identity_provider']['ca']),('idp.json',d['identity_provider']['definition'])]:files[key]=Path(path).read_bytes()
        upload(d,node,files)
        ssh(d,node,'test ! -e /root/toe-images\nmkdir -m 700 /root/toe-images\n',node+' image directory creation')
        for name in [p+'.tar' for p in PARTS]+['image-digests.txt']:
            run(['scp','-q','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-i',d['ssh_identity'],str(Path(d['toe']['directory'])/('images/'+name if name.endswith('.tar') else name)),'root@'+d['nodes'][node]+':/root/toe-images/'+name],node+' verified image transfer',timeout=900)
        sums=''.join(d['toe']['archive_sha256'][p]+'  '+p+'.tar\n' for p in PARTS)
        script='cd /root/toe-images\nsha256sum -c /dev/stdin <<\'ARCHIVE_HASHES\'\n'+sums+'ARCHIVE_HASHES\n'
        script+=command_source('08-002',d)
        script+='\nwhile read -r part hash; do case "$part" in b4|ballot-verifier|braid|harvest|keycloak|windmill) k3s ctr -n k8s.io images ls -q | grep -qx "sequentech.local/$part@$hash";; esac; done < /root/toe-images/image-digests.txt\n'
        ssh(d,node,script,node+' image import and readback',timeout=1800)
        print('PASS: '+node+' delivery archive hashes and imported image digests',flush=True)
    print('INSTALL 2/4: harden configuration and create private service credentials',flush=True)
    ssh(d,'c1',install_script(d),'Hardened platform deployment',timeout=2400)
    print('INSTALL 3/4: verify running platform configuration',flush=True)
    verify(d)
    cleanup_import(d)
    print('INSTALL 4/4: platform installed; customer and trustee commissioning still required',flush=True)


def verify(d):
    # Only configuration and operational readiness checks. Browser/MFA, client-network
    # filtering, custody and board authorization remain commissioning checkpoints.
    script=r'''
. /root/deploy/hosts.env
test -f /root/deploy/.platform-installed
k3s secrets-encrypt status | grep -q '^Encryption Status: Enabled$'
for namespace in step-apps step-infra; do
  names=$(k3s kubectl -n "$namespace" get deploy -o json | jq -r '.items[] | select(.metadata.name != "trustee1" and .metadata.name != "trustee2") | .metadata.name')
  test -n "$names"
  while IFS= read -r name; do k3s kubectl -n "$namespace" rollout status "deployment/$name" --timeout=10m; done <<< "$names"
  k3s kubectl -n "$namespace" get deploy -o json | jq -e '
    [.items[] | select(.metadata.name != "trustee1" and .metadata.name != "trustee2") |
      (.status.availableReplicas // 0) >= (.spec.replicas // 1)] | length>0 and all' >/dev/null
done
k3s kubectl -n step-apps get deploy hasura -o json | jq -e '
 [.spec.template.spec.containers[] | select(.name=="hasura") | .env[] |
  select(.name=="HASURA_GRAPHQL_ENABLE_CONSOLE" or .name=="HASURA_GRAPHQL_DEV_MODE") | .value] == ["false","false"]' >/dev/null
k3s kubectl -n step-apps get service b4 -o json | jq -e '.spec.type=="ClusterIP"' >/dev/null
k3s kubectl get ingress -A -o json | jq -e '[.items[].spec.rules[].http.paths[] | select(.backend.service.name=="b4")] | length==0' >/dev/null
for app in harvest windmill; do
 k3s kubectl -n step-apps exec "deploy/$app" -c "$app" -- sh -eu -c '
  test "${SECRETS_BACKEND:-}" = EnvVarMasterSecret
  test "${#MASTER_SECRET}" -eq 64'
done
deployments=$(k3s kubectl get deployments,jobs -A -o json)
pods=$(k3s kubectl get pods -A -o json)
for part in b4 ballot-verifier braid harvest keycloak windmill; do
 expected=$(sed -n "s/^$part //p" /root/toe-images/image-digests.txt)
 test -n "$expected"
 image="sequentech.local/$part@$expected"
 printf '%s' "$deployments" | jq -e --arg part "$part" --arg image "$image" '
   [.items[].spec.template.spec | (.containers[]?, .initContainers[]?) | .image |
    select(startswith("sequentech.local/"+$part+":") or startswith("sequentech.local/"+$part+"@"))] |
    length>0 and all(.==$image)' >/dev/null
 ids=$(printf '%s' "$pods" | jq -r --arg image "$image" '
   .items[].status | (.containerStatuses[]?, .initContainerStatuses[]?) |
   select(.image==$image and .imageID!="" and .imageID!=null) | .imageID' | sort -u)
 if test "$part" != braid; then test -n "$ids"; fi
 while IFS= read -r image_id; do
   if test -n "$image_id"; then
     k3s crictl inspecti "${image_id#containerd://}" | jq -e --arg image "$image" '.status.repoDigests | index($image)!=null' >/dev/null
   fi
 done <<< "$ids"
done
'''
    print('VERIFY: waiting for platform services and checking their security settings',flush=True)
    ssh(d,'c1',script,'Installed platform policy verification',timeout=1800)
    for host in [d['hosts'][name] for name in ('voting','administration','verifier')]:
        issuer=json.loads(run(['curl','--silent','--show-error','--fail','--cacert',d['tls']['ca'],'https://'+host+'/realms/'+R+'/.well-known/openid-configuration'],'External HTTPS discovery',timeout=30))['issuer']
        require(issuer=='https://'+d['hosts']['voting']+'/realms/'+R,'Published issuer differs from configured realm')
        status=run(['curl','--silent','--show-error','--cacert',d['tls']['ca'],'-o','/dev/null','-w','%{http_code}','https://'+host+'/realms/master/.well-known/openid-configuration'],'External master-realm restriction',timeout=30)
        require(status==b'403','Master realm is externally reachable or not verifiable')
    verify_login(d,R)
    print('PASS: installed platform readiness, restricted services, secret store and HTTPS discovery',flush=True)
    print('Customer sign-in, external MFA, client firewall and trustee custody remain commissioning checks.',flush=True)


def cleanup_import(d):
    body=command_source('08-006',d)
    lines=body.splitlines()
    body='\n'.join(line for line in lines if not line.startswith('curl -s '))+'\n'
    guard="test \"$(curl --silent --show-error --fail --cacert /root/deploy/tls/ca.pem -o /dev/null -w '%{http_code}' https://"+d['hosts']['voting']+'/realms/'+R+")\" = 200\n"
    assertion=r'''k3s kubectl -n step-infra get secret keycloak-realm -o json | jq -e --arg key "$R.json" '
 .data[$key] | @base64d | fromjson |
 ([.users[]? | select(.credentials != null)] | length==0) and
 ([.identityProviders[]?.config.clientSecret // empty] | length==0) and
 ([.clients[] | select(.clientId=="service-account" or .clientId=="api-key-client") | .secret] | sort)==["CHANGE_ME_API_KEY_CLIENT_SECRET","CHANGE_ME_SERVICE_ACCOUNT_SECRET"]' >/dev/null
 test ! -e /root/deploy/realm-tenant.new.json
'''
    ssh(d,'c1',guard+body+assertion,'Imported realm credential cleanup',timeout=120)
    print('PASS: imported realm credentials removed from its stored manifest and verified in the cluster',flush=True)


def verify_login(d,realm):
    host=d['hosts']['voting'];admin=d['hosts']['administration']
    for portal in (host,admin):
        discovery=json.loads(run(['curl','--silent','--show-error','--fail','--cacert',d['tls']['ca'],'https://'+portal+'/realms/'+realm+'/.well-known/openid-configuration'],'Realm discovery identity',timeout=30))
        require(discovery.get('issuer')=='https://'+host+'/realms/'+realm,'Realm discovery issuer differs from site configuration')
    endpoint='https://'+host+'/realms/'+realm+'/protocol/openid-connect'
    base=['curl','--silent','--show-error','--cacert',d['tls']['ca']]
    query={'client_id':'admin-portal','response_type':'code','scope':'openid','redirect_uri':'https://'+admin+'/'}
    pkce={**query,'code_challenge':'x'*43,'code_challenge_method':'S256'}
    redirect=run(base+['-o','/dev/null','-w','%{redirect_url}',endpoint+'/auth?'+urlencode(pkce)],'Identity-provider redirect check',timeout=30).decode()
    location=urlsplit(redirect)
    require(location.scheme=='https' and location.hostname==host and '/broker/'+d['idp_alias']+'/login' in location.path,'Authorization flow did not redirect to the configured external identity provider')
    rejected=run(base+['-o','/dev/null','-w','%{redirect_url}',endpoint+'/auth?'+urlencode(query)],'PKCE rejection check',timeout=30).decode()
    location=urlsplit(rejected)
    require(location.scheme=='https' and location.hostname==admin and parse_qs(location.query).get('error')==['invalid_request'],'A missing PKCE challenge was not rejected')
    other={**pkce,'redirect_uri':'https://other.invalid/'}
    require(run(base+['-o','/dev/null','-w','%{http_code}',endpoint+'/auth?'+urlencode(other)],'Unregistered redirect rejection',timeout=30)==b'400','An unregistered redirect URI was not rejected')
    for client,expected in [('admin-portal','unauthorized_client'),('admin-cli','unauthorized_client'),('cli-account-admin','invalid_client')]:
        response=json.loads(run(base+['-d','grant_type=password','-d','client_id='+client,'-d','username=agd-negative-check','-d','password=invalid-negative-check',endpoint+'/token'],'Password-grant rejection',timeout=30))
        require(response.get('error')==expected and 'access_token' not in response,'A forbidden password grant was not rejected')
    print('PASS: external login redirect, PKCE, exact redirect allowlist and forbidden password grants',flush=True)


def main(command,path):
    require(os.geteuid()==0,'Run from a root administration-workstation terminal')
    os.umask(0o077)
    config=checked_path(path);d=validate(json.loads(config.read_text()))
    if command=='check':check(d)
    elif command=='install':install(d)
    elif command=='verify':verify(d)
    elif command=='commission':
        import commission
        commission.main(d)
    else:raise ValueError('Unknown installation command')
