<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Next release

## Allow to assign a user to a trustee

From now on, each admin user can be assigned a trustee. Existing deployments need to adjust to this change by
assigning the right trustee to the admin users that have the trustee role.

### Steps to Manually Assign a Users to a Trustee

1. **Login to the Keycloak Admin Console**
   - Open the Keycloak Admin Console in your browser.
   - Log in using your admin credentials.
   - Choose the Realm to be the admin-portal default realm

2. **Assign `trustee1` to user `trustee1`**
   - Navigate to the `Users and Roles` section under your realm.
   - Click on `...` at the right of user `trustee1`.
   - On the menu that appears, click on `Edit`.
   - In the dialog that appears, set the `trustee` field to `trustee1`.
   - Click on `Save`.