# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
sed -i '/name: KEYCLOAK_URL$/{n;s|value: .*|value: "https://'"$PORTAL_HOST"'"|}' \
  admin-portal/deployment.yaml ballot-verifier/deployment.yaml
grep -A1 'name: KEYCLOAK_URL$' */deployment.yaml
