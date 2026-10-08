---
id: users-and-roles_users
title: Users
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Users** tab is in **Users and Roles**. It shows the accounts of the persons who use the
admin portal: the administrators and the trustees. Use it to create, change, disable and delete
these accounts. The tenant administrator uses it.

Voters are not on this tab. Each election event has its own voter list on its **Voters** tab.

## How to open it

1. In the menu on the left, click **Users and Roles**.
2. Click the **Users** tab.

## Required permission

To open **Users and Roles**, your account must have the **View users and roles** permission. To
see the **Users** tab, it must have the **Read User** permission. Each action needs more
permissions:

| Action | Permissions |
| --- | --- |
| Create a user | **Create Voter** (shows the **Add** button) and **Create User** |
| Edit a user | **Edit Voter** (shows **Edit**) and **Edit User** |
| Delete a user | **Delete Voter** (shows **Delete**) and **Edit User** |
| Change a password | **Change Voter Password** (shows **Change password**) and **Edit User** |
| Import users | **Import Users** (shows **Import**) and **Create User** |
| Export users | **Export Voter** (shows **Export**) and **Read User** |

The `admin` role has all these permissions.

## Columns and buttons

| Item | Meaning |
| --- | --- |
| **Enabled** | A check mark shows that the user can sign in. |
| **Username**, **First Name**, **Last Name** | The data of the user. The columns come from the user profile of the tenant. |
| Other columns | **Id**, **Email**, **Email Verified** and other user data are hidden by default. To show them, click **Columns**. |
| **Actions** | The menu of the user: **Send**, **Edit**, **Delete** and **Change password**. |
| **Add** | Opens the form to create a user. If the list is empty, click **Create user**. |
| **Import** | Creates users from a CSV file. |
| **Export** | Downloads the list of users as a CSV file. |
| **Send** | Sends a notification to the users. |

## The user form

The form shows the fields of the user profile of the tenant. Usually these are **Username**,
**Email**, **First Name** and **Last Name**. A field with `*` is required.

| Field | Meaning | Default |
| --- | --- | --- |
| **Act as Trustee** | The trustee that this person controls. The list shows the names of the **TRUSTEES** tab in **Settings**. Only for trustee users. | Empty |
| **Enabled \*** | If selected, the user can sign in. If not selected, the account is disabled. | Selected |
| **Password:** | The first password of the user. Only in a new user. | Empty |
| **Repeat Password:** | The same password again. If the two are different, the form shows "Passwords must match". Only in a new user. | Empty |
| **Temporary** | If on, the user must change the password at the next sign-in. Only in a new user. | On |
| **Role** / **Active** | The table of roles. Select **Active** for each role of the user. See [Roles](users-and-roles_roles.md). | No role |

**Act as Trustee** shows only when the user profile of the tenant has the trustee field.

## Create an administrator

1. On the **Users** tab, click **Add**.
2. Type the **Username**, the **Email**, the **First Name** and the **Last Name**.
3. Make sure that **Enabled \*** is selected.
4. In **Password:**, type a password.
5. In **Repeat Password:**, type the same password.
6. Make sure that **Temporary** is on.
7. In the role table, select **Active** for the `admin` role.
8. Click **Save**.
9. Give the user name and the password to the person. Use a secure channel.

**Expected result:** the message "Voter created" shows. The user is in the list.

## Create a trustee user

:::danger WARNING
Give each trustee their own account and their own trustee in **Act as Trustee**. Two persons
must not share one trustee. If one person controls as many trustees as the threshold, that
person can decrypt the votes alone.
:::

