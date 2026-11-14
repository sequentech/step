---
id: admin_portal_reference_user_manual_users_and_roles_roles
title: Roles
description: "The Roles tab holds the roles of the tenant. A role is a set of permissions. Each user gets one or more roles on the Users tab. A change to a role applies at once to all its users."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Roles** tab holds the roles of the tenant. A role is a set of permissions. Each user gets
one or more roles on the **Users** tab. A change to a role applies at once to all its users.

## Open the tab

1. In the menu on the left, click **Users and Roles**.
2. Click the **Roles** tab.

The tab needs the **Read Role** permission. To create a role, your role needs **Create Role**.

## Default roles

These roles are in a new tenant. Your organization can change them.

| Role | Use |
| --- | --- |
| `admin` | Election and tenant administrators. It has almost all permissions, but not **Trustee Ceremony**. |
| `admin-light` | Administrators with fewer permissions. Most tabs of the election event are hidden. |
| `admin-lockdown` | Administrators of a locked-down election event. Only the **Publish** permissions. |
| `trustee` | Trustees. It has **Trustee Ceremony** and the permissions to see the **Keys** and **Tally** tabs. |

## List

| Column | Content |
| --- | --- |
| **Name** | The name of the role. |
| **Id** | The identifier of the role. |
| Actions | Edit and delete icons. |

## Create a role

1. Click **Add**.
2. Type the **Name**.
3. In the list, select **Active** for each **Permission** of the role.
4. Click **Save**.

**Expected result:** the message "Role created" shows.

## Change a role

1. Click the edit icon of the role.
2. Select or clear **Active** for a permission.

**Expected result:** the message "Permission edited" shows. You cannot change the **Name** of a
role.

:::caution CAUTION
Do not remove permissions from your own role. If you do, you can lose access to a part of the
admin portal.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| "Error creating role" | Check the name, then save again. |
| "Error editing permission" | Try again. |
| "Error deleting role" | The role could not be deleted. |

## Related pages

- [Permissions](../Permissions/admin_portal_reference_user-manual_users-and-roles_permissions.md)
- [Users](../Users/admin_portal_reference_user-manual_users-and-roles_users.md)
- [Set Up the Tenant](../../../../procedures/01-tenant.md#2-check-the-roles)
