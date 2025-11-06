<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Release 5.0

## Authorize voters to election events

A new keycloak user attribute has been added to specify which election is a user
authorized.

### Migration: Add in Keycloaka new mapper to voting-portal client for Hasura & Harvest

It requires to add to change the configuration a bit:
1. Go to the Realm of the election event you want to configure
2. Go to `Clients` > `Voting Portal` > `Client scopes` >
   `voting-portal-dedicated`
3. Click on `Add mapper` > `By configuration` > `Authorized Election User Attribute`
4. Put the following values:
   - Name: `x-hasura-authorized-election-ids`
   - User Attribute: `authorized-election-ids`
   - Token Claim Name: `https://hasura\.io/jwt/claims.x-hasura-authorized-election-ids`
   - Claim JSON Type: `String`
5. Click `Save`

### Migration: Add in Keycloak a new User Profile Attribute

It requires to add to change the configuration a bit:
1. Go to the realm of the election event you want to configure
2. Go to `Realm settings` > `User profile`
3. Click on `Create Attribute`
4. Put the following values:
   - Attribute Name: `authorized-election-ids`
   - Display name: `Authorized Elections`
   - Multivalued: `On`
   - Attribute group: `None`
   - Enabled when: `Always`
   - Required field: `Off`
5. Click `Save`

### Migration: New env variable added for Keycloak container

An addidiontal environment variable is required for keycloak to make calls to
hasura:

```yaml
HASURA_ENDPOINT: ${HASURA_ENDPOINT}
```

Example in dev environment: `http://graphql-engine:8080/v1/graphql`

## Allow restricting Admins to specfic elections

A feature has been added to restrict access for Admin users to specific
elections. It works in two steps:

1. A new user attribute called `permission_labels` was added to the
admin portal realm in Keycloak and it's multivalued.

2. A new column `permission_label` was added to the `election` table in the DB.
If the `permission_label` for an election is `null`, all admin users can access
it, just like before. However, when `permission_label` is defined, then only
admins matching this label will be able to list this election. This matching is
performed using the `x-hasura-permission-labels` mapper from the user
attribute in the user's JWT.

A new group was added to keycloak called `admin-light` and a new role and
permission in Hasura called `permission-label-write` which the new group does
not have and can't edit the permission label at the election level and at the
user level.

### Migration notes

1. A new user multi-value attribute called `permission_labels` needs to be
   added to the Admin Portal realm.
2. In the Admin Portal Realm, a new custom Mapper to handle multivalued
   attribute for Hasura to read it right.
3. In the Admin Portal Realm a new permission `permission-label-write` was added
   which is now included in the `admin` group, and a new Group is added like
   `admin` without this permission was added,  called `admin-light`.

#### Migration: permission_labels attribute

In the Admin Portal realm in keycloak, go to `Realm settings` > `User Profile`, then click
on `Create Attribute`. The `Attribue [Name]` and `Display name`should be
`permission_labels`. Set `Multivalued` on and in `Permissions` set the `Admin`
with both edit and view permissions. Then click `Save`.

#### Migration: Hasura multivalued mapper

In the Admin Portal realm in keycloak, go to `Clients` > `admin-portal` > `Client scopes`
, then click on `admin-portal-dedicated` and `Add mapper` > `By configuration`, then click on
`Hasura Multivalue User Attribute`. Then configure:
- Name: `x-hasura-permission-labels`.
- User Attribute: `permission_labels`.
- Token Claim Name: `https://hasura\.io/jwt/claims.x-hasura-permission-labels`.

And click `Save`.
