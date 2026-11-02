---
id: admin_portal_tutorials_create-voters
title: Create Voters
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Create a Voter

1. Open the election event and select the **Voters** tab.
2. Select **Add**.
3. Complete the voter fields. Custom fields and their order come from the election event's
   Keycloak User Profile.
4. Review the changes and save the voter.

For instructions on defining custom fields, see
[Adding User Attributes to Keycloak](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

## Secret Voter Fields

Fields configured with `sequent.secret=true` are stored encrypted. They are not available as voter
list columns, filters, or sort fields, and their stored value is never loaded with the ordinary
voter record.

The permissions to reveal and edit these fields are independent:

- With `voter-secret-attribute-write`, enter a value when creating a voter or type a replacement
  while editing one. The previous value does not need to be revealed before replacement.
- With `voter-secret-attribute-read`, select the eye button to reveal the value. The button is
  disabled while decryption is in progress. Select it again to hide the value.
- Both operations also require the corresponding ordinary voter read or write permission.

When editing a voter, an existing unrevealed value is displayed as `••••••••`. Leaving it untouched
preserves the encrypted value. Use the **Clear** (×) icon at the right of the field and save to remove a value without revealing it. Required fields still need a value. For multivalued fields, each value can be edited, added, or removed separately.
Review screens mask new and changed secret values rather than repeating them.

A secret can also be used as the login credential with the
[Multi-Attribute + Password Form](./101-admin_portal_tutorials_multi-attribute-password-login.md#optional-use-an-encrypted-voter-attribute-as-the-credential).
In that mode, fill the configured secret field (for example, **Mysecret**); the separate Keycloak
password and password-reset controls do not set or reset that secret.

Revealed values are limited to the open voter editor. Closing the editor removes them from its
state; the voter list continues to receive only redacted values.

## Export Voters

Select **Export** from the Voters tab to generate a CSV.

- The default export omits every configured secret column. It does not export encrypted envelopes.
- A user with `voter-secret-attribute-read` can select **Include decrypted secret voter fields** in
  the confirmation dialog. This adds the secret columns with plaintext values.
- Treat a decrypted export as sensitive data. The generated document remains protected by the
  secret-read permission when it is downloaded.
- Each election in the export adds a participation column: `true` when the voter has voted in it,
  empty otherwise. The column is named after the election's external ID, not its name:
  `election__external_id__` followed by the external ID's UTF-8 bytes in lowercase hexadecimal.
  For example, external ID `GE-2026` gives `election__external_id__47452d32303236`. The ID is
  encoded exactly as stored, so IDs that differ only by case, punctuation or spacing get different
  columns.
- Every election in the export needs an external ID, unique within the election event. Otherwise
  the export stops and names the elections to fix. This also applies to an election event export
  that includes voters.
- Participation columns are report-only: import never stores them as voter attributes or voting
  history. Exports made before this change, with columns named after
  each election's alias or name, still import. Import still rejects a file that repeats a column,
  including a participation column repeated by an older export: export again, or delete the
  repeated column.
- A deployment older than this change also accepts the new columns, unless two external IDs start
  with the same 20 bytes, for example a 20-byte ID and a longer ID that begins with it. Its import
  keeps only the first 63 characters of a column name, so it fails with a database error that a
  column is specified more than once.
- Importing an exported election event can change an external ID that has the form of a UUID. The
  participation columns of a voters file exported with that event then no longer match the
  imported election. Import still does not store them.

For the complete permission combinations, see
[Permissions](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md#secret-voter-field-permissions).
