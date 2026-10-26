# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s secrets-encrypt status | grep 'Encryption Status'
k3s kubectl -n step-apps get service b4 -o jsonpath='{.spec.type}{"\n"}'
k3s kubectl get ingress -A -o json \
  | jq -r '.items[].spec.rules[].http.paths[].backend.service.name' | grep -cx b4
k3s kubectl get service -A -o json | jq -r '.items[]
  | select(.spec.type != "ClusterIP") | "\(.metadata.namespace)/\(.metadata.name)"'
