# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && . /root/deploy/secrets.env
[ -n "$POSTGRES_PASSWORD" ] && sequent deploy --manifests /root/deploy/kubernetes
k3s kubectl -n step-infra rollout status deployment/keycloak --timeout=15m
k3s kubectl get pods -A
