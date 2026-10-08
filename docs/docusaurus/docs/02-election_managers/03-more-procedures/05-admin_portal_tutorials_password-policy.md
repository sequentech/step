---
id: admin_portal_tutorials_password-policy
title: Configure Password Policy
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Each election event has its own Keycloak realm and password policy. Election
managers can view and update the managed parts of that policy directly from the
Admin Portal instead of editing the realm in the Keycloak Admin Console.

The policy is enforced when an administrator changes a voter's password and is
also used to generate the voter credential included in a Voter Information
Letter.

## Required permissions

| Operation                            | Required permissions      |
| ------------------------------------ | ------------------------- |
| Open the election event **Data** tab | `election-event-data-tab` |
| View the current policy              | `election-event-read`     |
| Change and save the policy           | `election-event-write`    |

A full administrator normally has these permissions. Custom least-privilege
roles must include them explicitly.

## Configure the policy

1. In the Admin Portal, select the election event.
2. Open the **Data** tab.
3. Expand **Password Policy**.
4. Set the minimum and maximum lengths.
5. Select the character classes that every new password must contain.
6. Select **Save** to update the election event realm.

Hover over the information icon beside a field to see its meaning.

| Field                          | Meaning                                                     |
| ------------------------------ | ----------------------------------------------------------- |
| **Minimum length**             | The minimum number of characters required for the password. |
| **Maximum length**             | The maximum number of characters allowed for the password.  |
| **Include uppercase letters**  | Require at least one uppercase character.                   |
| **Include lowercase letters**  | Require at least one lowercase character.                   |
| **Include digits**             | Require at least one numeric digit.                         |
| **Include special characters** | Require at least one supported special character.           |

Lengths must be whole numbers from 1 through 256, and the minimum cannot exceed
the maximum. Selecting a character class requires at least one character from
that class. Clearing it removes that requirement; it does not prohibit that
character class from appearing in a password.

:::info Default policy

If the event realm does not yet have a password policy, the Admin Portal shows
defaults of 12 minimum characters, 72 maximum characters, and all four character
classes selected. Saving applies those defaults to the realm.

:::

## What changes

Saving updates these Keycloak rules in the election event realm:

- `length`
- `maxLength`
- `upperCase`
- `lowerCase`
- `digits`
- `specialChars`

Other Keycloak password-policy rules that are not managed by this form are
preserved.

The update affects passwords set after the policy is saved. It does not replace
existing voter passwords automatically. If an administrator enters a
non-compliant password through **Voters > Change password**, the dialog remains
open, marks both password fields as invalid, and explains that the password must
be corrected.

:::warning Voter Information Letters

A Voter Information Letter cannot be generated until the election event has a
configured password policy. Each letter generation creates a new voter
credential that complies with the current policy.

:::
