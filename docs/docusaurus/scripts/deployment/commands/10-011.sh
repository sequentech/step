# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -d keycloak \
  -tAc "select name from realm where name like '%-event-%'"
