<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release 3.5

## Manual Migration of Permissions in Old Deployments

The following steps explain how to manually add the required roles and assign them appropriately in the Admin Portal.

### Steps to Manually Migrate Permissions

1. **Login to the Keycloak Admin Console**
   - Open the Keycloak Admin Console in your browser.
   - Log in using your admin credentials.
   - Choose the Realm to be the admin-portal default realm

2. **Add the `tasks-read` Role**
   - Navigate to the `Realm roles` section under your realm.
   - Click on `Create Role`.
   - Set the role name to `tasks-read`.
   - Save the new role.

3. **Assign the `tasks-read` Role to the `admin` Role**
   - Go to the `Groups` section.
   - Select the `admin` group from the list.
   - Navigate to the `Role Mappings` tab for the `admin` role.
   - Click `Assign role` to assign the `tasks-read` role to the `admin` role.
   - Choose `tasks-read` and click `Assign`.

4. **Repeat for All Relevant Tenants**
   - For each tenant in your system, repeat the above steps to ensure the `tasks-read` role is assigned correctly.

By following these steps, you will ensure that permissions are correctly set up for older deployments.

---
**Note:** These steps are only required for older deployments. New deployments will have the `tasks-read` role automatically assigned.
