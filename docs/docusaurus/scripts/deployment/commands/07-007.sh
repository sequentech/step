# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/kubernetes
printf '%s\n' '            - secretRef:' \
  '                name: keycloak-service-credentials' > /root/deploy/envfrom.txt
# After the application container's ConfigMap reference, not an init container's.
sed -i '/^      containers:$/,${/^                name: step-env$/r /root/deploy/envfrom.txt
}' harvest/deployment.yaml windmill/deployment.yaml windmill/beat.yaml
grep -c 'name: keycloak-service-credentials' harvest/deployment.yaml \
  windmill/deployment.yaml windmill/beat.yaml
