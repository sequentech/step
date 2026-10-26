# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy
jq --arg k "$T" --rawfile r realm-tenant.new.json \
  '.kind = "Secret" | .type = "Opaque" | del(.data) | .stringData = {($k): $r}' \
  realm.cm.json > kubernetes/keycloak/realm.json
jq --arg t "$T" --arg e "$E" --rawfile tt realm-tenant-template.new.json \
  --rawfile et realm-event-template.new.json \
  '.items |= map(.kind = "Secret" | .type = "Opaque" | del(.data)
    | .stringData = if .metadata.name == "keycloak-tenant-realm-template"
        then {($t): $tt} else {($e): $et} end)' \
  templates.cm.json > kubernetes/keycloak/realm-templates.json
sed -i 's/^              - configMap:$/              - secret:/' \
  kubernetes/rustfs/buckets.yaml
rm kubernetes/keycloak/realm.yaml kubernetes/keycloak/realm-templates.yaml
chmod -R go-rwx /root/deploy
ls kubernetes/keycloak
jq -r '.kind' kubernetes/keycloak/realm.json
jq -r '.items[] | "\(.kind) \(.metadata.name)"' kubernetes/keycloak/realm-templates.json
grep -A1 -e '- secret:' -e '- configMap:' kubernetes/rustfs/buckets.yaml
grep -rl 'kind: ConfigMap' kubernetes | while IFS= read -r _agd_config; do
  grep -il -e secret -e password "$_agd_config" || test "$?" = 1
done
