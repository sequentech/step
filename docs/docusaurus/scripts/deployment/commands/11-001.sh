# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -Eeuo pipefail
umask 077
C1=$(jq -er '.nodes.c1' site.json)
IDENTITY=$(jq -er '.ssh_identity' site.json)
c1() { ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -i "$IDENTITY" "root@$C1" "$@"; }
c1 'set -Eeuo pipefail; umask 077
 test ! -e /root/deploy/.backup-in-progress
 k3s kubectl -n step-apps get deploy -o json > /root/deploy/backup-apps.json
 k3s kubectl -n step-infra get deploy -o json > /root/deploy/backup-infra.json
 jq -s "[.[].items[] | {namespace:.metadata.namespace,name:.metadata.name,replicas:(.spec.replicas // 1)}]" \
   /root/deploy/backup-apps.json /root/deploy/backup-infra.json > /root/deploy/backup-replicas.json
 jq -e "length > 0 and all(.[]; .replicas >= 0)" /root/deploy/backup-replicas.json >/dev/null
 rm /root/deploy/backup-apps.json /root/deploy/backup-infra.json
 touch /root/deploy/.backup-in-progress
 for ns in step-apps step-infra; do k3s kubectl -n "$ns" scale deploy --all --replicas=0; done
 for attempt in $(seq 1 60); do
   count=$(k3s kubectl get pods -A -o json | jq "[.items[] | select(.metadata.namespace | startswith(\"step-\")) | select(.status.phase != \"Succeeded\" and .status.phase != \"Failed\")] | length")
   if test "$count" = 0; then echo "APPLICATIONS STOPPED: backup may begin"; exit 0; fi
   sleep 2
 done
 echo "Shutdown incomplete; do not create a backup" >&2; exit 1'
