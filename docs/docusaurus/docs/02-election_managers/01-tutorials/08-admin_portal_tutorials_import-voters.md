---
id: admin_portal_tutorials_import-voters
title: Import Voter List
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Prepare the CSV

Use one row per voter. Column names must match the canonical voter and Keycloak User Profile
attribute names; the display label shown in the Admin Portal is not the import header. For example,
an attribute named `customerReference` uses this header:

```csv
username,email,customerReference
voter-001,voter-001@example.com,REF-12345
```

Use `|` between values for a multi-valued User Profile attribute. Keep the source CSV private: it
contains the values in plaintext even when a column is configured as secret.

## Import the Voters

1. Open the election event and select the **Voters** tab.
2. Select the voter import action and upload the CSV.
3. Review the task result and correct any reported row or header errors.

## Importing Secret Voter Fields

A CSV can contain a field configured with `sequent.secret=true`. Supply the normal plaintext value;
do not pre-encrypt it and do not copy an encrypted `seqenc:` value from Keycloak. The import worker
creates the voter ID and encrypts each non-empty secret cell before it is written to Keycloak.

The initiating user needs `voter-secret-attribute-write` in addition to the normal voter import and
create access. If the CSV contains a secret header and the user lacks that permission, the whole
import is rejected and the error identifies the affected columns.

After import:

- ordinary voter views and exports do not return the secret value or its encrypted envelope;
- users with secret-read permission can reveal it from the voter editor;
- exports always omit secret fields, including password-protected election-event archives.

See [Protecting a Voter Attribute as Secret](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md#protecting-a-voter-attribute-as-secret)
for configuration restrictions, and
[Permissions](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md#secret-voter-field-permissions)
for authorization details.
