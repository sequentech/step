#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Package reviewed AGD-04 commands from source; does not execute them."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import textwrap
from step_names import STEP_NAMES

HERE = Path(__file__).resolve().parent
GUIDE = Path('docs/docusaurus/docs/05-reference/06-deployment_guide')
# The numbered operations the helpers run themselves (install.py, commission.py,
# verification.py). The bundle carries these and no other command file; each
# needs its marker in the guide, which ties the operation to its section.
HELPER_STEPS = frozenset({
    '03-006', '05-004', '05-005', '06-002',
    '07-001', '07-002', '07-003', '07-004', '07-005', '07-006', '07-007', '07-008', '07-009', '07-010',
    '07-011', '07-012', '07-013', '07-014', '07-015', '07-016', '07-017', '07-020', '07-021',
    '08-002', '08-003', '08-005', '08-006', '08-008', '08-009', '08-011',
    '10-001', '10-004', '10-005', '10-007', '10-009', '10-011', '10-012', '10-014', '10-015',
})

def extract(text):
    rows=[]; active=None; heading=''; marker=None
    for n,line in enumerate(text.splitlines(keepends=True),1):
        if active:
            if re.fullmatch(r' {0,3}```\s*\n?',line):
                raw=''.join(active['lines']);active.update(body=textwrap.dedent(raw),raw=raw);active.pop('lines');rows.append(active);active=None
            else:active['lines'].append(line)
            continue
        match=re.match(r'^ {0,3}<!-- deployment-step: ([0-9]{2}-[0-9]{3}) -->',line)
        if match:marker=match[1]
        match=re.match(r'^#{2,3} (.*)',line)
        if match:heading=match[1].strip()
        match=re.match(r'^ {0,3}```(bash|jq|text)\s*\n?',line)
        if match:
            if not marker:raise ValueError('Command lacks its reviewed step marker')
            active={'id':marker,'language':match[1],'heading':heading,'line':n+1,'lines':[]};marker=None
    if active:raise ValueError('Unclosed command fence')
    return rows

