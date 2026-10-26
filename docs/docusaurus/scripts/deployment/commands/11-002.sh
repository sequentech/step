# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
ssh root@$C1 'tar -C / -cf - var/lib/sequent-data root/deploy' \
  | gpg --symmetric --cipher-algo AES256 -o "$B"    # asks for a new passphrase twice
ssh root@$C1 'for ns in step-apps step-infra; do
  k3s kubectl -n $ns scale deployment --all --replicas=1; done'
gpg -d "$B" | tar -tf - | grep -o -e '^root/deploy/' -e '^var/lib/sequent-data/' \
  | sort | uniq -c                                   # asks for the passphrase
