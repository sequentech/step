# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
B=<backup-medium>/sequent-$(date -u +%Y%m%dT%H%MZ).tar.gpg
ssh root@$C1 'set -e -o pipefail; for ns in step-apps step-infra; do
  k3s kubectl -n $ns scale deployment --all --replicas=0; done'
ssh root@$C1 'set -e -o pipefail; pods=$(k3s kubectl get pods -A --no-headers \
  --field-selector=status.phase=Running)
printf "%s\n" "$pods" | awk '\''/^step-/ {n++} END {print n+0}'\'''
