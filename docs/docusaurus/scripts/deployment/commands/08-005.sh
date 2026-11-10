# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && . /root/deploy/secrets.env
[ -n "$POSTGRES_PASSWORD" ] && sequent deploy --manifests /root/deploy/kubernetes
k3s kubectl -n step-infra rollout status deployment/keycloak --timeout=15m
# Harvest, windmill and beat use this client's service account for the
# administration API. It may create realms and administer the installation
# tenant realm; Keycloak grants it administration of the realms it creates.
kc() { k3s kubectl -n step-infra exec -i deploy/keycloak -- \
  /opt/keycloak/bin/kcadm.sh "$@" --config /tmp/kcadm-admin-api.config; }
# kcadm takes the password from KC_CLI_PASSWORD, read from standard input, so
# that it is on no command line.
printf '%s\n' "$KC_ADMIN_PASSWORD" | k3s kubectl -n step-infra exec -i deploy/keycloak -- \
  bash -c 'IFS= read -r KC_CLI_PASSWORD && export KC_CLI_PASSWORD &&
    exec /opt/keycloak/bin/kcadm.sh config credentials --server http://localhost:8090 \
      --realm master --user "$1" --config /tmp/kcadm-admin-api.config' kcadm "$KC_ADMIN_USER"
jq -n '{clientId: "step-admin-api", enabled: true, protocol: "openid-connect",
    publicClient: false, clientAuthenticatorType: "client-secret",
    secret: $ENV.KC_ADMIN_API_CLIENT_SECRET, serviceAccountsEnabled: true,
    standardFlowEnabled: false, implicitFlowEnabled: false,
    directAccessGrantsEnabled: false, fullScopeAllowed: true,
    attributes: {"oauth2.device.authorization.grant.enabled": "false",
      "oidc.ciba.grant.enabled": "false"}}' \
  | kc create clients -r master -f -
kc add-roles -r master --uusername service-account-step-admin-api \
  --rolename create-realm < /dev/null
kc add-roles -r master --uusername service-account-step-admin-api \
  --cclientid "$R-realm" --rolename view-realm --rolename manage-realm \
  --rolename view-users --rolename manage-users --rolename query-users \
  --rolename query-groups --rolename view-clients < /dev/null
for C in account account-console; do
  I=$(kc get clients -r master -q clientId=$C --fields id \
    --format csv --noquotes < /dev/null)
  kc update clients/$I -r master -s enabled=false < /dev/null
done
k3s kubectl -n step-infra exec deploy/keycloak -- rm /tmp/kcadm-admin-api.config
# Beat runs its start-up realm migrations again, now with the client.
k3s kubectl -n step-apps rollout restart deployment/beat
k3s kubectl -n step-apps rollout status deployment/beat --timeout=5m
k3s kubectl get pods -A
