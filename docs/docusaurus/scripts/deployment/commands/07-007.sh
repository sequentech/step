# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/kubernetes
printf '%s\n' '            - secretRef:' \
  '                name: keycloak-service-credentials' > /root/deploy/envfrom.txt
sed -i '/^                name: step-secrets$/r /root/deploy/envfrom.txt' \
  harvest/deployment.yaml windmill/deployment.yaml windmill/beat.yaml
grep -c 'name: keycloak-service-credentials' harvest/deployment.yaml \
  windmill/deployment.yaml windmill/beat.yaml
