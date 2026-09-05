---
id: admin_portal_tutorials_voter-communication
title: Voter Communication
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Send a Voter Communication

1. Open an election event and select the **Voters** tab.
2. Select the intended voters or choose an audience such as all voters, voters who have voted, or
   voters who have not voted.
3. Open **Send communication**, select email or SMS, and write the localized content.
4. Send immediately or schedule delivery.

The normal communication permission is required. Delivery runs per voter, so template variables
are resolved separately for every recipient.

## Using Secret Voter Fields

Secret voter fields can be used in email and SMS content without making them visible in the voter
list. For an attribute named `customerReference`, use the same Handlebars shapes as an ordinary
custom voter attribute:

```handlebars
Your reference is {{user.customerReference}}.
```

`user.<attribute>` contains the first value. The complete value array is available through:

```handlebars
{{lookup user.attributes "customerReference"}}
```

When the communication is submitted, the Admin Portal declares each configured secret attribute
whose name appears in the template content. The worker validates that declaration, decrypts only
those fields for the current voter, renders the message, and discards the plaintext context. Secret
fields not referenced by the content are not decrypted or added to the template variables.

Sending or scheduling content that declares a secret field requires
`voter-secret-attribute-read` in addition to the normal notification permission. If the field is no
longer configured as secret, its envelope is invalid, or the operator lacks permission, the job
fails without substituting the stored value.

See [Permissions](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md#secret-voter-field-permissions)
and [Templates](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md#secret-voter-variables).
