#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Explicit, fail-closed maintenance for an existing accepted installation."""
import getpass
import hashlib
import json
import os
import re
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import time
import uuid
import archive_check
import install

STATE='/var/lib/sequent-maintenance'
PHASES={'preparation':0,'execution':1,'evaluation':2,'post_processing':3}
SERVICES={'keycloak':'step-infra','postgres':'step-infra','rustfs':'step-infra',
          'trustee1':'step-infra','trustee2':'step-infra','harvest':'step-apps',
          'windmill':'step-apps','beat':'step-apps','b4':'step-apps','hasura':'step-apps',
          'admin-portal':'step-apps','voting-portal':'step-apps','ballot-verifier':'step-apps'}


def load(configuration):
    p=Path(configuration)
    install.require(p.is_file() and p.stat().st_size<=1024*1024,'Provide the accepted site configuration')
    data=json.loads(p.read_text());install.validate(data)
    install.checked_path(data['ssh_identity'],private=True,label='SSH identity')
    return data


def identity(data):
    return {'schema':1,'nodes':data['nodes'],'hosts':data['hosts'],
            'release':data['release'],'toe_manifest_sha256':data['toe']['manifest_sha256']}


def confirm(phrase):
    install.require(input('Type '+phrase+' to confirm: ')==phrase,'Maintenance cancelled')


def backup_destination(prefix='sequent'):
    p=Path(input('Mounted encrypted-backup directory: ')).expanduser()
    install.require(p.is_absolute() and p.is_dir() and not p.is_symlink(),'Use an existing absolute backup directory, not a symlink')
    result=p/(prefix+'-'+time.strftime('%Y%m%dT%H%M%SZ',time.gmtime())+'-'+uuid.uuid4().hex[:8]+'.tar.gpg')
    install.require(not result.exists(),'Backup path already exists')
    return result


def password(new=False):
    value=getpass.getpass('New backup passphrase: ' if new else 'Backup passphrase: ')
    install.require(16<=len(value)<=1024 and '\n' not in value and '\r' not in value,'Use a passphrase of 16–1024 characters without line breaks')
    if new:install.require(value==getpass.getpass('Repeat backup passphrase: '),'Passphrases differ; nothing was stopped')
    return value


def ssh_command(data,script):
    return install.ssh_args(data,'c1')+['bash','-c',shlex.quote('set -Eeuo pipefail\nset +x\numask 077\n'+script)]


def gpg_process(arguments,secret,**kwargs):
    read,write=os.pipe()
    try:
        os.write(write,secret.encode()+b'\n');os.close(write);write=None
        return subprocess.Popen(['gpg','--no-options','--batch','--quiet','--no-symkey-cache',
            '--pinentry-mode','loopback','--passphrase-fd',str(read)]+arguments,pass_fds=(read,),**kwargs)
    finally:
        os.close(read)
        if write is not None:os.close(write)


def verify_archive(path,kind,passphrase=None):
    secret=passphrase if passphrase is not None else password()
    with open(path,'rb') as encrypted, tempfile.TemporaryFile() as errors:
        process=gpg_process(['--decrypt'],secret,stdin=encrypted,stdout=subprocess.PIPE,stderr=errors)
        try:
            metadata=archive_check.validate(process.stdout,kind)
            process.stdout.close()
            install.require(process.wait(timeout=30)==0,'Archive authentication failed; services remain stopped')
        except BaseException:
            process.kill();process.wait();raise
        finally:
            process.stdout.close()
    print('BACKUP CONTENTS VERIFIED: '+kind,flush=True)
    return metadata


def encrypt_remote(data,remote_script,output_path,kind='system',passphrase=None):
    secret=passphrase if passphrase is not None else password(new=True)
    output=Path(output_path)
    flags=os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW
    with os.fdopen(os.open(output,flags,0o600),'wb') as target, tempfile.TemporaryFile() as remote_errors, tempfile.TemporaryFile() as cipher_errors:
        remote=subprocess.Popen(ssh_command(data,remote_script),stdout=subprocess.PIPE,stderr=remote_errors)
        try:cipher=gpg_process(['--symmetric','--cipher-algo','AES256','--force-mdc'],secret,stdin=remote.stdout,stdout=target,stderr=cipher_errors)
        except BaseException:
            remote.kill();remote.wait();raise
        finally:remote.stdout.close()
        try:
            encrypted=cipher.wait();received=remote.wait(timeout=30)
            install.require(encrypted==0 and received==0,'Backup transfer or encryption failed; retain the partial file for diagnosis and keep services stopped')
            target.flush();os.fsync(target.fileno())
        except BaseException:
            cipher.kill();remote.kill();cipher.wait();remote.wait();raise
    verify_archive(output,kind,secret)
    return install.digest(output)


