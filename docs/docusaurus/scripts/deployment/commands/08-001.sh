# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
M=/mnt/temenos-delivery
for n in "$C1" "$C2"; do
  ssh root@$n 'install -d -m 700 /root/toe-images'
  scp "$M/image-digests.txt" root@$n:/root/toe-images/
  for p in b4 ballot-verifier braid harvest keycloak windmill; do
    tar -C "$M/images/$p" -cf - . | ssh root@$n "cat > /root/toe-images/$p.tar"
  done
done
umount "$M" && cryptsetup close temenos-delivery
