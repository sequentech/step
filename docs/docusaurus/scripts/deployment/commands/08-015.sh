# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
M=/mnt/temenos-delivery
D=$(sed -n 's/^braid \(sha256:[0-9a-f]*\)$/\1/p' "$M/image-digests.txt")
[ "$(skopeo inspect --format '{{.Digest}}' "oci:$M/images/braid:v10.0.0")" = "$D" ]
echo "braid image verified"
skopeo copy "oci:$M/images/braid:v10.0.0" containers-storage:localhost/braid:v10.0.0
[ "$(podman image inspect --format '{{.Digest}}' localhost/braid:v10.0.0)" = "$D" ]
echo "braid image loaded"
umount "$M" && cryptsetup close temenos-delivery
echo "D=$D" > ~/braid-digest
