# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
printf '      - "https://%s"\n' "$PORTAL_HOST" "$VERIFIER_HOST" "$ADMIN_HOST" \
  > /root/deploy/origins.txt
sed -i -e '/accessControlAllowOriginList:/r /root/deploy/origins.txt' \
  -e '/accessControlAllowOriginList:/{n;/- "\*"/d}' voting-portal/middleware.yaml
k3s kubectl set env --local --containers=hasura -f hasura/deployment.yaml \
  HASURA_GRAPHQL_CORS_DOMAIN="https://$PORTAL_HOST,https://$VERIFIER_HOST,https://$ADMIN_HOST" \
  -o yaml > /root/deploy/hasura-cors.yaml &&
mv /root/deploy/hasura-cors.yaml hasura/deployment.yaml
grep -A3 'accessControlAllowOriginList:' voting-portal/middleware.yaml
grep -A1 'name: HASURA_GRAPHQL_CORS_DOMAIN$' hasura/deployment.yaml
