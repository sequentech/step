# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/secrets.env
k3s kubectl -n step-infra create secret generic keycloak-bootstrap-admin \
  --from-env-file=<(printf '%s\n' "KC_BOOTSTRAP_ADMIN_USERNAME=$KC_ADMIN_USER" \
    "KC_BOOTSTRAP_ADMIN_PASSWORD=$KC_ADMIN_PASSWORD")
k3s kubectl -n step-apps create secret generic keycloak-service-credentials \
  --from-env-file=<(printf '%s\n' KEYCLOAK_CLIENT_ID=service-account \
    "KEYCLOAK_CLIENT_SECRET=$KC_SERVICE_ACCOUNT_SECRET" \
    "KEYCLOAK_ADMIN_CLIENT_ID=$KC_ADMIN_USER" \
    "KEYCLOAK_ADMIN_CLIENT_SECRET=$KC_ADMIN_PASSWORD")
for s in step-infra/keycloak-bootstrap-admin step-apps/keycloak-service-credentials; do
  k3s kubectl -n ${s%/*} get secret ${s#*/} -o jsonpath='{.data}' | jq -r 'keys[]'
done
