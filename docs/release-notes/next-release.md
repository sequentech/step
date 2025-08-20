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

## ✨ Improve demo mode

With this change, the DEMO tiled background and the Demo warning dialog
will appear when entering the voting portal from the preview screen in the
admin portal. Also, the warning dialog will appear on the election start
screen rather than in the election chooser. This includes a fix so that
the demo background/dialog will only appear for elections that don't have
generated keys when voters login to the voting portal. Also, css classes
are added to the demo background and dialog to help custom styling.

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