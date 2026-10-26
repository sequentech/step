# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
E="<names of the election event realms, separated by spaces>"
for r in $R $CR $E; do
  curl -s "https://$PORTAL_HOST/realms/$r/protocol/openid-connect/certs" \
    | jq -r '.keys[].kid'
done > realm-kids.txt
curl -s "https://$PORTAL_HOST/storage/public/certs.json" | jq -r '.keys[].kid' \
  > certs-kids.txt
wc -l < certs-kids.txt
grep -vxFf realm-kids.txt certs-kids.txt || test "$?" = 1
