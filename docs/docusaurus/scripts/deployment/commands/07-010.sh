# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/kubernetes
sed -i -e '/name: HASURA_GRAPHQL_ENABLE_CONSOLE$/{n;s/"true"/"false"/}' \
  -e '/name: HASURA_GRAPHQL_DEV_MODE$/{n;s/"true"/"false"/}' hasura/deployment.yaml
sed -i '/- path: \/hasura$/{s|/hasura$|/hasura/v1/graphql|;n;s/Prefix/Exact/}' \
  admin-portal/ingress.yaml voting-portal/ingress.yaml ballot-verifier/ingress.yaml
grep -A1 -e 'HASURA_GRAPHQL_ENABLE_CONSOLE$' -e 'HASURA_GRAPHQL_DEV_MODE$' \
  hasura/deployment.yaml
grep -A1 'path: /hasura' */ingress.yaml