def build(repo,out,commit,working=False):
    if out.exists():raise ValueError('Choose a fresh output directory')
    if not re.fullmatch('[0-9a-f]{40}',commit):raise ValueError('Exact source commit required')
    sha=lambda b:hashlib.sha256(b).hexdigest();files={};steps=[];sources=[]
    for p in sorted((repo/GUIDE).glob('*.md')):
        raw=p.read_bytes();rel=str(p.relative_to(repo))
        if not working and subprocess.check_output(['git','-C',str(repo),'show',commit+':'+rel])!=raw:raise ValueError('Guide differs from selected commit')
        sources.append({'path':rel,'sha256':sha(raw)})
        heading=''
        for line_number,line in enumerate(raw.decode().splitlines(),1):
            if re.match(r'^#{2,3} ',line):heading=re.sub(r'^#{2,3} ','',line)
            marker=re.search(r'<!-- deployment-step: ([0-9]{2}-[0-9]{3}) -->',line)
            if not marker:continue
            ident=marker[1]
            if ident not in HELPER_STEPS:continue
            candidates=list((HERE/'commands').glob(ident+'.*'))
            if len(candidates)!=1 or candidates[0].suffix not in ('.sh','.jq'):raise ValueError('Unique command source required: '+ident)
            command=candidates[0];body=command.read_bytes();command_rel=str(command.relative_to(repo))
            if not working and subprocess.check_output(['git','-C',str(repo),'show',commit+':'+command_rel])!=body:raise ValueError('Command differs from selected commit')
            name='steps/'+command.name;files[name]=body
            e={'id':STEP_NAMES[ident],'legacy_id':ident,'language':'bash' if command.suffix=='.sh' else 'jq','heading':heading,'source':rel,'line':line_number,'source_sha256':sha(raw),'file':name,'sha256':sha(body),'command_source':command_rel,'transformation':'none'}
            steps.append(e);sources.append({'path':command_rel,'sha256':sha(body)})
    if len({s['id'] for s in steps})!=len(steps):raise ValueError('Duplicate step identifier')
    if {s['legacy_id'] for s in steps}!=HELPER_STEPS:raise ValueError('Guide lacks the marker of a helper operation')
    for name in ('runner.py','build.py','step_names.py','install.py','commission.py','oci.py','inspection.py','site_checks.py','verification.py','archive_check.py','integrity.py','maintenance.py','trustee.py','trustee.example.json','site.example.json'):
        p=HERE/name;rel=str(p.relative_to(repo));raw=p.read_bytes()
        if not working and subprocess.check_output(['git','-C',str(repo),'show',commit+':'+rel])!=raw:raise ValueError('Runner differs from selected commit')
        files[name]=raw;sources.append({'path':rel,'sha256':sha(raw)})
    files['deploy']=b'#!/bin/sh\n# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>\n# SPDX-License-Identifier: AGPL-3.0-only\nexec python3 "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/runner.py" "$@"\n'
    files['trustee']=b'#!/bin/sh\n# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>\n# SPDX-License-Identifier: AGPL-3.0-only\nexec python3 "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/trustee.py" "$@"\n'
    files['README.md']=("# AGD-04 configuration-driven installation\n\nUse this complete verified folder on the administration workstation, after delivery acceptance and installation of the two site-specific server images.\n\n1. Copy `site.example.json` to `site.json` and fill its public site values and protected input-file paths. Use hashes and fingerprints from the independently accepted delivery records; do not copy an observed mismatch into the expected fields. JSON is the same configuration format as the offline kit and requires no additional parser package. Point `toe.directory` at the accepted OCI delivery folder; image blobs and descriptors are checked against its accepted digest manifest. The installer creates and removes its own private transport archives, so the operator does not create tar files or calculate ten additional archive hashes. Keep private keys and client secrets in their protected input files, never in this configuration.\n2. Run `./deploy check site.json`. It checks the delivery inputs, certificates, SSH/time policy and the two fresh cluster nodes. It performs no installation or disk erasure.\n3. Run `./deploy install site.json`. It imports the verified images, applies the reviewed hardened configuration and verifies the platform. It refuses an occupied cluster and does not automatically retry failed writes. The operator does not run the internal numbered operations.\n4. Create the customer tenant in the administration portal as the guide explains. Add the `commissioning` configuration described in AGD-04 and run `./deploy commission site.json`. Follow its pauses for identity-provider registration, trustee registration and physical key custody. It automates the remaining account and native-trustee commands; it does not authorize an election or board handover.\n\n`./deploy inspect site.json kit` checks the signed kit before boot. After installation, `./deploy inspect site.json network` lists actual ingress ports and checks SSH, authenticated time and cluster policy; `./deploy inspect site.json tls` checks allowed and refused TLS connections. After commissioning, `./deploy inspect site.json platform` groups the server acceptance checks. The network command writes a public `client-network.json` for `./deploy inspect client-network.json client-network` on a separate client-network computer; copy it with a fresh verified tool folder, never with the private working site configuration. After each import, use `./deploy inspect site.json election REALM` with the actual realm printed by the platform inspection. Read AGD-04 for the expected results and remaining human checks.\n\n`./deploy inspect site.json integrity` verifies all eleven accepted TOE image layouts and every installed server image manifest, configuration and layer on both nodes. It also checks the deployed image references. If verification fails, keep voting unavailable and restore the accepted delivery; never replace the expected digest with an observed mismatch. The on-demand election-verifier is checked in the same eleven-image accepted delivery before use.\n\n`./deploy backup site.json` stops services, streams an encrypted backup to the selected medium, verifies it and restores the recorded replica counts. `./deploy restore site.json` requires a current backup and an explicit recovery decision, validates the selected archive before replacement, refuses phase rollback and keeps voting in recovery. `./deploy services site.json status` shows the actual servers; `./deploy services site.json logs keycloak` and `./deploy services site.json restart keycloak` inspect or restart a named component. If an unchanged backup attempt failed, `./deploy services site.json resume-unchanged` is the explicit recovery command; it refuses any started restore. `./deploy renew-tls site.json` puts the renewed certificate and key that the configuration names into service, from the same CA file: the TLS endpoint reloads them without closing port 443 and the internal routes serve them too. `./deploy backup-trustees site.json`, `./deploy resume-trustees site.json` and `./deploy remove-trustees site.json` are separate custody operations. Offline trustees use `./trustee prepare trustee.json`, then the documented generate, public-keys or attest command; on the election board's instruction at the end of the election event, `./trustee destroy trustee.json` destroys a trustee's keys file and `./trustee destroy trustee.json PATH` each copy of it.\n\n`./deploy verify site.json` repeats the automated platform checks. Browser authentication/MFA, client-network filtering, offline trustee operations and board handover remain explicit human checks in the guide. The installer never claims these passed merely because a command exited successfully.\n\nRead AGD-04 before executing these commands. Python 3 and the listed administration-workstation tools are required; nodes use the tools of the accepted offline kit. No repository checkout, Internet download or developer build is performed during installation. SSH host keys must already be verified in the workstation's known_hosts.\n\n`manifest.json` binds this folder to its exact source revision; `SHA256SUMS` detects changed files. `steps/` holds the numbered operations that `./deploy` runs itself; the operator does not run them. Backup, restore and trustee removal are separate authorized maintenance operations, never automatic installation stages.\n").encode()
    m={'schema':1,'source_commit':commit,'test_only':working,'document':'AGD-04','document_version':'1.1','sources':sources,'steps':steps,'files':{n:sha(b) for n,b in files.items()}}
    files['manifest.json']=(json.dumps(m,indent=2)+'\n').encode();files['SHA256SUMS']=''.join(f'{sha(b)}  {n}\n' for n,b in sorted(files.items())).encode()
    for name,data in files.items():
        p=out/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data);p.chmod(0o755 if name in ('deploy','trustee') else 0o644)
    return m

def main():
    p=argparse.ArgumentParser();p.add_argument('--repo',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--commit',required=True);p.add_argument('--test-working-tree',action='store_true');a=p.parse_args();m=build(a.repo.resolve(),a.out.resolve(),a.commit,a.test_working_tree);print(json.dumps({'steps':len(m['steps']),'source_commit':m['source_commit'],'test_only':m['test_only']}))
if __name__=='__main__':main()
