# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -Eeuo pipefail
umask 077
C1=$(jq -er '.nodes.c1' site.json)
IDENTITY=$(jq -er '.ssh_identity' site.json)
c1() { ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -i "$IDENTITY" "root@$C1" "$@"; }
read -r -p "Mounted encrypted-backup directory: " BACKUP_DIRECTORY
test -d "$BACKUP_DIRECTORY" && test ! -L "$BACKUP_DIRECTORY"
B="$BACKUP_DIRECTORY/trustee-state-$(date -u +%Y%m%dT%H%M%SZ).tar.gpg"
test ! -e "$B"
c1 'set -Eeuo pipefail; umask 077
 test ! -e /root/deploy/.trustee-backup-in-progress
 k3s kubectl -n step-infra get deploy trustee1 trustee2 -o json |
   jq "[.items[] | {name:.metadata.name,replicas:(.spec.replicas // 1)}]" > /root/deploy/trustee-backup-replicas.json
 touch /root/deploy/.trustee-backup-in-progress
 k3s kubectl -n step-infra scale deploy trustee1 trustee2 --replicas=0
 for attempt in $(seq 1 60); do
   count=$(k3s kubectl -n step-infra get pods -l "app in (trustee1,trustee2)" -o json | jq ".items | length")
   if test "$count" = 0; then echo "TRUSTEES STOPPED: state backup may begin"; exit 0; fi
   sleep 2
 done
 echo "Trustee shutdown incomplete; no backup taken" >&2; exit 1'
c1 'set -Eeuo pipefail
 test -f /root/deploy/.trustee-backup-in-progress
 count=$(k3s kubectl -n step-infra get pods -l "app in (trustee1,trustee2)" -o json | jq ".items | length")
 test "$count" = 0
 tar -C /var/lib/sequent-data -cf - trustee1 trustee2' |
  gpg --symmetric --cipher-algo AES256 --output "$B"
gpg --decrypt "$B" | python3 ./archive_check.py trustee
c1 'set -Eeuo pipefail
 jq -r ".[] | [.name,(.replicas|tostring)] | @tsv" /root/deploy/trustee-backup-replicas.json |
 while read -r name replicas; do k3s kubectl -n step-infra scale "deployment/$name" --replicas="$replicas"; done
 for name in trustee1 trustee2; do k3s kubectl -n step-infra rollout status "deployment/$name" --timeout=120s; done
 rm /root/deploy/.trustee-backup-in-progress'
printf 'TRUSTEE BACKUP COMPLETE: %s\n' "$B"
