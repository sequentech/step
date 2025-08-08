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

## ✨ Add support retrieving master secret in an env variable

A new environment variable `MASTER_SECRET` has been added to use in DEV evironment instead of hashicorp.
`SECRETS_BACKEND` was updated to `SECRETS_BACKEND=EnvVarMasterSecret` accordingly.

This change should not affect production, there the value should be `SECRETS_BACKEND=AwsSecretManager`, more info in `.devcontainer/.env.development`.

The Braid Trustee service and its initialization script (`trustee.sh`) have been updated also support the env vars secrets backends.

## ✨ Read tally in frontend from Sqlite3

With this change, the admin portal starts reading the results directly
from the Sqlite3 file produced by the Tally. This makes it faster and
more scalable.
