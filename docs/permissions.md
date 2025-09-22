<!--
SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Permissions system

The system uses [Keycloak](https://www.keycloak.org/) for the permissions system. The frontend interacts with Keycloak using graphql/Hasura, which calls the `harvest` service thorugh Hasura actions and `harvest` is the service that calls Keycloak. 

The main concepts in the permissions system are Tenants, Election Events, Admin Users, Voter Users, Permissions and Roles. In Keycloak both Tenants and Election Events map to Reigns, so each Tenant and each Election Event have a unique Reign. Admin Users are Keycloak users in a reign for a Tenant, and Voter users are Keycloak users in a reign for an Election Event. Both Admin users and Voter users can be assigned Permissions and Roles. A Permission enables assigned users to perform different actions including reading and modifying data.

Roles aggregate permissions. For Permissions we use Keycloak Realm Roles, and for Roles we use Keycloak Groups. Hasura understands permissions but not roles, and permissions are called "Roles" in Hasura. Keycloak also requires Clients and Client Scopes, so users need to be mapped/assigned to a Client. This client will have a Client Scope with a list of dedicated mappers so that hasura can correctly authenticate it:

- Name: x-hasura-default-role. Type: Hardcoded claim. Token Claim Name: https://hasura\.io/jwt/claims.x-hasura-default-role . Claim Value: admin-user (for example). 
- Name: x-hasura-user-id. Type: User Property. Property: id: Token Claim Name: https://hasura\.io/jwt/claims.x-hasura-user-id .
- Name: x-hasura-tenant-id. Type: User Attribute. User Attribute: tenant-id. Token Claim Name: https://hasura\.io/jwt/claims.x-hasura-tenant-id .
- Name: x-hasura-allowed-roles. Type: User Realm Role. Token Claim Name: https://hasura\.io/jwt/claims.x-hasura-allowed-roles . Multivalued: true/On.

Users (both admin and voter users) need to have configured an attribute: tenant-id. This is used by Hasura to check permissions. Also in hasura all tables are permissioned. Permissions in Hasura are called roles, and for each relevant permission there's a Hasura role on each table defining the access level (insert, select, update and delete).

# API

CRUD endpoints for:
- Admin/Voter Users.
- Permissions.
- Roles.

# Permissions

- tenant-create|read|write
- election-event-create|read|write|delete|archive
- election-create|read|write|delete
- voter-create|read|write
- user-create|read|write
- user-permission-create|read|write
- role-create|read|write|assign
- communication-template-create|read|write
- notification-read|write|send
- area-read|write
- election-state-write
- election-type-create|read|write
- voting-channel-read|write
- trustee-create|read|write
- tally-read|start|write
- tally-results-read
- publish-read|write
- logs-read
- keys-read
- contest-create|read|write|delete
- candidate-create|read|write|delete
- election-data-tab|approvals-tab
- election-event-areas-tab|data-tab|keys-tab|logs-tab|publish-tab|reports-tab|scheduled-tab|tally-tab|tasks-tab|voters-tab|approvals-tab
- election-publish-tab|voters-tab
