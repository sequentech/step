<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## 🐞 Hasura stops working when assigning too many permissions

In order to reduce the jwt size for tenant realms, we need to remove the "realm_access" key from the jwt.
To do this, go to each tenant realm in Keycloak, then `Client scopes` > `roles` > `Mappers` > `realm roles`
then set `Add to ID token` Off.