def floors(rows):
    install.require(isinstance(rows,list),'Lifecycle safety record is missing')
    result={}
    for row in rows:
        install.require(isinstance(row,dict) and set(row)=={'tenant_id','election_event_id','phase','safety','generation'},'Invalid lifecycle safety record')
        for field in ('tenant_id','election_event_id'):uuid.UUID(row[field])
        install.require(row['phase'] in PHASES and row['safety'] in ('operational','recoverable','unrecoverable') and type(row['generation']) is int and row['generation']>=0,'Invalid lifecycle safety state')
        key=(row['tenant_id'],row['election_event_id']);install.require(key not in result,'Repeated lifecycle identity');result[key]=row
    return result


def restore_compatible(metadata,current,expected):
    install.require(isinstance(metadata,dict) and metadata.get('installation')==expected,'Backup belongs to a different installation, release or image manifest')
    previous=floors(metadata.get('lifecycle'));present=floors(current)
    install.require(set(previous)==set(present),'Restore would remove or resurrect an election; keep the installation stopped for a separately reviewed recovery')
    for key,row in present.items():
        before=previous[key]
        install.require(before['phase']==row['phase'],'Restore would change an electoral phase; ended voting must never reopen')
        install.require(row['safety']!='unrecoverable' or before['safety']=='unrecoverable','Restore would undo an unrecoverable security state')
        install.require(row['phase']!='execution','Execution recovery requires a separately reviewed procedure preserving all accepted ballots and audit records; generic archive rollback is refused')
    return previous


PODS=r'''count=$(k3s kubectl get pods -A -o json | jq '[.items[] | select(.metadata.namespace | startswith("step-")) | select(.status.phase != "Succeeded" and .status.phase != "Failed")] | length')
test "$count" = 0'''
LIFECYCLE_SQL="SELECT coalesce(json_agg(json_build_object('tenant_id',tenant_id,'election_event_id',election_event_id,'phase',phase,'safety',safety,'generation',generation) ORDER BY election_event_id),'[]'::json) FROM sequent_backend.election_lifecycle"


def stop(data):
    print('Stopping applications, recording safety state, then stopping the database…',flush=True)
    script=r'''
+test ! -e /root/deploy/.trustee-backup-in-progress
+test ! -e /root/deploy/.backup-in-progress
+install -d -m 0700 /var/lib/sequent-maintenance
+test ! -e /var/lib/sequent-maintenance/state.json
+mkdir /var/lib/sequent-maintenance/claim
+printf system > /var/lib/sequent-maintenance/claim/owner
+trap 'if test ! -e /var/lib/sequent-maintenance/state.json; then rm -f /var/lib/sequent-maintenance/claim/owner; rmdir /var/lib/sequent-maintenance/claim; fi' EXIT
+k3s kubectl get deployments -n step-apps -o json > /var/lib/sequent-maintenance/apps.json
+k3s kubectl get deployments -n step-infra -o json > /var/lib/sequent-maintenance/infra.json
+jq -s '[.[].items[] | {namespace:.metadata.namespace,name:.metadata.name,replicas:(.spec.replicas // 1)}]' /var/lib/sequent-maintenance/apps.json /var/lib/sequent-maintenance/infra.json > /var/lib/sequent-maintenance/replicas.json
+jq -e 'length > 0 and all(.[]; (.namespace=="step-apps" or .namespace=="step-infra") and (.name|test("^[a-z0-9-]+$")) and (.replicas|type=="number") and .replicas>=0) and any(.[]; .namespace=="step-infra" and .name=="postgres" and .replicas==1)' /var/lib/sequent-maintenance/replicas.json >/dev/null
+printf '{"phase":"stopping"}\n' > /var/lib/sequent-maintenance/state.json
+touch /root/deploy/.backup-in-progress
+jq -r '.[] | select(.name!="postgres" or .namespace!="step-infra") | [.namespace,.name] | @tsv' /var/lib/sequent-maintenance/replicas.json |
+while read -r ns name; do k3s kubectl -n "$ns" scale "deployment/$name" --replicas=0; done
+for attempt in $(seq 1 30); do
+ count=$(k3s kubectl get pods -A -o json | jq '[.items[] | select(.metadata.namespace | startswith("step-")) | select(.status.phase!="Succeeded" and .status.phase!="Failed") | select(.metadata.namespace!="step-infra" or (.metadata.labels.app // "")!="postgres")] | length')
+ if test "$count" = 0; then break; fi
+ sleep 1
+done
+test "$count" = 0
+'''.replace('\n+','\n')
    script+='k3s kubectl exec -n step-infra deployment/postgres -- psql -U postgres -d hasura -At -v ON_ERROR_STOP=1 -c '+shlex.quote(LIFECYCLE_SQL)+' > '+STATE+'/lifecycle.json\n'
    script+='jq -e \'type=="array"\' '+STATE+'/lifecycle.json >/dev/null\n'
    script+='k3s kubectl -n step-infra scale deployment/postgres --replicas=0\n'
    script+='for attempt in $(seq 1 30); do\n'+PODS+' && break\nsleep 1\ndone\n'+PODS+'\n'
    script+='printf \'{"phase":"stopped"}\\n\' > '+STATE+'/state.json\necho "APPLICATIONS STOPPED: backup may begin"\n'
    output=install.ssh(data,'c1',script,'Application shutdown for maintenance',timeout=80)
    print(output.decode(),end='',flush=True)


