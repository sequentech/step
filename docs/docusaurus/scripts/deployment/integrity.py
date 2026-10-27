#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Repeatable TSF image integrity verification against accepted release media."""
import re
import shlex
from pathlib import Path
import install


def node_script(expected):
    install.require(set(expected)==set(install.PARTS), 'Exactly six server component digests are required')
    install.require(all(isinstance(d,str) and re.fullmatch(r'sha256:[0-9a-f]{64}',d) for d in expected.values()), 'Invalid server component digest')
    manifest='\n'.join(part+' '+expected[part] for part in install.PARTS)+'\n'
    return r'''set -euo pipefail
D=$(mktemp -d /tmp/sequent-tsf-integrity.XXXXXX)
trap 'rm -rf "$D"' EXIT
blob() {
 local digest=$1 target
 [[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] || return 1
 target="$D/${digest#sha256:}"
 if [ ! -f "$target" ]; then
   k3s ctr -n k8s.io content get "$digest" > "$target"
   [ "$(sha256sum "$target" | cut -d' ' -f1)" = "${digest#sha256:}" ] || return 1
 fi
 printf '%s\n' "$target"
}
graph() {
 local digest=$1 depth=$2 body child
 [ "$depth" -le 8 ] || return 1
 body=$(blob "$digest") || return
 if jq -e '.config and (.layers | type=="array")' "$body" >/dev/null; then
   child=$(jq -er '.config.digest' "$body") || return
   blob "$child" >/dev/null || return
   while IFS= read -r child; do blob "$child" >/dev/null || return; done < <(jq -er '.layers[].digest' "$body")
 else
   jq -e '.manifests | type=="array" and length==1' "$body" >/dev/null || return
   child=$(jq -er '.manifests[0].digest' "$body") || return
   graph "$child" "$((depth+1))" || return
 fi
}
while read -r component digest; do
 graph "$digest" 0
 printf '%s manifest, configuration and every layer verified\n' "$component"
done <<'ACCEPTED_COMPONENT_DIGESTS'
'''+manifest+'ACCEPTED_COMPONENT_DIGESTS\n'


def check(data):
    install.require(data.get('toe',{}).get('format','tar')=='oci',
                    'TSF integrity inspection requires the complete seven-image accepted OCI delivery')
    # This rechecks the independently accepted manifest, every OCI descriptor and
    # every content blob of all seven parts, including the on-demand verifier.
    install.verify_inputs(data)
    manifest=(Path(data['toe']['directory'])/'image-digests.txt').read_text()
    expected={m[1]:m[2] for line in manifest.splitlines()
              if (m:=re.fullmatch(r'(\S+) (sha256:[0-9a-f]{64})',line))}
    install.require(set(expected)==set(install.TOE_PARTS),'All seven accepted TOE parts are required')
    print('PASS: all seven delivered TSF images, including election-verifier, match the accepted release',flush=True)
    servers={part:expected[part] for part in install.PARTS}
    script=node_script(servers)
    for node in ('c1','c2'):
        raw=install.ssh(data,node,script,'Full TSF content verification on '+node,timeout=180).decode()
        lines=raw.strip().splitlines()
        install.require(lines==[part+' manifest, configuration and every layer verified' for part in install.PARTS],
                        'Incomplete TSF content verification on '+node)
        print('PASS: '+node+' — all six installed server images match every accepted content blob',flush=True)
    refs='\n'.join('sequentech.local/'+part+'@'+servers[part] for part in install.PARTS)+'\n'
    # Trusted expected values come from verified delivery, not the current cluster.
    runtime="expected="+shlex.quote(refs)+r'''
k3s kubectl get deployments -A -o json | jq -e --arg expected "$expected" '
 ($expected | split("\n") | map(select(length>0))) as $allowed |
 [.items[].spec.template.spec | (.containers[]?,.initContainers[]?) | .image |
 select(test("^sequentech\\.local/(b4|ballot-verifier|braid|harvest|keycloak|windmill)(@|:)"))] |
 length>0 and all(. as $image | $allowed | index($image)!=null)' >/dev/null
'''
    install.ssh(data,'c1',runtime,'Running TSF deployment identity verification',timeout=10)
    print('PASS: installed TSF deployment references match the accepted images; no software was replaced',flush=True)
