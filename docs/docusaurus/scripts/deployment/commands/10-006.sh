# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
for h in $PORTAL_HOST $ADMIN_HOST $VERIFIER_HOST; do
  curl -s -o /dev/null -w "$h %{http_code}\n" \
    "https://$h/realms/$R/election/x/public/login"
done