def safety_record(data):
    result=install.ssh(data,'c1','test "$(jq -r .phase '+STATE+'/state.json)" = stopped\ncat '+STATE+'/lifecycle.json\n','Stopped lifecycle record',timeout=10)
    rows=json.loads(result);floors(rows);return rows


def resume(data,restored=False):
    phase='restored' if restored else 'stopped|stopping|backup-failed|backed-up'
    script=(f'test "$(cat {STATE}/claim/owner)" = system\n'
            'test ! -e /root/deploy/.trustee-backup-in-progress\n'
            f'jq -er .phase {STATE}/state.json | grep -Ex {shlex.quote(phase)} >/dev/null\n')
    script+=r'''jq -e 'length>0 and all(.[]; (.namespace=="step-apps" or .namespace=="step-infra") and (.name|test("^[a-z0-9-]+$")) and (.replicas|type=="number") and .replicas>=0)' /var/lib/sequent-maintenance/replicas.json >/dev/null
+jq -r '.[] | [.namespace,.name,(.replicas|tostring)] | @tsv' /var/lib/sequent-maintenance/replicas.json |
+while read -r ns name replicas; do k3s kubectl -n "$ns" scale "deployment/$name" --replicas="$replicas"; done
+'''.replace('\n+','\n')
    # Each rollout is bounded separately and the caller sees progress.
    install.ssh(data,'c1',script,'Restore recorded replica counts',timeout=30)
    replicas=json.loads(install.ssh(data,'c1','cat '+STATE+'/replicas.json','Recorded deployments',timeout=10))
    for item in replicas:
        if item['replicas']==0:continue
        install.require(item['namespace'] in ('step-apps','step-infra') and re.fullmatch(r'[a-z0-9-]+',item['name']) is not None,'Unexpected saved deployment; keep maintenance state and investigate')
        install.ssh(data,'c1','k3s kubectl -n '+shlex.quote(item['namespace'])+' rollout status deployment/'+shlex.quote(item['name'])+' --timeout=45s','Wait for '+item['name'],timeout=50)
        print('READY: '+item['name'],flush=True)
    install.ssh(data,'c1','rm /root/deploy/.backup-in-progress\nmv '+STATE+'/state.json '+STATE+'/last-completed.json\nrm '+STATE+'/claim/owner\nrmdir '+STATE+'/claim\n','Close completed maintenance',timeout=10)
    print('RECORDED REPLICA COUNTS RESTORED; voting recovery still requires the board',flush=True)


def set_phase(data,expected,following):
    script='test "$(cat '+STATE+'/claim/owner)" = system\ntest "$(jq -r .phase '+STATE+'/state.json)" = '+shlex.quote(expected)+'\n'
    script+='printf %s '+shlex.quote(json.dumps({'phase':following}))+' > '+STATE+'/state.json\n'
    install.ssh(data,'c1',script,'Record '+following+' maintenance state',timeout=10)


