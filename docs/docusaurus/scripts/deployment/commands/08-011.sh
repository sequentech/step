# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && . /root/deploy/secrets.env
N=<native-trustee-number>
P=TRUSTEE${N}_PASSWORD
k3s kubectl -n step-infra create secret generic trustee$N-auth \
  --from-env-file=<(printf '%s\n' "TRUSTEE_PASSWORD=${!P}" \
    "TRUSTEE_KEYCLOAK_REALM=$CR" TRUSTEE_KEYCLOAK_CLIENT_ID=api-key-client \
    "TRUSTEE_KEYCLOAK_CLIENT_SECRET=$KC_API_KEY_CLIENT_SECRET")
k3s kubectl -n step-infra rollout restart deployment/trustee$N
k3s kubectl -n step-infra rollout status deployment/trustee$N --timeout=5m
