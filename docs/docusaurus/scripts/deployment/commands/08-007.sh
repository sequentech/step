# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && cd /root/deploy
k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -tA \
  -d hasura -c "select id, slug from sequent_backend.tenant order by slug"
read -r -p 'Customer tenant UUID from the list above: ' _agd_tenant_id
if [[ ! $_agd_tenant_id =~ ^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$ ]]; then
  printf 'Invalid tenant UUID; no customer realm was saved.\n' >&2
  return 1
fi
export CR="tenant-$_agd_tenant_id"
printf 'export CR=%s\n' "$CR" >> /root/deploy/hosts.env
printf 'Customer realm selected: %s\n' "$CR"
