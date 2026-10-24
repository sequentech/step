---
id: signing_rules
title: Configuring Protected Actions
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## What protected actions are

A security-critical action runs only after the required number of election
board members has authorised it. Each authorisation is a signature that a board
member makes with the signing certificate on their security token. The admin
portal calls a security-critical action a **protected action**, and its rule a
**signing rule**: who can sign, how many signatures are needed, and when a
request expires.

You set the signing rules, the trusted issuers and the certificate checks of
each election event on its tab "Signatures". The tab explains: "Protected
actions run only after enough authorized people sign them with their digital
certificates. Each signature is checked against the trusted issuers and
recorded in the log." It has three areas: "Protected actions",
"Certificates" and "Requests". "Requests" belongs to the election board: a
request shows what it authorises, for a configuration version its "Changes in
this version", which are election data. The administrator role holds no
permission for it.

Signing rules, trusted issuers, certificate checks and registered certificates
belong to one election event. Set them for every new election event, also for
one that replaces a wrong import.

## Permissions

| Function | Permissions | Held by |
|---|---|---|
| Open the tab and read the rules and certificates | "Election Event Signatures Tab" (`election-event-signatures-tab`), "Signatures: read protected actions" (`signing-rules-read`), "Signatures: read certificates" (`signing-certificates-read`) | administrator role and "Election board" |
| Change signing rules | "Signatures: edit protected actions" (`signing-rules-write`); to change "Who can sign" also "Read Role" (`role-read`) and "Edit Role" (`role-write`) | administrator role only |
| Import and remove trusted issuers | "Signatures: import and remove trusted issuers" (`signing-issuers-write`) | administrator role only |
| Change the certificate checks | "Signatures: edit certificate checks" (`signing-checks-write`) | administrator role only |
| Register certificates to people | "Signatures: register certificates" (`signing-certificates-register`) | administrator role only |
| Revoke certificates | "Signatures: revoke certificates" (`signing-certificates-revoke`) | administrator role only |
| Read, cancel and export signing requests | "Signatures: read requests" (`signing-requests-read`), "Signatures: cancel requests" (`signing-requests-cancel`), "Signatures: export requests" (`signing-requests-export`) | "Election board" only; give `signing-requests-cancel` to every board member |

Without "Signatures: edit protected actions", the rule shows "Read only.
Changing signing rules needs the permission “Signatures: edit protected
actions”."

## Security-critical actions that are always protected

The TOE always requires the election board's authorisations for these actions;
they cannot be switched off:

| Security-critical action | Electoral phase |
|---|---|
| import of the ballot (the candidate list and its display data) | preparation |
| import of the voters' register | preparation |
| import of the set of election rules | preparation |
| import of the election dates | preparation |
| termination of the election execution ("Close voting") | execution |
| resumption of the election execution ("Open voting") | execution |
| start of the counting of the votes with determination of the election result | evaluation |
| export of election execution data | post-processing |
| deletion of election execution data | post-processing |
| the election board's read of the voters' register | post-processing |

