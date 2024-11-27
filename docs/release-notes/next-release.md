<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

### Migration to add permissions to keycloak realm

It requires to add a couple of permissions In order use Election event
`Approvals` tab:
1. Go to realm roles, select the admin role and click on `Create role`
2. Add the following roles to the sbei group: `registered-voter-read`, `unverified-voter-read`
