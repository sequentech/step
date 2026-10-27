# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -Eeuo pipefail
umask 077
C1=$(jq -er '.nodes.c1' site.json)
IDENTITY=$(jq -er '.ssh_identity' site.json)
c1() { ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -i "$IDENTITY" "root@$C1" "$@"; }
read -r -p "Type RESTORE AUTHORIZED BACKUP to confirm: " CONFIRM
test "$CONFIRM" = "RESTORE AUTHORIZED BACKUP"
read -r -p "Absolute path to the authorized backup: " B
test -f "$B" && test -r "$B" && test ! -L "$B"
case "$B" in /*) ;; *) echo 'Use an absolute backup path' >&2; exit 1;; esac
BACKUP_HASH=$(sha256sum "$B" | cut -d ' ' -f 1)
gpg --decrypt "$B" | python3 ./archive_check.py system
test "$(sha256sum "$B" | cut -d ' ' -f 1)" = "$BACKUP_HASH"
c1 'set -Eeuo pipefail
test -f /root/deploy/.backup-in-progress
count=$(k3s kubectl get pods -A -o json | jq "[.items[] | select(.metadata.namespace | startswith(\"step-\")) | select(.status.phase != \"Succeeded\" and .status.phase != \"Failed\")] | length")
test "$count" = 0
echo "RESTORE PRECHECK: services stopped"'
gpg --decrypt "$B" | c1 'set -Eeuo pipefail
test -f /root/deploy/.backup-in-progress
count=$(k3s kubectl get pods -A -o json | jq "[.items[] | select(.metadata.namespace | startswith(\"step-\")) | select(.status.phase != \"Succeeded\" and .status.phase != \"Failed\")] | length")
test "$count" = 0
find /var/lib/sequent-data -mindepth 1 -delete
find /root/deploy -mindepth 1 -delete
tar --numeric-owner -C / -xf -
echo "BACKUP EXTRACTED: services remain stopped"'
c1 'set -Eeuo pipefail
 test -f /root/deploy/.backup-in-progress
 jq -r ".[] | [.namespace,.name,(.replicas|tostring)] | @tsv" /root/deploy/backup-replicas.json |
 while read -r ns name replicas; do k3s kubectl -n "$ns" scale "deployment/$name" --replicas="$replicas"; done
 jq -r ".[] | [.namespace,.name] | @tsv" /root/deploy/backup-replicas.json |
 while read -r ns name; do k3s kubectl -n "$ns" rollout status "deployment/$name" --timeout=120s; done
 rm /root/deploy/.backup-in-progress
 echo "RECORDED REPLICA COUNTS RESTORED"'
./deploy inspect site.json platform
printf 'RESTORE CHECKS COMPLETE: board review is still required\n'
