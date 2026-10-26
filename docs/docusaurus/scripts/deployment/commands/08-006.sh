# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && cd /root/deploy
T=$R.json
curl -s -o /dev/null -w '%{http_code}\n' "https://$PORTAL_HOST/realms/$R"    # 200
jq '(.clients[] | select(.clientId == "service-account") | .secret)
    = "CHANGE_ME_SERVICE_ACCOUNT_SECRET"
  | (.clients[] | select(.clientId == "api-key-client") | .secret)
    = "CHANGE_ME_API_KEY_CLIENT_SECRET"
  | .users |= map(del(.credentials))
  | .identityProviders |= map(del(.config.clientSecret))' realm-tenant.new.json \
  > realm-tenant.clean.json
jq --arg k "$T" --rawfile r realm-tenant.clean.json '.stringData[$k] = $r' \
  kubernetes/keycloak/realm.json > realm.tmp && mv realm.tmp kubernetes/keycloak/realm.json
k3s kubectl apply -f kubernetes/keycloak/realm.json
shred -u realm-tenant.new.json
jq -r --arg k "$T" '.stringData[$k]' kubernetes/keycloak/realm.json | jq -c '{
  credentials: [.users[] | select(.credentials)] | length,
  idp_secret: [.identityProviders[].config.clientSecret // empty] | length,
  secrets: [.clients[] | select(.clientId == "service-account"
    or .clientId == "api-key-client") | .secret]}'
