---
id: admin_portal_reference_user_manual_users_and_roles_users
title: Users
description: "The Users tab holds the accounts of the administrators and trustees of the tenant. Each user has one or more roles. A trustee account is also linked to a trustee of Settings > TRUSTEES."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Users** tab holds the accounts of the administrators and trustees of the tenant. Each
user has one or more roles. A trustee account is also linked to a trustee of
**Settings** > **TRUSTEES**.

## Open the tab

1. In the menu on the left, click **Users and Roles**.
2. Click the **Users** tab.

The tab needs the **Read User** permission.

## Toolbar

| Button | Use | Permission |
| --- | --- | --- |
| **Columns** | Selects the columns. | **View Election Event Voters Columns** |
| **Add filter** | Filters the list. | **View Election Event Voters Filters** |
| **Add** | Opens the form to create a user. | **Create Voter** |
| **Import** | Imports users from a file. | **Import Users** |
| **Export** | Exports the list of users. | **Export Voter** |
| **Send** | Sends a message to the users. | **Send Notification** |

In version 9.0 some buttons of this tab use voter permissions, as the table shows. When the list
is empty, the tab shows **Create user**.

## Actions of a user

| Action | Use |
| --- | --- |
| **Send** | Sends a message to this user. |
| **Edit** | Changes the data and the roles of the user. |
| **Delete** | Deletes the user, after the question "Are you sure you want to delete this user?". You cannot delete your own account. |
| **Change password** | Sets a new password. |

## User form

| Field | Use |
| --- | --- |
| Profile fields | For example **Username**, **Email**, **First Name** and **Last Name**. **Username** is always required. |
| **Act as Trustee** | For a trustee only: the trustee of **Settings** > **TRUSTEES** that this person represents. |
| **Permission Label** | The permission labels of the user. It needs the **Edit Permission Label** permission. See [Permission Labels](../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| **Enabled \*** | If the user can sign in. |
| **Password:** and **Repeat Password:** | The password. |
| **Temporary** | If on, the user must change the password at the next sign-in. It is on by default. |
| **Role** and **Active** | The roles of the user. Select **Active** for each role. For an existing user, the change is saved immediately. |
| **Save** | Saves the user. |

The fields come from the user profile of the tenant. **Act as Trustee** and **Permission Label**
show only if the user profile has these attributes.

:::danger WARNING
Give each trustee their own account and their own trustee in **Act as Trustee**. Two persons
must not share one trustee.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| "Passwords must match" | Type the same password in both fields. |
| "Error creating voter" when you create a user | Version 9.0 uses the voter messages for users. Check the required fields and the user name. |
| A role change shows a text code such as `usersAndRolesScreen.roles.notifications.permissionEditSuccess`. | The message is not translated in version 9.0. The code ending in `Success` means that the change is saved. |
| **Act as Trustee** is not in the form. | The user profile has no trustee attribute. Ask Sequent support. |
| The **Add** button is not there. | Your role does not have the **Create Voter** permission. |

## Related pages

- [Set Up the Tenant](../../../../procedures/01-tenant.md#3-create-the-user-accounts)
- [Roles](../Roles/admin_portal_reference_user-manual_users-and-roles_roles.md)
- [Trustees](../../Settings/Trustees/admin_portal_reference_user-manual_settings_trustees.md)
