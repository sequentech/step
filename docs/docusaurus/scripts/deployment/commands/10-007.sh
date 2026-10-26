# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl get ingress -A -o json | jq -r '.items[].spec.rules[].http.paths[]
  | select(.backend.service.name == "hasura") | "\(.path) \(.pathType)"' | sort -u
k3s kubectl -n step-apps get deployment hasura -o json \
  | jq -r '.spec.template.spec.containers[].env[]
    | select(.name | test("CONSOLE|DEV_MODE")) | "\(.name) \(.value)"'
