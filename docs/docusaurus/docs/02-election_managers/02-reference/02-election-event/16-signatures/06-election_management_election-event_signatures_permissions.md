---
id: election_management_election_event_signatures_permissions
title: Signature permissions
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Signing adds 21 permissions. They are assigned to roles in **Users and Roles** > **Roles**,
where they are listed with the labels below; typing "Signatures" or "Sign:" in the grid's
search finds them. A role change takes effect at the user's next sign-in. The general rules
for these permissions, and how existing tenants get them, are in
[Permissions](../../user-manual/users-and-roles/users-and-roles_permissions.md#signature-permissions).

## The permissions

| Permission | Label | Allows |
|---|---|---|
| `election-event-signatures-tab` | Election Event Signatures Tab | Show the Signatures tab, together with at least one read permission below. |
| `signing-rules-read` | Signatures: read protected actions | See the Protected actions sub-tab and open rules read only. |
| `signing-rules-write` | Signatures: edit protected actions | Edit the rules: needs signatures, number, requester, expiry. |
| `signing-certificates-read` | Signatures: read certificates | See the Certificates sub-tab. |
| `signing-issuers-write` | Signatures: import and remove trusted issuers | Import and remove trusted issuers. |
| `signing-checks-write` | Signatures: edit certificate checks | Change the certificate checks. |
| `signing-certificates-register` | Signatures: register certificates | Register a certificate to a person, or link it to a second account of the same person. |
| `signing-certificates-revoke` | Signatures: revoke certificates | Revoke a registered certificate (and its key). |
| `signing-requests-read` | Signatures: read requests | See the Requests sub-tab. Staff with permission labels see their Posts' requests. |
| `signing-requests-cancel` | Signatures: cancel requests | Cancel someone else's waiting request. The person who started a request can always cancel it. |
| `signing-requests-export` | Signatures: export requests | Export the requests as CSV. |
| `sign-initialize-voting` | Sign: initialize voting | Sign Initialize voting. |
| `sign-open-voting` | Sign: open voting | Sign Open voting. |
| `sign-close-voting` | Sign: close voting | Sign Close voting. |
| `sign-generate-election-returns` | Sign: generate election returns | Sign the election returns. |
| `sign-generate-reports` | Sign: generate other election reports | Sign the other election reports. |
| `sign-transmit-results` | Sign: transmit results | Sign a results package. |
| `sign-approve-voter` | Sign: approve a voter manually | Sign a manual voter approval. |
| `sign-approve-configuration` | Sign: approve a configuration version | Sign a configuration version. |
| `sign-key-ceremony` | Sign: confirm a key share | Sign one's own key share in the key ceremony. |
| `sign-tally-key` | Sign: contribute a key share | Sign one's own key share contribution in the tally. |

A `sign-<action>` permission is what **Who can sign** shows for the action. Signing also
needs access to the request's Post through the user's permission labels (users without
labels reach every Post), and, for the trustees' steps, being the request's trustee.
Signers need no permission on the Signatures tab: the request panel gives them what they
need.

## How they are enforced

- The Admin Portal hides a sub-tab without its read permission, hides or disables each
  control without its write permission, and marks the sub-tab **Read only**.
- The server checks the same permission on every operation, so a direct API call without
  it is refused, and it returns no signing data to a role without the read permission.
- Every change records the person who made it in the election event's
  [Logs](../11-election_management_election-event_logs.md).

## Changing who can sign

Changing who can sign an action means turning its `sign-<action>` permission on or off for
a role, from the rule drawer or in **Users and Roles**. It takes `role-write`. With
`role-write` but without `user-permission-write`, a user can change only the `sign-<action>`
permissions; changing any other permission of a role still takes both. So whoever
administers who signs can't grant themselves anything else.

The change applies to the role in every election event of the tenant, and is logged as
`SigningPermissionChanged` in each election event that has a rule for that action. Adding a
user to a role, or removing them, is not logged there.

## Sample preset

A tenant template can assign these permissions to its own groups. The janitor's sample
template does it as below. It is a **sample, to be confirmed by the organization that uses
it**; adjust the groups and rules to the organization's own roles.

| Permission | `configuration-manager` | `security-officer` | `ofov` | `auditor` | `sbei` | `trustee` |
|---|---|---|---|---|---|---|
| `election-event-signatures-tab` | Yes | Yes | Yes | Yes | | |
| `signing-rules-read` | Yes | | | Yes | | |
| `signing-rules-write` | Yes | | | | | |
| `signing-certificates-read` | | Yes | | Yes | | |
| `signing-issuers-write`, `signing-checks-write` | | Yes | | | | |
| `signing-certificates-register`, `signing-certificates-revoke` | | Yes | | | | |
| `signing-requests-read`, `signing-requests-export` | | | Yes | Yes | | |
| `signing-requests-cancel` | | | Yes | | | |
| `role-write` (who can sign) | | Yes | | | | |
| `sign-initialize-voting`, `sign-open-voting`, `sign-close-voting` | | | | | Yes | |
| `sign-generate-election-returns`, `sign-generate-reports`, `sign-transmit-results` | | | | | Yes | |
| `sign-approve-voter` | | | Yes | | Yes | |
| `sign-approve-configuration` | Yes | Yes | | | | |
| `sign-key-ceremony`, `sign-tally-key` | | | | | | Yes |

So the Configuration Manager sees only Protected actions, the Security Officer only
Certificates, the requests operators (`ofov`) only Requests, and the Auditor all three read
only. The template's Post board members (`sbei`) sign the Post actions and voter approvals,
and its trustees their own key steps. The `admin` group holds all 21. The four settings groups also get
`election-event-read`, to reach an election event; the Security Officer also gets what
opening **Users and Roles** and finding a person take.

The sample template's rules and checks, imported with its election events:

| Action | Signatures | Expires after |
|---|---|---|
| Initialize, open and close voting | 2 of the Post's board members | 30 minutes |
| Generate election returns | 3 | 2 hours |
| Generate other election reports, transmit results | 2 | 2 hours |
| Approve a voter manually | 1 | No limit |
| Approve a configuration version | 2 | 24 hours |
| Confirm and contribute a key share | Each trustee | 1 hour |

The person who starts a request can sign it. Certificates are checked against revocation
lists (no signature without a list), registered on first use, and sign for one Post only.
Each board member has a title (Chairperson, Poll Clerk, Third Member) shown in the request
panel.
