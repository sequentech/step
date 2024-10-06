<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Next Release

## Authorize voters to election events

A new keycloak user attribute has been added to specify which election is a user authorized.

### Keycloak: Add a new mapper to voting-portal client
It requires to add to change the configuration a bit:
1. Go to the realm of the election event you want to configure
2. Go to clients > voting-portal > Client scopes > voting-portal-dedicated
3. Click on Add mapper > By configuration > Hasura Multivalue User Attribute
4. Put the following values:
    Name: x-hasura-authorized-election-ids
    User Attribute: authorized-election-ids
    Token Claim Name: https://hasura\.io/jwt/claims.x-hasura-authorized-election-ids
    Claim JSON Type: String
5. Click Save

### Keycloak: Add a new User Profile Attribute
It requires to add to change the configuration a bit:
1. Go to the realm of the election event you want to configure
2. Go to Realm settings > User profile
3. Click on Create Attribute
4. Put the following values:
    Attribute Name: authorized-election-ids
    Display name: Authorized Elections
    Multivalued: On
    Attribute group: None
    Enabled when: Always
    Required field: Off
5. Click save

### Keycloak: New env variable added
An addidiontal environment variable is required for keycloak to make calls to hasura.
    HASURA_ENDPOINT: ${HASURA_ENDPOINT}
    example in dev environment: http://graphql-engine:8080/v1/graphql