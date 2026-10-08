---
title: Set Up the Tenant
sidebar_position: 1
description: "This procedure prepares the tenant for your election events. You set the tenant settings, you create the accounts of the administrators and trustees, and you make a backup of the configuration."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Set Up the Tenant

This procedure prepares the tenant for your election events. You set the tenant settings, you
create the accounts of the administrators and trustees, and you make a backup of the
configuration. The tenant administrator does this procedure once, before the first election
event.

## Before you start

- Sequent has created your tenant and has given you an account with the `admin` role.
- Sequent has installed the trustee services and has added the trustees to
  **Settings** > **TRUSTEES**.
- You have the list of persons: the administrators and the trustees, with their names and
  email addresses.
- For each trustee person, you know which trustee of the list they represent.

:::note
Only Sequent can create a tenant. The **+** icon next to the tenant name does not work for
other users.
:::

## 1. Set the tenant settings

1. In the menu on the left, click **Settings**.
2. Click each tab and set the values. The table tells what each tab controls.

| Tab | What it controls |
| --- | --- |
| **ELECTION TYPES** | The list of election types of the tenant. |
| **VOTING CHANELS** | The voting channels that the tenant can use: **Online Voting** and **Kiosk Voting**. The admin portal saves a change immediately. |
| **TEMPLATES** | Shows if the tenant can send messages by email (**Mails**) and by text message (**SMS**). You cannot change these values. Ask Sequent support. |
| **LANGUAGES** | The languages that election events can use. Only the languages that you turn on here are available in the election events. |
| **LOCALIZATION** | Changes to the texts of the portals, for each language. Select the language, click **Add**, then type the **Key** and the **Value**. |
| **Look & Feel** | The **Logo URL**, the **Custom CSS** and the **Help Links** of the tenant. The help links show in the **Help** menu. |
| **TRUSTEES** | The list of trustees. Sequent adds the trustees. Do not change this list. |
| **Countries** | The countries from which voters cannot vote or enroll. |
| **Backup / Restore** | The backup and the restore of the tenant configuration. See [step 4](#4-make-a-backup-of-the-tenant-configuration). |

3. On the **LANGUAGES** tab, turn on each language that your election events will use.
4. On the **VOTING CHANELS** tab, turn on each voting channel that your election events will
   use.
5. On the **TRUSTEES** tab, make sure that the list contains all the trustees.

**Expected result:** the settings show the values that you set. The **TRUSTEES** tab shows one
row for each trustee.

## 2. Check the roles

A role is a set of permissions. Each user gets one or more roles.

1. In the menu on the left, click **Users and Roles**.
2. Click the **Roles** tab.
3. Make sure that the list contains the roles that you need, for example `admin` and
   `trustee`.
4. To see the permissions of a role, click the edit icon in the row of the role. Each
   permission has a check box in the **Active** column.

:::caution CAUTION
A change to a role applies immediately to all users with that role. Do not remove a permission
from the `admin` role. If you do, you can lose access to a part of the admin portal.
:::

To create a role:

1. On the **Roles** tab, click **Add**.
2. Type the **Name** of the role.
3. In the permission list, select **Active** for each permission of the role.
4. Click **Save**.

**Expected result:** the message "Role created" shows.

**Expected result:** the **Roles** tab shows the roles with the permissions that you need.

## 3. Create the user accounts

Do these steps for each administrator and for each trustee.

1. In the menu on the left, click **Users and Roles**.
2. Click the **Users** tab.
3. Click **Add**. If the list is empty, click **Create user**.
4. Type the user name and the other fields of the form, for example the email address and the
   name.
5. Select **Enabled**.
6. Type a password in **Password:** and type it again in **Repeat Password:**.
7. Make sure that **Temporary** is on. The user must then change the password at the first
   sign-in.
8. For a trustee only: in **Act as Trustee**, select the trustee that this person represents.
9. In the role list, select **Active** for each role of the user:
    - Administrators: `admin`.
    - Trustees: `trustee`.
10. Click **Save**.
11. Give the user name and the temporary password to the person. Use a secure channel.

:::danger WARNING
Give each trustee their own account and their own trustee in **Act as Trustee**. Two persons
must not share one trustee. If one person controls more trustees than the threshold, that
person can decrypt the votes alone.
:::

**Expected result:** the **Users** tab shows the new users. Each user can sign in, changes
the password and sees the menu items of their role.

## 4. Make a backup of the tenant configuration

Make a backup after you set up the tenant and after each important change.

1. In the menu on the left, click **Settings**.
2. Click the **Backup / Restore** tab.
3. Click **Backup**.
4. Wait for the task to finish. The browser downloads a file with the name
   `tenant-config-<tenant>-export.zip`.
5. Keep the file in a safe location. It contains the configuration of your tenant.

**Expected result:** you have the backup file.

:::caution CAUTION
A restore replaces the tenant configuration, the sign-in configuration and the roles. Do a
restore only with the help of Sequent support.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| The message "You don't have permission to access settings." shows. | Your account does not have the necessary permissions. Ask Sequent support to give your account the `admin` role. |
| The **TRUSTEES** tab is empty or a trustee is missing. | Ask Sequent support to add the trustees. You cannot do the key ceremony without them. |
| **Online Voting** shows as turned on after you turn it off. | This is a display error of version 9.0. Ask Sequent support to confirm the value. |
| **Act as Trustee** is not in the user form. | The user profile of the tenant does not have the trustee field. Ask Sequent support. |
| A user cannot sign in. | Make sure that **Enabled** is selected for the user. On the **Users** tab, use **Change password** to set a new temporary password. |
| The **Add** button is not on the **Users** tab. | In version 9.0 this button needs the **Create Voter** permission. Ask Sequent support to check your role. |
| "Passwords must match" | Type the same password in **Password:** and in **Repeat Password:**. |

## More information

- [Users and Roles](../Reference/User-Manual/Users-and-Roles/admin_portal_reference_user-manual_users-and-roles.md)
- [Users](../Reference/User-Manual/Users-and-Roles/Users/admin_portal_reference_user-manual_users-and-roles_users.md)
- [Roles](../Reference/User-Manual/Users-and-Roles/Roles/admin_portal_reference_user-manual_users-and-roles_roles.md)
- [Permissions](../Reference/User-Manual/Users-and-Roles/Permissions/admin_portal_reference_user-manual_users-and-roles_permissions.md)
- [Settings](../Reference/User-Manual/Settings/admin_portal_reference_user-manual_settings.md)
- [Trustees](../Reference/User-Manual/Settings/Trustees/admin_portal_reference_user-manual_settings_trustees.md)
- [Permission Labels](../Tutorials/admin_portal_tutorials_permission-labels.md)

**Next:** [Create the Election Event](02-event.md).
