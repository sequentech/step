# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
openssl x509 -in /root/deploy/auth-server-ca.pem -noout -subject -fingerprint -sha256
k3s kubectl -n step-infra create configmap auth-server-ca \
  --from-file=ca.pem=/root/deploy/auth-server-ca.pem
