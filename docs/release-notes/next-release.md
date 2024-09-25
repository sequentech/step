<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Next release

## Allow to delete an election event

From now on there's a new permission to be able to delete an election event.
Existing deployments need to adjust to this change by adding this permission to
the admin role.

### Steps to Manually Add `election-event-delete` to `admin` group

1. **Login to the Keycloak Admin Console**

   - Open the Keycloak Admin Console in your browser.
   - Log in using your admin credentials.
   - Choose the Realm to be the admin-portal default realm

2. **Create the `election-event-delete` role**

   - Navigate to the `Realm Roles` section under the Admin realm.
   - Click the `Create role` blue button.
   - In the Create Role form, fill the `Role Name` with the value
     `election-event-delete`.
   - Click `Save`.

3. **Add the `election-event-delete` role to the `admin` group**

   - Navigate to the `Groups` section under the Admin realm.
   - Click in the blue `admin` group name in the right side.
   - In the `Group Details` sidebar at the right, click in the `Role Mapping`
     tab.
   - Click the `Assign Role` blue button.
   - In the dialog that appears, click the checkbox for `election-event-delete`
     item.
   - Click the `Assign` button.

## Allow to assign a user to a trustee

From now on, each admin user can be assigned a trustee. Existing deployments
need to adjust to this change by assigning the right trustee to the admin users
that have the trustee role.

### Steps to Manually Assign a Users to a Trustee

1. **Login to the Keycloak Admin Console**

   - Open the Keycloak Admin Console in your browser.
   - Log in using your admin credentials.
   - Choose the Realm to be the admin-portal default realm

2. **Assign `trustee1` to user `trustee1`**

   - Navigate to the `Users and Roles` section under your realm.
   - Click on `...` at the right of user `trustee1`.
   - On the menu that appears, click on `Edit`.
   - In the dialog that appears, set the `Act as a Trustee` field to `trustee1`.
   - Click on `Save`.
   - Repeat the process for `trustee2`.