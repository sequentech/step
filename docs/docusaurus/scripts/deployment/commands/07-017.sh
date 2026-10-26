# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && . /root/deploy/secrets.env
cd /root/deploy/kubernetes/keycloak
k3s kubectl create --dry-run=client -o json -f realm.yaml > ../../realm.cm.json
k3s kubectl create --dry-run=client -o json -f realm-templates.yaml \
  | jq -s '{apiVersion: "v1", kind: "List", items: .}' > ../../templates.cm.json
cd /root/deploy
T=$R.json
E=$R-event-33f18502-a67c-4853-8333-a58630663559.json
pick() { jq -r --arg n "$1" --arg k "$2" \
  '.items[] | select(.metadata.name == $n) | .data[$k]' templates.cm.json; }
jq -r --arg k "$T" '.data[$k]' realm.cm.json > realm-tenant.json
pick keycloak-tenant-realm-template "$T" > realm-tenant-template.json
pick keycloak-election-event-realm-template "$E" > realm-event-template.json
jq '.authenticationFlows[] | select(.alias == "certificate-first-login-flow")' \
  realm-event-template.json > flow.json
for d in tenant tenant-template event-template; do
  jq -e -L /root/deploy --arg doc $d --slurpfile idp idp.json \
    --slurpfile flow flow.json -f realm.jq realm-$d.json > realm-$d.new.json && echo "$d done"
done
