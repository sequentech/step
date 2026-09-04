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
preserves the encrypted value. Use **Clear** and save to remove a value without revealing it. For multivalued fields, each value can be edited, added, or removed separately.
Review screens mask new and changed secret values rather than repeating them.

Revealed values are limited to the open voter editor. Closing the editor removes them from its
state; the voter list continues to receive only redacted values.

## Export Voters

Select **Export** from the Voters tab to generate a CSV.

- The default export omits every configured secret column. It does not export encrypted envelopes.
- A user with `voter-secret-attribute-read` can select **Include decrypted secret voter fields** in
  the confirmation dialog. This adds the secret columns with plaintext values.
- Treat a decrypted export as sensitive data. The generated document remains protected by the
  secret-read permission when it is downloaded.

For the complete permission combinations, see
[Permissions](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md#secret-voter-field-permissions).
