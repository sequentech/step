# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
B=<backup-medium>/<file-of-the-backup-to-restore>.tar.gpg
test -f "$B" && test -r "$B"
# Validate decryption and the complete tar stream before deleting anything.
# Plaintext flows through a pipe only; GPG obtains the passphrase privately.
gpg -d "$B" | tar -tf - | awk '
  /^root\/deploy\// {deployment=1}
  /^var\/lib\/sequent-data\// {data=1}
  /^\// || /(^|\/)\.\.(\/|$)/ {unsafe=1}
  END {exit !(deployment && data && !unsafe)}'
ssh root@$C1 'set -e -o pipefail; find /var/lib/sequent-data -mindepth 1 -delete'
gpg -d "$B" | ssh root@$C1 'set -e -o pipefail; tar -C / -xf -'      # asks for the passphrase
