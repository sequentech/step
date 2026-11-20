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
| **ELECTION TYPES** | Not in use. You do not have to set it. |
| **VOTING CHANELS** | The voting channels that a new election event gets when you create it. After that, each election event sets its own channels in **Voting Channels Allowed** on its **Data** tab. See [Voting channels](../02-reference/user-manual/settings/settings_voting-channels.md). |
| **TEMPLATES** | Not in use. The switches cannot be changed. |
| **LANGUAGES** | The languages that election events can use, the **Default Language** and the **Language Detection Policy**. |
| **LOCALIZATION** | Changes to the texts of the portals, for each language. Select the language, click **Add**, then type the **Key** and the **Value**. |
| **Integrations** | The Google Calendar account that makes video meeting links. See [Google Meet credentials](../01-tutorials/98-admin_portal_tutorials_create-and-set-google-meet-credentials.md). |
| **Look & Feel** | The **Logo URL**, the **Custom CSS** and the **Help Links** of the admin portal. They do not change the voting portal. The help links show in the **Help** menu. |
| **TRUSTEES** | The list of trustees. Sequent adds the trustees. Do not change this list. See [Trustees](../02-reference/user-manual/settings/settings_trustees.md). |
| **Countries** | The countries from which voters cannot vote or enroll. It works only if Sequent has set up country blocking for your platform. See [Countries](../02-reference/user-manual/settings/settings_countries.md). |
| **Backup / Restore** | The backup and the restore of the tenant configuration. See [step 4](#4-make-a-backup-of-the-tenant-configuration). |
| **Previews** | A record of the ballot previews that external systems requested. You cannot change it. |

3. On the **LANGUAGES** tab, turn on each language that your election events will use.
   Select the **Default Language**. In **Language Detection Policy**, select **Browser Detect**
   to show the language of the voter's browser, or **Force Default** to always show the default
   language.
4. On the **VOTING CHANELS** tab, turn on the channels that your election events will use.
5. On the **TRUSTEES** tab, make sure that the list contains all the trustees.

**Expected result:** the settings show the values that you set. The **TRUSTEES** tab shows one
row for each trustee.

:::note
A new election event copies the language settings and the voting channels of the tenant when
you create it. A change on the **LANGUAGES** or **VOTING CHANELS** tab does not change the
election events that exist.
:::

## 2. Check the roles

A role is a set of permissions. Each user gets one or more roles.

1. In the menu on the left, click **Users and Roles**.
2. Click the **Roles** tab.
3. Make sure that the list contains the roles that you need, for example `admin` and
   `trustee`.
4. To see the permissions of a role, click its edit icon. The **Role Data** panel shows each
   permission with an **Active** checkbox.

:::caution CAUTION
A change to a role applies immediately to all users with that role. Do not remove a permission
from the `admin` role. If you do, you can lose access to a part of the admin portal.
:::

To create a role:

1. On the **Roles** tab, click **Add**.
2. Type the **Name** of the role. You cannot change the name later.
3. In the permission table, select **Active** for each permission of the role.
4. Click **Save**.

For all the options, see [Roles](../02-reference/user-manual/users-and-roles/users-and-roles_roles.md).

**Expected result:** the **Roles** tab shows the roles with the permissions that you need.

## 3. Create the user accounts

Do these steps for each administrator and for each trustee.

1. In the menu on the left, click **Users and Roles**.
2. Click the **Users** tab.
3. Click **Add**. If the list is empty, click **Create user**.
4. Type the user name and the other fields of the form, for example the email address and the
   name.
5. Make sure that **Enabled** is selected. It is selected by default.
6. Type a password in **Password:** and type it again in **Repeat Password:**.
7. Make sure that **Temporary** is on. It is on by default. The user must then change the
   password at the first sign-in.
8. For a trustee only: in **Act as Trustee**, select the trustee that this person represents.
9. In the **Role** table of the form, select **Active** for each role of the user:
    - Administrators: `admin`.
    - Trustees: `trustee`.
10. Click **Save**. The message "Voter created" shows. The admin portal uses this message for
    all users.
11. Give the user name and the temporary password to the person. Use a secure channel.

:::danger WARNING
Give each trustee their own account and their own trustee in **Act as Trustee**. Two persons
must not share one trustee. If one person controls more trustees than the threshold, that
person can decrypt the votes alone.
:::

**Expected result:** the **Users** tab shows the new users. Each user can sign in, changes
the password and sees the menu items of their role.

To create many users from a CSV file, see [Users](../02-reference/user-manual/users-and-roles/users-and-roles_users.md).
Give each row a `group_name` column with the role, for example `admin`. Without it, the
imported user gets no administrator role.

## 4. Make a backup of the tenant configuration

Make a backup after you set up the tenant and after each important change.

1. In the menu on the left, click **Settings**.
2. Click the **Backup / Restore** tab.
3. Click **Backup**.
4. Wait for the task to finish. The browser downloads a file with the name
   `tenant-config-<tenant ID>-export.zip`.
5. Keep the file in a safe location. It contains the configuration of your tenant.

**Expected result:** you have the backup file.

:::caution CAUTION
A restore changes the tenant configuration, the texts of the sign-in pages and the roles in the
file. Each role in the file gets exactly the permissions of the file. Do a restore only with the
help of Sequent support. See [Backup / Restore](../02-reference/user-manual/settings/settings_backup-restore.md).
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| The message "You don't have permission to access settings." shows. | Your account does not have the necessary permissions. Ask Sequent support to give your account the `admin` role. |
| The **TRUSTEES** tab shows "No Trustees yet." | Your account needs the **Read Trustee** permission. If it has it, ask Sequent support to add the trustees. You cannot do the key ceremony without them. |
| **Act as Trustee** is not in the user form. | The user profile of the tenant does not have the trustee field. Ask Sequent support. |
| The admin portal shows **Review changes** when you save a user. | Read the changes, then click **Confirm changes**. |
| A user cannot sign in. | Make sure that **Enabled** is selected for the user. On the **Users** tab, use **Change password** to set a new temporary password. |

## More information

- [Settings: Languages](../02-reference/user-manual/settings/settings_languages.md)
- [Settings: Localization](../02-reference/user-manual/settings/settings_localization.md)
- [Settings: Integrations](../02-reference/user-manual/settings/settings_integrations.md)
- [Settings: Look & Feel](../02-reference/user-manual/settings/settings_look-feel.md)
- [Settings: Previews](../02-reference/user-manual/settings/settings_previews.md)
- [Users](../02-reference/user-manual/users-and-roles/users-and-roles_users.md)
- [Roles](../02-reference/user-manual/users-and-roles/users-and-roles_roles.md)
- [Permissions](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md)
- [Navigate the admin portal](../01-tutorials/01-admin_portal_tutorials_system-navigation.md)

**Next:** [Create the Election Event](02-event.md).
