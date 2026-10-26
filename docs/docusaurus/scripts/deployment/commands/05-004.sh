# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl apply -f /root/deploy/kubernetes/namespaces.yaml
for ns in step-apps step-infra; do
  k3s kubectl -n $ns create secret tls step-tls-cert \
    --cert=/root/deploy/tls/tls.crt --key=/root/deploy/tls/tls.key
done
k3s kubectl get secret -A --field-selector metadata.name=step-tls-cert
