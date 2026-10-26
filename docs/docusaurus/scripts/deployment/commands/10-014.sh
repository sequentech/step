# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && cd /root/deploy
q() { k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -tA -d "$@"; }
q keycloak -c "select count(*) from keycloak_role r
  where r.name in ('datafix-account', 'voter-manually-verify')
  and (exists (select 1 from user_role_mapping m where m.role_id = r.id)
    or exists (select 1 from group_role_mapping m where m.role_id = r.id)
    or exists (select 1 from composite_role c where c.child_role = r.id))"
q keycloak -c "select count(*) from client
  where client_id in ('ivr-service', 'ivr-voting') and enabled"
{ grep -rl 'KEYCLOAK_IVR_' kubernetes || test "$?" = 1; } | wc -l
q hasura -c "select id from sequent_backend.tenant order by id"
q hasura -c "select count(*) from sequent_backend.election_event
  where tenant_id = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5'"
q keycloak -c "select count(*) from user_entity u join realm r on r.id = u.realm_id
  where r.name = '$R' and u.enabled and u.service_account_client_link is null"
q keycloak -c "select r.name, r.admin_events_enabled, r.admin_events_details_enabled
  from realm r where r.name in ('$R', '$CR') order by r.name"
