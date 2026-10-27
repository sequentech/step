# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -Eeuo pipefail
umask 077
C1=$(jq -er '.nodes.c1' site.json)
IDENTITY=$(jq -er '.ssh_identity' site.json)
c1() { ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -i "$IDENTITY" "root@$C1" "$@"; }
read -r -p "Mounted encrypted-backup directory: " BACKUP_DIRECTORY
test -d "$BACKUP_DIRECTORY" && test ! -L "$BACKUP_DIRECTORY"
B="$BACKUP_DIRECTORY/sequent-$(date -u +%Y%m%dT%H%M%SZ).tar.gpg"
test ! -e "$B"
c1 'set -Eeuo pipefail
test -f /root/deploy/.backup-in-progress
count=$(k3s kubectl get pods -A -o json | jq "[.items[] | select(.metadata.namespace | startswith(\"step-\")) | select(.status.phase != \"Succeeded\" and .status.phase != \"Failed\")] | length")
test "$count" = 0
tar -C / -cf - var/lib/sequent-data root/deploy' |
  gpg --symmetric --cipher-algo AES256 --output "$B"
gpg --decrypt "$B" | python3 ./archive_check.py system
c1 'set -Eeuo pipefail
 test -f /root/deploy/.backup-in-progress
 jq -r ".[] | [.namespace,.name,(.replicas|tostring)] | @tsv" /root/deploy/backup-replicas.json |
 while read -r ns name replicas; do k3s kubectl -n "$ns" scale "deployment/$name" --replicas="$replicas"; done
 jq -r ".[] | [.namespace,.name] | @tsv" /root/deploy/backup-replicas.json |
 while read -r ns name; do k3s kubectl -n "$ns" rollout status "deployment/$name" --timeout=120s; done
 rm /root/deploy/.backup-in-progress
 echo "RECORDED REPLICA COUNTS RESTORED"'
printf 'SYSTEM BACKUP COMPLETE: %s\n' "$B"
