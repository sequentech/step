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

## ✨ Voting Portal > Start Screen: Allow Showing Election Event Title instead of Election Title

The title of the Start Screen (Voting Portal) can be to either the election title or the Election Event Title. 
The default value is the Election title, so there is no action required by the admin.

This an be changed at election level > Data > Advanced Configuration.


