# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/kubernetes
cp /root/toe-images/image-digests.txt /root/deploy/
d() { sed -n "s/^$1 \(sha256:[0-9a-f]*\)\$/\1/p" /root/deploy/image-digests.txt; }
for p in harvest windmill ballot-verifier; do
  grep -rl --include=*.yaml "sequentech.local/$p:latest" . | xargs sed -i \
    "s|sequentech.local/$p:latest|sequentech.local/$p@$(d $p)|"
done
sed -i "s|sequentech.local/b4v6:latest|sequentech.local/b4@$(d b4)|" b4/deployment.yaml
sed -i "s|sequentech.local/trustee:latest|sequentech.local/braid@$(d braid)|" \
  trustees/deployment.yaml
sed -i "s|\"sequentech.local/keycloak\"|\"sequentech.local/keycloak@$(d keycloak)\"|" \
  keycloak/deployment.json
grep -rhoE 'sequentech\.local/(b4|ballot|braid|harvest|keycloak|trustee|windmill)[^" ]*' \
  . | sed 's/@sha256:.*/@sha256:.../' | sort | uniq -c
