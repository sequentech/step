# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
for h in $PORTAL_HOST $ADMIN_HOST $VERIFIER_HOST; do
  curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"type":"export_metadata","args":{}}' "https://$h/hasura/v1/metadata" \
    | awk '/"code"/ {n++} END {print n+0}'
  curl -s -o /dev/null -D - -H 'Origin: https://other.invalid' \
    "https://$h/hasura/v1/graphql" | awk 'tolower($0) ~ /^access-control-allow-origin/ {n++} END {print n+0}'
done
