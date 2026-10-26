# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl -n step-infra exec -i deploy/postgres -- psql -U postgres -tA -d keycloak <<'EOF'
with recursive used(id) as (
  select unnest(array[browser_flow, registration_flow, direct_grant_flow,
    reset_credentials_flow, client_auth_flow, docker_auth_flow]) from realm
  union select value from realm_attribute where name = 'firstBrokerLoginFlowId'
  union select flow_id from client_auth_flow_bindings
  union select first_broker_login_flow_id from identity_provider
  union select post_broker_login_flow_id from identity_provider
  union select e.auth_flow_id from authentication_execution e
    join used u on u.id = e.flow_id)
select count(*) from authentication_execution e join used u on u.id = e.flow_id
  where e.authenticator = 'smart-link'
EOF
