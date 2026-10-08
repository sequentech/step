---
id: users-and-roles_roles
title: Roles
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Roles** tab is in **Users and Roles**. Use it to see, create, change and delete the roles
of the tenant. The tenant administrator uses it.

A role is a named set of permissions. Each permission allows one action or one screen in the
admin portal, for example **Create User** or **View settings**. A user gets the permissions of
all the roles that you give to the user. In the sign-in service (Keycloak), a role is a group
of the tenant realm.

For the meaning of some permissions, see [Permissions](users-and-roles_permissions.md).

## How to open it

1. In the menu on the left, click **Users and Roles**.
2. Click the **Roles** tab.

## Required permission

- To open **Users and Roles**, your account must have the **View users and roles** permission,
  and **Read User** or **Read Role**. If not, the screen shows "You don't have permission to
  access users or roles."
- To see the **Roles** tab, your account must have the **Read Role** permission.
- The **Add** button shows only to users with the **Create Role** permission.
- To turn a permission on or off, your account must have the **Edit User Permission** and
  **Edit Role** permissions.
- To delete a role, your account must have the **Edit Role** permission.

## Default roles

Sequent creates the roles of a new tenant. Usually, the tenant has these roles:

| Role | What it allows |
| --- | --- |
| `admin` | Almost all permissions: all menus, all tabs of the election event, the users and roles, the settings and the ceremonies. Give it to the tenant administrators. |
| `admin-light` | A reduced set of administrator permissions. It does not include **Users and Roles**, **Settings** or the creation of a key ceremony. |
| `admin-lockdown` | Only the **Publish** tab of the election event and the publication of results. |
| `trustee` | The **Keys** and **Tally** tabs of the election event and the trustee part of the key ceremony and the tally ceremony. Give it to the trustee users. |

The list of your tenant can be different. To see the permissions of a role, click the edit
icon of the role.

## Columns and buttons

| Item | Meaning |
| --- | --- |
| **Name** | The name of the role. |
| **Id** | The internal identifier of the role. |
| Edit icon | Opens **Role Data**: the name of the role and its permissions. |
| Delete icon | Deletes the role, after a confirmation. |
| **Add** | Opens **Create role**. |
| **Columns** | Shows or hides columns. |

The permission table has two columns: **Permission** (the name of the permission) and
**Active** (a checkbox). In **Role Data**, use the search field above the table to find a
permission.

## Create a role

1. On the **Roles** tab, click **Add**.
2. In **Name**, type the name of the role.
3. In the table, select **Active** for each permission of the role.
4. Click **Save**.

**Expected result:** the message "Role created" shows. The new role is in the list.

## Change the permissions of a role

:::caution CAUTION
A change to a role applies immediately to all users with that role. Do not remove a permission
from the `admin` role. If you do, you can lose access to a part of the admin portal.
:::

1. On the **Roles** tab, click the edit icon of the role.
2. Find the permission in the table.
3. Click the **Active** checkbox of the permission.

**Expected result:** the message "Permission edited" shows. There is no **Save** button: each
click saves the change.

You cannot change the name of a role. To rename a role, create a new role, move the users to
it, then delete the old role.

## Delete a role

:::caution CAUTION
Make sure that no user needs the role. When you delete a role, all its users lose its
permissions. You cannot undo this action.
:::

1. On the **Roles** tab, click the delete icon of the role.
2. Read the message "Are you sure you want to delete this role?".
3. Click **Delete**.

**Expected result:** the message "Role deleted" shows. The role is not in the list.

## If there is a problem

| Problem | Action |
| --- | --- |
| The message "You don't have permission to access users or roles." shows. | Ask a tenant administrator to give your account the necessary permissions. |
| The **Roles** tab or the **Add** button does not show. | Your account does not have the **Read Role** or **Create Role** permission. |
| An error message shows when you save a new role. | Make sure that no other role has the same name. Then try again. |
| The **Active** checkbox does not change, or the message "Permission edited" does not show. | Make sure that your account has the **Edit User Permission** and **Edit Role** permissions. |
| The role stays in the list after you delete it. | Make sure that your account has the **Edit Role** permission. |
| A user does not see a new permission. | Ask the user to sign out and sign in again. |

**Related procedure:** [01-TENANT: Set up the tenant](../../../03-procedures/01-tenant.md),
step 2.