1. Do steps 1 to 6 of [Create an administrator](#create-an-administrator).
2. In **Act as Trustee**, select the trustee that this person controls.
3. In the role table, select **Active** for the `trustee` role.
4. Click **Save**.
5. Give the user name and the password to the person. Use a secure channel.

**Expected result:** the message "Voter created" shows. When the person signs in, the person
sees the key ceremony and the tally ceremony of their trustee.

## Edit a user

1. In the **Actions** column of the user, open the menu.
2. Click **Edit**.
3. Change the fields, the roles or **Act as Trustee**.
4. Click **Save**. The screen **Review changes** shows each change with its **Current value**
   and its **New value**.
5. Read the changes.
6. Click **Confirm changes**. To correct a value, click **Edit**.

**Expected result:** the message "Voter edited" shows.

If you did not change a value, the message "No changes to review" shows.

## Disable or enable a user

A disabled user cannot sign in. The account and its data stay in the system.

1. Open the user with **Edit**.
2. Clear **Enabled \*** to disable the user, or select it to enable the user.
3. Click **Save**.
4. Click **Confirm changes**.

## Change the password of a user

1. In the **Actions** column of the user, open the menu.
2. Click **Change password**.
3. In **Password:**, type the new password.
4. In **Repeat Password:**, type the same password.
5. Make sure that **Temporary** is on.
6. Click **Save**.
7. Give the new password to the person. Use a secure channel.

**Expected result:** the message "Voter edited" shows.

## Delete a user

:::caution CAUTION
You cannot undo a delete. If the person needs access later, you must create a new account.
To stop access for a short time, disable the user.
:::

1. In the **Actions** column of the user, open the menu.
2. Click **Delete**.
3. Read the message "Are you sure you want to delete this user?".
4. Click **Delete**.

**Expected result:** the message "User deleted" shows. The user is not in the list.

You cannot delete your own account. To delete more than one user, select the users in the
list and click **Delete** above the list.

## Import users

1. Make a CSV file with one row for each user. Use these column names in the first row:
   `username`, `email`, `first_name`, `last_name`, `enabled`, `password` and `group_name`.
2. In `group_name`, type the name of one role, for example `admin`.
3. On the **Users** tab, click **Import**.
4. Drag the CSV file to the panel, or click the panel to select the file.
5. Wait for the message "File uploaded to server - but not imported yet".
6. Click **Import**.
7. Wait for the task **Import Users** to finish.

**Expected result:** the new users are in the list.

:::note
Each row can have only one role in `group_name`. Without the `group_name` column, the users do
not get an administrator role. To add more roles, edit the user after the import.
:::

## Export users

1. On the **Users** tab, click **Export**.
2. Read the message "Export can be a long operation. Are you sure you want to export records?".
3. Click **Export**.
4. Wait for the browser to download the file `users-export.csv`.

The file contains the identifier, the email, the name, the user name, the state and the other
user data. It does not contain the passwords or the roles.

## If there is a problem

| Problem | Action |
| --- | --- |
| The message "You don't have permission to access users or roles." shows. | Ask a tenant administrator to give your account the necessary permissions. |
| A user cannot sign in. | Make sure that **Enabled \*** is selected for the user. Use **Change password** to set a new temporary password. |
| **Act as Trustee** is not in the user form. | The user profile of the tenant does not have the trustee field. Ask Sequent support. |
| **Act as Trustee** has no names. | The **TRUSTEES** tab in **Settings** is empty. Ask Sequent support to add the trustees. |
| A trustee user does not see the ceremony. | Make sure that the user has the `trustee` role and the correct trustee in **Act as Trustee**. Ask the user to sign out and sign in again. |
| The message "Error creating voter: …" shows. | Read the reason in the message. Correct the field, then click **Save** again. |
| The message "Voter created, but their password could not be set" shows. | The account exists without a password. Use **Change password** to set the password. |
| The message "Error editing voter" shows. | Make sure that your account has the necessary permissions. If you changed a password, make sure that it meets the password rules of the tenant. |
| The **Add**, **Import** or **Export** button does not show. | Your account does not have the necessary permission. See [Required permission](#required-permission). |

**Related procedure:** [Set Up the Tenant](../../../03-procedures/01-tenant.md),
step 3.
