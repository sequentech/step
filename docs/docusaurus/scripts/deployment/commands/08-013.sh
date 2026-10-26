# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
B=<backup-medium>/trustee-state-$(date +%F).tar.gpg
ssh root@$C1 'k3s kubectl -n step-infra scale --replicas=0 \
  deployment/trustee1 deployment/trustee2'
ssh root@$C1 'tar -C /var/lib/sequent-data -cf - trustee1 trustee2' \
  | gpg --symmetric --cipher-algo AES256 -o "$B"
ssh root@$C1 'k3s kubectl -n step-infra scale --replicas=1 \
  deployment/trustee1 deployment/trustee2'
gpg -d "$B" | tar -tf - | cut -d/ -f1 | sort -u    # trustee1 and trustee2
