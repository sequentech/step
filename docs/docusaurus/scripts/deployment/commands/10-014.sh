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
q keycloak -c "with profile as materialized (
    select p.realm_id, f.value from component p
    join component_config f on f.component_id = p.id
    where p.provider_id = 'declarative-user-profile'
    and f.name = 'kc.user.profile.config')
  select (select count(*) from client
      where client_id in ('account', 'account-console') and enabled)
  + (select count(*) from required_action_provider a join realm r on r.id = a.realm_id
      where r.name like 'tenant-%' and (a.enabled or a.default_action))
  + (select count(*) from realm r join composite_role c on c.composite = r.default_role
      join keycloak_role k on k.id = c.child_role
      where r.name like 'tenant-%' and k.client_role and k.name = 'manage-account')
  + (select count(*) from realm r where r.name like 'tenant-%-event-%'
      and not exists (select 1 from profile f where f.realm_id = r.id
        and (select count(*) from jsonb_array_elements(f.value::jsonb -> 'attributes') e
          where e ->> 'name' in ('username', 'email')
          and jsonb_typeof(e -> 'permissions' -> 'edit') = 'array'
          and not (e -> 'permissions' -> 'edit') ? 'user') = 2))
  + (select count(*) from user_entity u join realm r on r.id = u.realm_id
      join user_role_mapping m on m.user_id = u.id
      join keycloak_role k on k.id = m.role_id
      where r.name = 'master' and u.service_account_client_link is not null
      and not k.client_role and k.name = 'admin')
  + (select case count(*) when 1 then 0 else 1 end from client c
      join realm r on r.id = c.realm_id
      where r.name = 'master' and c.client_id = 'step-admin-api' and c.enabled
      and not c.public_client and c.service_accounts_enabled
      and not c.standard_flow_enabled and not c.implicit_flow_enabled
      and not c.direct_access_grants_enabled)"
k3s kubectl -n step-apps get secret keycloak-service-credentials \
  -o jsonpath='{.data}' | jq -r '[keys[] | select(startswith("KEYCLOAK_ADMIN_"))]
  | "\(map(select(startswith("KEYCLOAK_ADMIN_CLIENT_"))) | length)|\(map(select(startswith("KEYCLOAK_ADMIN_API_CLIENT_"))) | length)"'
q keycloak -c "select count(*) filter (where not guard),
    count(*) filter (where staff and not separation),
    (select count(*) from client_scope s join realm r on r.id = s.realm_id
      where r.name like 'tenant-%' and s.name = 'offline_access'
      and (exists (select 1 from client_scope_client l where l.scope_id = s.id)
        or exists (select 1 from default_client_scope d where d.scope_id = s.id)))
  from (select r.name not like '%-event-%' as staff,
      exists (select 1 from protocol_mapper p where p.client_id = c.id
        and p.protocol_mapper_name = 'sequent-audit-storage-guard') as guard,
      exists (select 1 from protocol_mapper p where p.client_id = c.id
        and p.protocol_mapper_name = 'sequent-staff-authorization') as separation
    from client c join realm r on r.id = c.realm_id
    where r.name like 'tenant-%'
    and coalesce(c.protocol, 'openid-connect') = 'openid-connect') m"
