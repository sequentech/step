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
HOSTS = {
    '03-001':['admin'], '03-002':['admin'], '03-003':['admin'], '03-004':['admin'],
    '04-002':['client'], '04-007':['admin'], '04-003':['c1','c2'], '04-004':['c1','c2'],
    '05-003':['admin'], '05-006':['admin'], '06-004':['c2'],
    '08-001':['admin'], '08-002':['c1','c2'], '08-010':['admin'],
    '08-013':['admin'], '08-015':['trustee'], '08-016':['trustee'],
    '10-002':['admin'], '10-003':['admin'], '10-006':['admin'], '10-008':['admin'],
    '11-001':['admin'], '11-002':['admin'], '11-003':['admin'],
}
DATA = {'07-008':'/root/deploy/keycloak.jq','07-015':'/root/deploy/board.jq','07-016':'/root/deploy/realm.jq'}

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
            ident=marker[1];candidates=list((HERE/'commands').glob(ident+'.*'))
            if len(candidates)!=1 or candidates[0].suffix not in ('.sh','.jq'):raise ValueError('Unique command source required: '+ident)
            command=candidates[0];body=command.read_bytes();command_rel=str(command.relative_to(repo))
            if not working and subprocess.check_output(['git','-C',str(repo),'show',commit+':'+command_rel])!=body:raise ValueError('Command differs from selected commit')
            name='steps/'+command.name;files[name]=body
            e={'id':STEP_NAMES[ident],'legacy_id':ident,'language':'bash' if command.suffix=='.sh' else 'jq','heading':heading,'source':rel,'line':line_number,'source_sha256':sha(raw),'roles':HOSTS.get(ident,['c1']),'destination':DATA.get(ident),'destructive':ident in ('08-014','11-003'),'file':name,'sha256':sha(body),'command_source':command_rel,'transformation':'none'}
            steps.append(e);sources.append({'path':command_rel,'sha256':sha(body)})
    if len({s['id'] for s in steps})!=len(steps):raise ValueError('Duplicate step identifier')
    for name in ('runner.py','session.sh','build.py','step_names.py','install.py','site.example.json'):
        p=HERE/name;rel=str(p.relative_to(repo));raw=p.read_bytes()
        if not working and subprocess.check_output(['git','-C',str(repo),'show',commit+':'+rel])!=raw:raise ValueError('Runner differs from selected commit')
        files[name]=raw;sources.append({'path':rel,'sha256':sha(raw)})
    files['deploy']=b'#!/bin/sh\n# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>\n# SPDX-License-Identifier: AGPL-3.0-only\nexec python3 "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/runner.py" "$@"\n'
    files['README.md']=('''# AGD-04 deployment commands

Use this folder from the verified delivery ZIP. It is self-contained: Python 3,
Bash and the handbook's operational tools are needed; no repository, package
installation or network download is performed by this launcher.

1. Finish Verifying the Delivery for this command package first. The separate
   installation kit is then checked in AGD-04 section 3 using this launcher. The
   hash check below detects changed files; it does not authenticate a delivery.
2. Copy this whole folder onto each named machine. On each machine, as root,
   run `./deploy check`, then `./deploy configure ROLE` and
   `./deploy session ROLE`. Roles: admin, c1, c2, client, trustee. The client
   network machine is distinct from the administration workstation. Trustees
   remain offline. Configuration is public, validated data in operator.json;
   never enter a password, token or private key there. The guide's hidden
   prompts and root-only secret files retain secret input handling.
3. Follow AGD-04 in order. `steps` lists readable command names (legacy numeric
   IDs remain supported aliases), `show 07-configure-storage-route` displays the exact
   reviewed command, and `step 07-configure-storage-route` runs only that block on its allowed host.
   Read its preconditions and compare all outputs with the guide before moving
   on. jq steps install the exact bundled filter; text steps are expected output.
4. Keep the same shell open for dependent commands: variables, functions and
   working directories persist. If disconnected, inspect the current state and
   re-establish the variables the guide requires; never rerun a write blindly.
   The runner neither retries nor bulk-runs steps and records no secret output.
   An exit status is not an acceptance decision; negative checks may deliberately
   return nonzero. No installation, handover or approval is inferred by the runner.
5. Public placeholders are prompted when needed; supplied values are retained
   per step. Node sessions check the configured IP against local addresses.
   Each workstation's configured machine identity is checked. This does not
   replace the guide's network-position, physical custody or role checks.

`manifest.json` binds the source commit, source pages and every bundled byte.
`SHA256SUMS` supports an independent `sha256sum --check SHA256SUMS`.
Read the corresponding PDF before each command; all security and acceptance
checks, receipt confirmation, account procedures and browser operations remain
in the handbook. Never pipe `step` output to a transcript or run with shell tracing.

These commands are not an unattended installer. In particular backup, restore
and trustee-key removal require their stated lifecycle and board authorization.
The confirmation prompt does not itself confer that authorization.
''').encode()
    m={'schema':1,'source_commit':commit,'test_only':working,'document':'AGD-04','document_version':'1.0','sources':sources,'steps':steps,'files':{n:sha(b) for n,b in files.items()}}
    files['manifest.json']=(json.dumps(m,indent=2)+'\n').encode();files['SHA256SUMS']=''.join(f'{sha(b)}  {n}\n' for n,b in sorted(files.items())).encode()
    for name,data in files.items():
        p=out/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data);p.chmod(0o755 if name=='deploy' else 0o644)
    return m

def main():
    p=argparse.ArgumentParser();p.add_argument('--repo',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--commit',required=True);p.add_argument('--test-working-tree',action='store_true');a=p.parse_args();m=build(a.repo.resolve(),a.out.resolve(),a.commit,a.test_working_tree);print(json.dumps({'steps':len(m['steps']),'source_commit':m['source_commit'],'test_only':m['test_only']}))
if __name__=='__main__':main()