def backup(data):
    destination=backup_destination();secret=password(new=True)
    confirm('BACKUP WITH SERVICE OUTAGE');stop(data)
    metadata={'installation':identity(data),'created_at':int(time.time()),'lifecycle':safety_record(data)}
    encoded=json.dumps(metadata,separators=(',',':'))
    install.ssh(data,'c1','printf %s '+shlex.quote(encoded)+' > /root/deploy/.backup-identity.json\n','Record backup identity',timeout=10)
    set_phase(data,'stopped','backing-up')
    try:
        digest=encrypt_remote(data,'test "$(jq -r .phase '+STATE+'/state.json)" = backing-up\ntest -f /root/deploy/.backup-in-progress\n'+PODS+'\ntar -C / -cf - var/lib/sequent-data root/deploy\n',destination,passphrase=secret)
    except BaseException:
        try:set_phase(data,'backing-up','backup-failed')
        except BaseException:print('Unable to record failed transfer; retain the maintenance claim and keep services stopped',flush=True)
        raise
    set_phase(data,'backing-up','backed-up')
    receipt={'archive':str(destination),'sha256':digest,'installation':identity(data),'created_at':int(time.time())}
    receipt_path=Path('.backup-receipt.json');install.require(not receipt_path.is_symlink(),'Backup receipt path is a symlink')
    receipt_path.write_text(json.dumps(receipt)+'\n');receipt_path.chmod(0o600)
    resume(data)
    print('SYSTEM BACKUP COMPLETE: '+str(destination)+'\nSHA256: '+digest,flush=True)


def invalidate_restored_state(data,current):
    rows=floors(current)
    sql="BEGIN; SET LOCAL sequent.trusted_write='on'; DELETE FROM sequent_backend.lifecycle_service_lease;\n"
    for row in rows.values():
        event=str(uuid.UUID(row['election_event_id']));tenant=str(uuid.UUID(row['tenant_id']));phase=row['phase'];generation=row['generation']
        sql+=f"UPDATE sequent_backend.election_lifecycle SET safety=CASE WHEN safety='unrecoverable' THEN safety ELSE 'recoverable' END,generation=greatest(generation,{generation})+1,reason='Authorized backup restored; current self-tests and board recovery required' WHERE tenant_id='{tenant}' AND election_event_id='{event}' AND phase='{phase}';\n"
        sql+=f"INSERT INTO sequent_backend.lifecycle_notice(tenant_id,election_event_id,kind,details) VALUES('{tenant}','{event}','SystemBackupRestored','{{\"board_review_required\":true}}');\n"
    sql+='COMMIT;\n'
    script="k3s kubectl exec -i -n step-infra deployment/postgres -- psql -U postgres -d hasura -v ON_ERROR_STOP=1 <<'AGD_SQL'\n"+sql+'AGD_SQL\n'
    install.ssh(data,'c1',script,'Invalidate restored voting admission and record recovery',timeout=20)


def restore(data):
    selected=Path(input('Absolute path to the authorized older backup: '))
    install.checked_path(str(selected),label='Authorized backup')
    install.require(selected.is_absolute(),'Use an absolute backup path')
    receipt=json.loads(Path('.backup-receipt.json').read_text())
    install.require(receipt.get('installation')==identity(data) and time.time()-receipt.get('created_at',0)<86400,'Create a verified current-state backup with this configuration before restoring')
    current_backup=Path(receipt['archive']);install.require(current_backup!=selected and install.digest(current_backup)==receipt['sha256'],'Retain a separate verified current-state backup')
    secret=password();confirm('RESTORE AUTHORIZED BACKUP')
    # Copy encrypted bytes only into a private directory: the second decryption
    # cannot race an exchanged or edited removable-media file after validation.
    with tempfile.TemporaryDirectory(prefix='.sequent-restore-',dir='.') as temporary:
        snapshot=Path(temporary)/'authorized.tar.gpg';shutil.copyfile(selected,snapshot);snapshot.chmod(0o600)
        metadata=verify_archive(snapshot,'system',secret)
        install.require(isinstance(metadata,dict) and metadata.get('installation')==identity(data),'Backup identity differs from this accepted installation')
        stop(data);current=safety_record(data);restore_compatible(metadata,current,identity(data))
        install.ssh(data,'c1',PODS+'\nprintf \'{"phase":"restoring"}\\n\' > '+STATE+'/state.json\n','Enter durable restore state',timeout=10)
        script='test "$(cat '+STATE+'/claim/owner)" = system\ntest "$(jq -r .phase '+STATE+'/state.json)" = restoring\n'+PODS+'\nfind /var/lib/sequent-data -mindepth 1 -delete\nfind /root/deploy -mindepth 1 -delete\ntar --numeric-owner -C / -xf -\n'
        with snapshot.open('rb') as source, tempfile.TemporaryFile() as errors:
            decrypt=gpg_process(['--decrypt'],secret,stdin=source,stdout=subprocess.PIPE,stderr=errors)
            try:remote=subprocess.Popen(ssh_command(data,script),stdin=decrypt.stdout,stdout=subprocess.DEVNULL,stderr=errors)
            except BaseException:
                decrypt.kill();decrypt.wait();raise
            finally:decrypt.stdout.close()
            try:install.require(remote.wait()==0 and decrypt.wait(timeout=30)==0,'Restore failed; services remain stopped. Do not use resume-unchanged')
            except BaseException:remote.kill();decrypt.kill();remote.wait();decrypt.wait();raise
        install.ssh(data,'c1','k3s kubectl -n step-infra scale deployment/postgres --replicas=1\nk3s kubectl -n step-infra rollout status deployment/postgres --timeout=45s\n','Start isolated restored database',timeout=50)
        restored=json.loads(install.ssh(data,'c1','k3s kubectl exec -n step-infra deployment/postgres -- psql -U postgres -d hasura -At -v ON_ERROR_STOP=1 -c '+shlex.quote(LIFECYCLE_SQL),'Read actual restored safety state',timeout=10))
        restore_compatible({'installation':identity(data),'lifecycle':restored},current,identity(data))
        invalidate_restored_state(data,current)
        install.ssh(data,'c1','printf \'{"phase":"restored"}\\n\' > '+STATE+'/state.json\n','Record validated restoration',timeout=10)
        resume(data,restored=True)
    print('RESTORE CHECKS COMPLETE: run platform inspection; all six self-tests and board review are required before voting',flush=True)


