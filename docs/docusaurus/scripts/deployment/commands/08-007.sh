# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && cd /root/deploy
k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -tA \
  -d hasura -c "select id, slug from sequent_backend.tenant order by slug"
echo 'export CR=tenant-<id-of-the-customer-tenant>' >> /root/deploy/hosts.env
