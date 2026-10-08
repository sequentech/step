---
id: admin_portal_reference_user_manual_settings_trustees
title: Trustees Settings
sidebar_label: Trustees
description: "The TRUSTEES tab is the list of the trustees of the tenant. A trustee keeps one fragment of the private key of an election. The key ceremony and the tally ceremony use the trustees of this list."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **TRUSTEES** tab is the list of the trustees of the tenant. A trustee keeps one fragment of
the private key of an election. The key ceremony and the tally ceremony use the trustees of
this list. Each trustee in the list is also a trustee service that Sequent installs. Sequent
adds the trustees. Do not change this list.

## Open the tab

1. In the menu on the left, click **Settings**.
2. Click the **TRUSTEES** tab.

The actions need these permissions:

| Action | Permission |
| --- | --- |
| See the list | **Read Trustee** |
| Create and edit a trustee | **Edit Trustee** |
| Export the list | **Export Trustees** |

## Screen elements

| Element | Use |
| --- | --- |
| **Create Trustee** | Opens the form **Create Trustee**, with the fields **Name** and **Public key**. |
| **Add filter** | Filters by **Name** or **Public Key**. |
| **Export** | Exports the list. The task "Export Trustees" makes the file. |
| **Id**, **Public key**, **Name** | The columns of the list. |
| Edit and delete icons | Change or delete a trustee. |

When the list is empty, the tab shows "No Trustees yet.".

## Link a person to a trustee

A trustee person signs in with their own account. The account is linked to a trustee of this
list by **Act as Trustee** on the **Users** tab. See
[Users](../../Users-and-Roles/Users/admin_portal_reference_user-manual_users-and-roles_users.md).

:::danger WARNING
Give each trustee of the list to one person only. If one person controls more trustees than
the threshold, that person can decrypt the votes alone.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| The list is empty or a trustee is missing. | Ask Sequent support to add the trustees. |
| A trustee is not in **Act as Trustee**. | The trustee is not in this list. Ask Sequent support. |

## Technical documentation

For the configuration of the trustee services, see the
[Braid Trustees Configuration Guide](/docs/developers/Braid/braid_trustees_configuration).

## Related pages

- [Run the Key Ceremony](../../../../procedures/04-keys.md)
- [Set Up the Tenant](../../../../procedures/01-tenant.md)
- [Settings](../admin_portal_reference_user-manual_settings.md)
