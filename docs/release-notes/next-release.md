<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Automatically create super tenant

This change requires new env vars:  `AWS_S3_JWKS_CERTS_PATH`=certs.json and 
`ENV_SLUG`. `ENV_SLUG` should be the short name of the environment.

Also keycloak no longer needs to import realms at
 `/opt/keycloak/data/import`. Furthermore the file `certs.json` doesn't
 need to exist initially, as windmill-beat will automatically create it
 along with the first tenant.

## 🐞 Uncategorized error while casting ballot

Improve error handling on the Voting Portal when casting a vote. This
includes handling a Timeout, Excess Allowed Revotes, Voting in another
Area, Internal Server Error.

## 🐞 service-account-realm-management shouldn't appear as a voter

This fixes the issue where a service account appears in the voters list.
In order to deploy this in production, the configmap for the default
election event configuration needs to be changed.

## 🐞 Accessing tenant url after logging out does not show tenant selection page.

Previously, if you're logged in to the Admin Portal, and you logged out,
and then went to the /tenant page to select the tenant, the page didn't load
correctly the first time. This change fixes the issue.
