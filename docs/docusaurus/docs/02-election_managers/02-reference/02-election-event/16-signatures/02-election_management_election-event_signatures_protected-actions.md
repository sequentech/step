---
id: election_management_election_event_signatures_protected_actions
title: Protected actions
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

**Signatures** > **Protected actions** sets, for each
[protected action](./01-election_management_election-event_signatures.md#protected-actions),
whether it needs signatures and how many. Reading it needs `signing-rules-read`; editing
needs `signing-rules-write`.

## The list

The actions are grouped as **Voting**, **Results and reports**, **Enrollment**, and
**Configuration and keys**. Each row shows:

| Column | Shows |
|---|---|
| Action | The action's name. An organization can rename it with its translation overrides. |
| Applies to | What one request covers: each Post, each Post and country, the voter's Post, the election event, or each trustee. |
| Who can sign | The roles that hold the action's sign permission, as chips. |
| Signatures needed | The number of signatures, **Each trustee** for the trustees' key steps, or **Off**. Every signature is made with a certificate, so the number of signatures is also the number of certificates. |
| Request expires | How long a request waits for its signatures: 30 minutes, 1 hour, 2 hours, 24 hours or **No limit**. |
| Waiting | How many requests of the action are waiting for signatures. |

A pencil opens the rule for editing; without `signing-rules-write` an eye opens it read
only. The footer names the configuration version the rules belong to and when, and by
whom, they last changed.

## Editing an action

The drawer shows the action, where it is started and what it does, then:

- **Needs signatures**: on or off. Off, the action runs as it does without signatures.
- **Who can sign**: the roles that hold the permission "Sign: *action*". See
  [Who can sign](#who-can-sign).
- **Signatures needed**: at least 1. The helper says how many people every Post has who
  can sign.
- **The person who starts it can also sign**: when unchecked, the person who started a
  request can't count as one of its signers.
- **A request expires after**: one of the expiry options. An expired request can't be
  signed and its signatures no longer count.

For the trustees' key steps (**Confirm a key share** and **Contribute a key share**) the
drawer only switches **Trustees sign this step** on or off: each trustee signs their own
step, and the key ceremony sets how many trustees take part.

**Save** stays disabled until something changes. Saving stores the rule as a new revision
and logs `SigningRuleChanged` with the old and new values.

### Waiting requests are cancelled

A rule change applies to new requests only. When the action has waiting requests, the
drawer warns before saving: saving cancels them (reason "The action's signing rule
changed"), their signatures no longer count, and the people who started them start again.

### Checks while editing

The number is checked against the people who can actually sign:

- More signatures than the Post with most signers can give is refused: "No Post has *n*
  people who can sign. The most is *max*." For an event-level action, the limit is the
  number of people who can sign it.
- Posts that can't reach the number are named, with how many signers they have: add a
  signer there or lower the number. The rule can still be saved; requests at those Posts
  can't complete until someone is added.
- When the person who starts a request can't sign, the check counts one signer less.

## Who can sign

Who can sign an action is a permission on roles: each action has a `sign-<action>`
permission, the same one **Users and Roles** shows as "Sign: *action*". It applies to the
role in every election event of the tenant. A signer also needs access to the request's
Post: staff whose roles carry permission labels sign only for the Posts those labels
allow; staff without labels can sign for any Post.

In the drawer, someone who also holds `role-write` can add or remove roles; the change is
checked against the Posts' capacity before it is applied, and logged as
`SigningPermissionChanged` in every election event that has a rule for that action.
Without `role-write`, the roles are shown read only. So one role can set the numbers while
another decides who signs. See
[Signature permissions](./06-election_management_election-event_signatures_permissions.md).

The signers list in a request shows each person's **title** (for example "Chairperson"),
from the user's `title` attribute, or the name of the group that gives them the
permission.

## Example

An organization wants two of the three board members of each Post to open and close
voting, and all three to sign the election returns:

1. In **Users and Roles**, give the board members' role `sign-open-voting`,
   `sign-close-voting` and `sign-generate-election-returns`, and give each member the
   permission label of their Post.
2. Edit **Open voting**: switch **Needs signatures** on, set **Signatures needed** to 2,
   keep **The person who starts it can also sign**, and choose an expiry such as 30
   minutes. Do the same for **Close voting**.
3. Edit **Generate election returns** with 3 signatures. If a Post has only two members
   with the permission, the drawer names it.
4. Make sure the [Certificates](./03-election_management_election-event_signatures_certificates.md)
   sub-tab trusts the issuer of the members' certificates.

## Locked-down events

The rules are part of the election event's configuration version. While the event is
locked down, the sub-tab says so and the rules can't be edited.
