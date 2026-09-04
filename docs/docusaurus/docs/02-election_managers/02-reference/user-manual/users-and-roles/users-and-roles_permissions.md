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
| `voter-secret-attribute-read` | Explicitly reveal a secret value, use declared secret variables in voter-level communications or reports, and download restricted secret-bearing reports. |
| `voter-secret-attribute-write` | Set, replace, clear, or import a secret voter value. It does not allow revealing an existing value. |

These permissions supplement the ordinary permission for the operation:

| Operation | Required permissions |
|---|---|
| Reveal a value | Voter read access and `voter-secret-attribute-read` |
| Create or edit a secret value | The corresponding voter create/write access and `voter-secret-attribute-write` |
| Import a CSV containing a secret column | Voter import/create access and `voter-secret-attribute-write` |
| Send or generate an output that declares secret fields | The normal communication/report permission and `voter-secret-attribute-read` |

A standard voter list or export never needs either secret permission because secret columns are
omitted. A secret-bearing report remains restricted: downloading it checks
`voter-secret-attribute-read` again.

Assign read and write independently where duties require it. For example, an import operator can
receive secret-write without secret-read, while a support operator who must inspect a value can
receive secret-read without permission to change it.
