# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/toe-images
for p in b4 ballot-verifier braid harvest keycloak windmill; do
  k3s ctr -n k8s.io images import --local --base-name sequentech.local/$p --digests $p.tar
done
grep -E '^(b4|ballot-verifier|braid|harvest|keycloak|windmill) sha256:' \
  image-digests.txt | while read -r p d; do
  k3s ctr -n k8s.io images ls -q | grep -qx "sequentech.local/$p@$d" \
    && echo "$p loaded" || echo "$p MISSING"
done
