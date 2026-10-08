---
id: settings_trustees
title: Trustees
description: "The TRUSTEES tab is in Settings. It shows the list of trustees of the tenant."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **TRUSTEES** tab is in **Settings**. It shows the list of trustees of the tenant.

A trustee is a service that keeps one part of the election key. The votes can be decrypted
only when enough trustees work together. Each trustee service has a person who controls it: the
trustee user. The key ceremony and the tally ceremony show this list so that you can select the
trustees of an election event.

Sequent installs the trustee services and adds them to this list. You do not create trustees
yourself. Use this tab to check that the list is complete before the key ceremony.

For the technical configuration of the trustee services, see
[Braid Trustees Configuration](../../../../07-developers/09-braid/braid_trustees_configuration.md).

## How to open it

1. In the menu on the left, click **Settings**.
2. Click the **TRUSTEES** tab.

## Required permission

- To open **Settings**, your account must have the **Edit Tenant** and **View settings**
  permissions.
- To see the list, your account must have the **Read Trustee** permission. Without it, the tab
  shows "No Trustees yet."
- The **Add** button shows only to users with the **Edit Trustee** permission.
- The **Export** button shows only to users with the **Export Trustees** permission.

## Columns and buttons

| Item | Meaning |
| --- | --- |
| **Name** | The name of the trustee. It must be the same as the name of the trustee service. The user form shows this name in **Act as Trustee**. |
| **Public key** | The public key of the trustee service. Hidden by default. To show it, click **Columns**. |
| **Id** | The internal identifier of the trustee. Hidden by default. |
| Edit icon | Opens **Edit Trustee**, with the fields **Name** and **Public key**. |
| Delete icon | Removes the trustee from the list, after a confirmation. |
| **Add** | Opens **Create Trustee**, with the fields **Name** and **Public key**. |
| **Export** | Makes an encrypted file with the configuration of the trustee services. |
| **Filter** | Filters the list by **Name** or **Public Key**. |
| **Columns** | Shows or hides columns. |

:::danger WARNING
Do not add, edit or delete a trustee unless Sequent support asks you to. A wrong name or public
key stops the key ceremony or the tally ceremony. If you delete a trustee that keeps a part of
an election key, the votes of that election event can become impossible to decrypt.
:::

## Link a trustee user to a trustee

The trustee user is a person with an account in the admin portal. To link the account to a
trustee:

1. In the menu on the left, click **Users and Roles**.
2. On the **Users** tab, create or edit the user.
3. In **Act as Trustee**, select the name of the trustee.
4. In the role list, select the `trustee` role.
5. Save the user.

See [Users](../users-and-roles/users-and-roles_users.md) for the full steps.

:::danger WARNING
Link each trustee to one person only, and link each person to one trustee only. If one person
controls as many trustees as the threshold, that person can decrypt the votes alone.
:::

## Export the trustee configuration

The export file contains secret configuration of the trustee services. Do this only when
Sequent support asks you to.

1. Click **Export**.
2. Wait for the task **Export Trustees** to finish.
3. Wait for the browser to download the file `trustees-export.ezip`.
4. The dialog **Password** shows the password of the file. Click the copy button and keep the
   password.

:::danger WARNING
Keep the file and its password in two different safe locations. A person with both can read
the secret configuration of the trustee services. The admin portal shows the password only
once.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| The tab shows "No Trustees yet." | Your account does not have the **Read Trustee** permission, or Sequent has not added the trustees. Ask Sequent support. |
| A trustee is missing from the list. | Ask Sequent support to add the trustee. You cannot do the key ceremony without all the trustees. |
| A trustee user does not see the key ceremony. | Make sure that the user has the `trustee` role and the correct trustee in **Act as Trustee**. |
| The **Add** or **Export** button does not show. | Your account does not have the necessary permission. |

**Related procedures:** [Set Up the Tenant](../../../03-procedures/01-tenant.md) and
[Run the Key Ceremony](../../../03-procedures/03-keys.md).
