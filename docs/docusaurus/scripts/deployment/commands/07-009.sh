# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/kubernetes/keycloak
k3s kubectl create --dry-run=client -o json -f deployment.yaml \
  | jq -f /root/deploy/keycloak.jq > deployment.json && rm deployment.yaml
jq -r '(.spec.template.spec.containers[] | select(.name == "keycloak")
  | (.args | join(" ")),
    (.env[] | select(.name | test("^KC_HOST|^KC_HTTP_EN|^KC_TRUST|^JAVA")) | .name)),
  (.spec.template.spec.volumes[] | select(.name == "realm-import")
    | .secret.secretName)' deployment.json