On the tab, termination is the protected action "Close voting", and
resumption is "Open voting", which also covers the first opening of voting.
The four imports are performed together, by one import of a signed
configuration package, which a member of the election board performs. The
election event does not exist before its import, so no rule of this tab applies
to the imports: their required authorisations are the approvals of board
members inside the package, at least the number that the tenant's signed
configuration packages require
([Tenant and Election Event Settings](05-tenant_and_event_settings.md#signed-configuration-packages)).

A protected action whose "Signatures needed" has not been set cannot be
performed: the TOE refuses it. Set "Signatures needed" for every protected
action that the tab lists, before the election board starts any of them.

## Protected actions to switch on

The catalogue of the tab groups the protected actions in "Voting", "Results
and reports", "Enrollment" and "Configuration and keys". For each row the
column "Applies to" shows where the rule applies. Switch on these protected
actions as well, so that every step the election board takes towards the
execution phase needs its authorisations:

| Protected action | Applies to | Required setting | Why |
|---|---|---|---|
| "Approve a configuration version" | The election event | "Needs signatures" on | No configuration version, with its settings and signing rules, becomes valid without the board's approval. Set it before the first "Publish Changes". |
| "Initialize voting" | Each Post | "Needs signatures" on | The initialization report, in which witnesses see that the ballot boxes are empty, is authorised by the board. |
| "Generate other election reports" | Each Post | "Needs signatures" on | Covers the initialization report. |
| "Confirm custody (key ceremony)" | Each trustee | "Trustees sign this step" on | Each trustee confirms custody of their key share with their own certificate. |
| "Authorize tally custody (tally)" | Each trustee | "Trustees sign this step" on | The tally ceremony starts only with the trustees' authorisations. |

The trustee rows need "Manual Ceremonies"
([Tenant and Election Event Settings](05-tenant_and_event_settings.md)): with
automatic ceremonies the TOE refuses the rule with "This event uses automatic
key ceremonies. Trustees do not perform these steps, so their signatures cannot
be required. To require trustee signatures, use manual key ceremonies."

The remaining actions of the catalogue, "Generate election returns", "Transmit
results" and "Approve a voter manually", are not used in the evaluated
configuration: voters do not enrol, and results are not transmitted. Switch on
"Generate election returns" if the election organizer wants signed election
returns.

## Setting a signing rule

You can change signing rules only in the preparation phase, before the
election board approves the configuration version.

1. Open the election event, tab "Signatures", area "Protected actions". The
   table shows "Action", "Applies to", "Who can sign", "Signatures needed",
   "Request expires" and "Waiting". A rule that is not switched on shows
   "Off".
2. Select the edit icon of the action. The drawer opens.
3. Set the fields:

| Field | Required value |
|---|---|
| "Needs signatures", or "Trustees sign this step" for the trustee rows | on |
| "Who can sign" | only "Election board". Never the administrator role. Its help text says: "These roles get the permission “Sign: …” in Users and Roles, for every election event. Signers must also have access to the Post." |
| "Signatures needed" | the number the electoral regulation fixes, at least 2 |
| "The person who starts it can also sign" | off, unless the electoral regulation counts the person who starts the action as one of the authorising members |
| "A request expires after" | "30 minutes", "1 hour", "2 hours" or "24 hours", long enough for the board members to sign; never "No limit" |

4. Select "Save". The TOE confirms "The signing rule was saved."

Messages of the drawer:

| Message | Meaning |
|---|---|
| "At least 1." / "At most …" | The number of signatures is outside the allowed range. |
| "No Post has … people who can sign. The most is …" | Fewer people can sign than the number you entered. Give the role to more board members, or ask the election organizer. |
| "N requests are waiting for signatures under the current rule. Saving cancels them; the people who started them start again." | Saving a changed rule cancels its waiting requests. |
| "The election event is locked down: signing rules change only through a new configuration version." | The election event is in a lockdown period; the TOE refuses the change. |
| "You don't have the permission for this change." | Your account lacks a permission of the table above. |
| "Someone else changed this meanwhile. Reload the page and try again." | Reload and check the rule again before you change it. |
| "The server refused these values. Check them and try again." | A value is outside what the TOE accepts. |

:::warning
Never put the administrator role, or any role that an administrator holds,
into "Who can sign", and never set "Signatures needed" to 1. A single person
could then perform a security-critical action alone. A rule change becomes part
of the next configuration version, which the election board compares with the
electoral regulation before it approves it.
:::

What the TOE does with a rule:

- Each change is recorded in the election event's electoral log; a change of
  "Who can sign" also changes the "Sign:" permissions of the roles and is
  recorded as well.
- The footer of the drawer says: "Changes are recorded in the election event's
  log and become part of the next configuration version." Before the first
  publication it says "Signing rules become part of this event's first
  configuration version when it is published."
- After the approval of a configuration version, a rule changes only through a
  new configuration version, which the board approves again.
- The TOE requires mutually distinct board members, certificates and keys for
  the signatures of one request.

## Trusted issuers and certificate checks

Open the tab "Signatures", area "Certificates".

### Trusted issuers

"Staff certificates must chain to one of these. They are separate from the
certificates voters sign in with." While the list is empty the tab shows "No
trusted issuers yet. Staff can't sign until one is imported."

1. Receive the CA certificates that issue the board members' signing
   certificates from the election organizer, with their SHA-256 fingerprints
   in writing, through a channel that protects their integrity.
2. Select "Import issuer certificates", then "Choose a certificate file", and
   "Import".
3. Compare the "SHA-256" column with the fingerprints in writing. If one
   differs, remove the issuer at once.

To remove an issuer, confirm "Remove … from the trusted issuers? Certificates
it issued can no longer sign." Remove an issuer only when the election
organizer tells you to.

### Checks

The checks are saved when you change them ("The certificate checks were
saved."):

| Check | Default | Required value |
|---|---|---|
| "Check revocation lists" ("Downloaded from each issuer every hour.") | on | on, where the election organizer's CA publishes revocation lists that the server can download |
| "When a list can't be downloaded": "Don't accept signatures" / "Accept and mark the signature as unchecked" | "Don't accept signatures" | "Don't accept signatures" |
| "Registering a certificate to a person": "When its holder first signs with it" / "Only when someone who can register certificates registers it" | "When its holder first signs with it" | "Only when someone who can register certificates registers it" |
| "A certificate signs for one Post only" | on | on |

If the election organizer's CA publishes no revocation lists that the server
can download, revocation is done only by revoking the registration in the TOE
(see below). Agree with the election organizer that it tells you at once of
every certificate it revokes.

:::warning
With "When its holder first signs with it", the TOE binds a certificate to
whoever signs with it first, without any check by a second person. With
"Accept and mark the signature as unchecked", a certificate that the issuer
has revoked can still sign while its list cannot be downloaded.
:::

### Registered certificates

With "Only when someone who can register certificates registers it", a board
member can sign only after an administrator has registered their certificate.
Only the administrator role holds "Signatures: register certificates".

1. Meet the board member in person. Check their identity and receive their
   public certificate (PEM) from them, for example on a medium they hand over.
2. Select "Register a certificate". In "Person", type part of the username
   ("Type part of a username to find the person.") and select the board
   member's account.
3. Paste the certificate into "Certificate (PEM)" and select "Register". The
   TOE confirms "The certificate was registered." It refuses with "This
   certificate can't be registered: check that a trusted issuer issued it,
   that it's valid today and that it's made for signing."
4. Check the registered certificate together with the board member.

The list shows each registration with its status: "Active", "Expires soon",
"Expired" or "Revoked". Before each security-critical action that is due, make
sure that enough registrations of each rule's signers are "Active".

To revoke a registration, select "Revoke" and enter the "Reason". "A revoked
certificate can't sign anymore. Signatures it already made still count."
Revoke a registration on the same day when a board member leaves, loses their
security token, or the election organizer revokes their certificate
([Users, Roles and Role Separation](04-users_and_roles.md)).

## Signing requests

The area "Requests" lists the signing requests of the election event with
their status: "Waiting", "Signed", "Done", "Cancelled", "Expired" or "Failed".
"Export CSV" exports the list. Only the election board uses it.

- A board member starts the request "Approve a configuration version" by
  selecting "Publish Changes"
  ([Tenant and Election Event Settings](05-tenant_and_event_settings.md#publishing-the-configuration-version)).
  You never start, sign or cancel a request.
- Every board member holds "Signatures: cancel requests"
  (`signing-requests-cancel`), so that any board member can abort a
  security-critical action they object to.
- A "Failed" request means that all signatures were given but the action
  failed. When the election board tells you of one, treat it as a
  security-relevant event
  ([Security-Relevant Events for Administrators](11-security_relevant_events.md)).

:::warning
Without "Signatures: cancel requests" in the "Election board" role, only the
member who started a request can abort it. The other members could then not
stop an action they object to.
:::