def main(command,configuration,operation=None,component=None):
    if command=='services':
        install.require(operation in ('status','stop','resume-unchanged','restart','logs'),'Choose status, stop, resume-unchanged, restart or logs')
        install.require((component is None) if operation in ('status','stop','resume-unchanged') else component in SERVICES,'Only restart and logs accept a required component name; stop always means all services')
    data=load(configuration)
    if command in ('backup','restore'):
        install.require(shutil.which('gpg') is not None,'Install the accepted GPG tool before maintenance')
        install.verify_inputs(data)
        return backup(data) if command=='backup' else restore(data)
    if operation=='status':
        script='hostname\nk3s kubectl get nodes\nk3s kubectl get pods -A\ndf -h /var/lib/sequent-data\n'
        script+='if test -f /run/sequent-security/activity.json; then\n'
        script+="jq -r 'if .status==\"available\" and ((now*1000)-.updated_at_ms>=0) and ((now*1000)-.updated_at_ms<=30000) then \"REQUEST MONITORING: available\" else \"REQUEST MONITORING: unavailable — \"+(.reason//\"observation is stale or missing\") end' /run/sequent-security/activity.json\n"
        script+='else echo \"REQUEST MONITORING: unavailable — observation file is missing\"; fi\n'
        result=install.ssh(data,'c1',script,'Current server state',timeout=15)
        print(result.decode(),end='');return
    if operation=='stop':confirm('STOP SERVICES FOR MAINTENANCE');return stop(data)
    if operation=='resume-unchanged':confirm('RESUME UNCHANGED SERVICES');return resume(data)
    install.require(operation in ('restart','logs') and component in SERVICES,'Choose status, stop, resume-unchanged, or restart/logs with a documented component')
    ns=SERVICES[component]
    if operation=='logs':
        print(install.ssh(data,'c1','k3s kubectl -n '+ns+' logs deployment/'+component+' --tail=20','Recent component log',timeout=15).decode(),end='');return
    confirm('RESTART '+component)
    script='install -d -m 0700 '+STATE+'\nmkdir '+STATE+'/claim\nprintf restart > '+STATE+'/claim/owner\n'
    script+="trap 'rm -f "+STATE+"/claim/owner; rmdir "+STATE+"/claim' EXIT\n"
    script+='test ! -e '+STATE+'/state.json\ntest ! -e /root/deploy/.trustee-backup-in-progress\nk3s kubectl -n '+ns+' rollout restart deployment/'+component+'\nk3s kubectl -n '+ns+' rollout status deployment/'+component+' --timeout=45s'
    install.ssh(data,'c1',script,'Restart '+component,timeout=50)
    print('COMPONENT READY: '+component+'; perform platform and browser checks; voting remains a board decision')
