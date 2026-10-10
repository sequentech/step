---
id: users-and-roles_permissions
title: Permissions
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Permissions are assigned to roles under **Users and Roles** > **Roles**. A user receives the
permissions of the roles assigned to them. Use the narrowest role that covers the user's work;
having access to an election event does not automatically grant access to every action or field in
it.

## Secret Voter Field Permissions

Secret voter fields have independent read and write permissions so an operator can manage a value
without necessarily being able to see it.

| Permission | Allows |
|---|---|
| `voter-secret-attribute-read` | Explicitly reveal a secret value, include decrypted secret fields in a voter export, use declared secret variables in voter-level communications or reports, and download restricted decrypted voter exports. |
| `voter-secret-attribute-write` | Set, replace, clear, or import a secret voter value. It does not allow revealing an existing value. |

These permissions supplement the ordinary permission for the operation:

| Operation | Required permissions |
|---|---|
| Reveal a value | Voter read access and `voter-secret-attribute-read` |
| Create or edit a secret value | The corresponding voter create/write access and `voter-secret-attribute-write` |
| Import a CSV containing a secret column | Voter import/create access and `voter-secret-attribute-write` |
| Export decrypted secret columns | Normal voter export access and `voter-secret-attribute-read` |
| Send or generate an output that declares secret fields | The normal communication/report permission and `voter-secret-attribute-read` |

A standard voter list or export never needs either secret permission because secret columns are
omitted. A decrypted voter-export document remains restricted: downloading it checks
`voter-secret-attribute-read` again.

Assign read and write independently where duties require it. For example, an import operator can
receive secret-write without secret-read, while a support operator who must inspect a value can
receive secret-read without permission to change it.

## User Creation and Import Permissions

Creating or importing tenant users needs `user-create`; creating or importing election-event voters
needs `voter-create`. Some fields determine what a user can access, so setting them needs the same
permissions as changing them on an existing user:

| Operation | Required permissions |
|---|---|
| Assign roles while creating a user | The create permission, `user-write` and `role-write` |
| Import a CSV whose `group_name` column names a group other than `voter` | The create permission, `user-write` and `role-write` |
| Import a CSV with a non-empty `permission_labels` value | The create permission and `permission-label-write` |

If a row needs a permission the importing user lacks, the whole import is rejected and the error
names the row and the column. Empty `group_name` and `permission_labels` cells need no extra
permission.

Users always belong to the tenant they are created or imported into. Creating a user with a
`tenant-id` attribute is rejected. A `tenant-id` column in an import CSV is not used: it must be
empty or hold the id of that tenant, and the import is rejected for any other value.
