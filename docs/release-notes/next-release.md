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

## 🐞 Intermitten errors loading preview

Fix a race condition for calling WASM code when loading the voting portal that
was sometimes causing an error.

## 🐞 Voter actions are not logged

Voter actions were not being logged because they were published to a message queue
that didn't include the environment prefix.

## 🐞 Can't cast vote

When an Election was created manually through the Admin Portal, the voting channels
column was left empty. This means voters couldn't cast their vote as the online
channel was not set active.

## 🐞 Tenant/Event keycloak configs have static secrets 

When a new tenant or event is created, some clients have secrets and they are 
being imported as-is. When creating/importing a new tenant/event, now the secrets are 
stripped from the config to be regenerated. 

## 🐞 Default language in the voting portal is not honored in preview mode

Previously the default language was not being selected when loading the Voting
Portal, now it is.

## 🐞 Voters can't login to election events in new tenants

For security, secrets/certificates are generated randomly when creating a new
election event/tenant. However the secret for the service account of the tenant
should be set by the system as it is used internally. This is now set by
environment variables  `KEYCLOAK_CLIENT_ID` and `KEYCLOAK_CLIENT_SECRET`.

## 🐞 Keycloak voter logs are not recorded

Voter logs related to Keycloak (login, login error, code to token) were being 
published to the wrong rabbitmq queue. This has been fixed and now they are 
published to the queue for the respective environment.
