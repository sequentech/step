# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -Eeuo pipefail
umask 077
C1=$(jq -er '.nodes.c1' site.json)
IDENTITY=$(jq -er '.ssh_identity' site.json)
c1() { ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -i "$IDENTITY" "root@$C1" "$@"; }
read -r -p "Type REMOVE TRUSTEE KEYS to confirm: " CONFIRM
test "$CONFIRM" = "REMOVE TRUSTEE KEYS"
c1 'set -Eeuo pipefail
 k3s kubectl -n step-infra scale deployment/trustee1 deployment/trustee2 --replicas=0
 for attempt in $(seq 1 60); do
   count=$(k3s kubectl -n step-infra get pods -l "app in (trustee1,trustee2)" -o json | jq ".items | length")
   if test "$count" = 0; then break; fi
   sleep 2
 done
 test "$count" = 0
 k3s kubectl -n step-infra delete secret trustee1-keys trustee2-keys trustee1-auth trustee2-auth
 find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 -delete
 remaining=$(k3s kubectl -n step-infra get secret trustee1-keys trustee2-keys trustee1-auth trustee2-auth --ignore-not-found -o name)
 test -z "$remaining"
 count=$(find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 | wc -l)
 test "$count" = 0
 echo "TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty"'
