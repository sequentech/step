---
id: board_roles
title: Election Board Roles and Permissions
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The TOE grants functions through roles. A role is a set of permissions; the
admin portal lists the roles under Users and Roles > "Roles" and the accounts
under Users and Roles > "Users". The administrators create the roles and the
accounts as the Handbook for Election Administrators describes. The election
board checks them before the preparation phase ends and after every change.

## Administrator and election board are separate

- The **administrator** role configures the TOE and the settings of the
  election events, manages users, roles, signing rules and background audit policy,
  and operates the platform. It holds none of the election board permissions
  listed below. The administrators see an election event's settings, never
  its election data or its audit records; only the election board reads,
  audits and exports the audit records.
- The **election board** role is a role that the administrators create, for
  example named "Election board". It holds the permissions in the next section.
  Each board member has one personal account with this role, linked to the
  external authentication server.
- **Trustees** are board members whose account also has the role `trustee`.
- No account holds both the administrator role and the election board role.

:::warning
An account that holds both roles can change the signing rules and the roles
that protect the security-critical actions, and then perform those actions.
The required authorisations would no longer prevent one person from acting
alone. Check the role assignments in Users and Roles before the preparation
phase ends, and again whenever someone joins or leaves.
:::

## Permissions of the election board

Each row names the function, the permissions it needs (label in Users and
Roles, then the technical name) and what to watch.

| Function | Permissions | What to watch |
|---|---|---|
| Open the election event and its tabs | "Read Election Event" (`election-event-read`), "View Election Event Data" (`election-event-data-tab`), "View Election Data" (`election-data-tab`), "View Election Event Keys" (`election-event-keys-tab`), "View Election Event Tally" (`election-event-tally-tab`), "View Election Event Publish" (`election-event-publish-tab`), "View Election Publish" (`election-publish-tab`), "View Election Event Logs" (`election-event-logs-tab`), "View Election Event Tasks" (`election-event-tasks-tab`), "Read Tasks Execution" (`tasks-read`), "Election Event Signatures Tab" (`election-event-signatures-tab`), "Signatures: read protected actions" (`signing-rules-read`), "Signatures: read certificates" (`signing-certificates-read`), "Read Publish" (`publish-read`), "Read Keys" (`keys-read`), "View Election Dashboard" (`election-dashboard-tab`), "View Monitoring Dashboards" (`monitoring-view`) | Read access only. The Dashboard shows the board the election execution data (turnout, cast votes, ballot boxes); "Attack detections" on the Monitoring tab shows the attacks the TOE detected. |
| Read the areas | "Read Area" (`area-read`) | Areas are election data: read only; the TOE refuses changes. |
| Read the voters' register | "Read Voter" (`voter-read`), "View Election Event Voters" (`election-event-voters-tab`) | Read only. In the post-processing phase the read is a security-critical action and needs the required authorisations (see [Security-critical actions](05-electoral-phases.md#security-critical-actions)). |
| Check the accounts and their roles | "View users and roles" (`users-menu`), "Read User" (`user-read`), "Read Role" (`role-read`), "Read User Permission" (`user-permission-read`) | Read access only; needed for the checks of the role assignments in this page. |
| Approve the configuration packages of the election data before a board member imports them | No permission in the TOE: the board member's own approval certificate from the election organizer's certification authority, used in the election organizer's package tool outside the TOE | Each approval by a different board member, after comparing the approval code aloud; at least the number the tenant requires (at least two); the importing member may be one of them. See [Approving a configuration package](06-preparation.md#approving-a-configuration-package). |
| Import the election data | "Create Election Event" (`election-event-create`) | Only as a signed configuration package with "Import Election Event"; the import needs no other permission. See [Importing the package](06-preparation.md#importing-the-package). |
| Publish configuration versions | "View Election Event Publish" (`election-event-publish-tab`), "View Election Publish" (`election-publish-tab`), "Read Publish" (`publish-read`), "Edit Publish" (`publish-write`), "Create Publish" (`publish-create`), "Regenerate Publish" (`publish-regenerate`), "Publish Changes" (`publish-changes`) | Preparation phase only; each configuration version needs the board's approval. See [Approving the configuration](06-preparation.md#approving-the-configuration). |
| Set the lockdown period | "View Election Event Scheduled" (`election-event-scheduled-tab`), "Election Event Scheduled Event Columns" (`election-event-scheduled-event-columns`), "Edit Scheduled Events" (`scheduled-event-write`), "Create Scheduled Event" (`scheduled-event-create`), "Delete Scheduled Event" (`scheduled-event-delete`) | Only "Start Lockdown Period" and "End Lockdown Period"; no other scheduled event. See [Setting the lockdown period](06-preparation.md#setting-the-lockdown-period). |
| Authorise the security-critical actions | "Sign: initialize voting" (`sign-initialize-voting`), "Sign: open voting" (`sign-open-voting`), "Sign: close voting" (`sign-close-voting`), "Sign: generate election returns" (`sign-generate-election-returns`), "Sign: generate other election reports" (`sign-generate-reports`), "Sign: transmit results" (`sign-transmit-results`), "Sign: approve a voter manually" (`sign-approve-voter`), "Sign: approve a configuration version" (`sign-approve-configuration`) | Whoever holds a "Sign: …" permission counts as an authorising board member. Give it only to board members. |
| Follow, abort and export signing requests | "Signatures: read requests" (`signing-requests-read`), "Signatures: cancel requests" (`signing-requests-cancel`), "Signatures: export requests" (`signing-requests-export`) | Every board member holds them, so that any board member can abort a security-critical action. |
| Start, interrupt, resume and terminate the election execution | "Edit Election State" (`election-state-write`), "Start Voting" (`publish-start-voting`), "Pause Voting" (`publish-pause-voting`), "Stop Voting" (`publish-stop-voting`) | Where the action is security-critical it also needs the required authorisations, each given after a fresh login at the external authentication server (see [Confirming an action by authenticating again](04-secure-use.md#confirming-an-action-by-authenticating-again)). |
| Run the key ceremony, the initialization report and the counting | "View Election Event Keys" (`election-event-keys-tab`), "Read Keys" (`keys-read`), "Admin Ceremony" (`admin-ceremony`), "Create Ceremony" (`create-ceremony`), "Export Ceremony" (`export-ceremony`), "Transmission Ceremony" (`transmition-ceremony`), "View Election Event Tally" (`election-event-tally-tab`), "Read Tally" (`tally-read`), "Start Tally" (`tally-start`), "Edit Tally" (`tally-write`), "Read Tally Results" (`tally-results-read`) | Ceremonies only as manual ceremonies (see [The key ceremony](06-preparation.md#the-key-ceremony)). |
| Check the audit records | "View Election Event Logs" (`election-event-logs-tab`), "Read Logs" (`logs-read`), "Audit Electoral Log" (`electoral-log-audit`), "Browse Electoral Log" (`electoral-log-console-read`), "Query Electoral Log" (`electoral-log-console-query`); "Read Electoral Log Personal Data" (`electoral-log-personal-data-read`) only if the election organizer decides that the board needs it | Only the election board reads and audits the audit records; needed at the beginning of the evaluation phase. |
| Export the electoral log | "Export Logs" (`logs-export`) and "Edit Reports" (`report-write`) | The export holds personal data of voters; keep it confidential. |
| Audit the electoral log | `electoral-log-audit`, `tasks-read` | Board only. The complete self-test suite and its permissions are not implemented. |
| Export election execution data | "Read Election Event" (`election-event-read`), "Export Voter" (`voter-export`); "Read Document Passwords" (`document-password-read`) and "Reveal Secret Voter Fields" (`voter-secret-attribute-read`) only if the election organizer decides that the board needs them | Post-processing phase only, with the required authorisations. |
| Delete an election event and its election execution data | "Delete Election Event" (`election-event-delete`) | Only the election board holds it. The TOE refuses every deletion unless the election execution data has been exported and the required authorisations are reached. The deletion also deletes the election event's audit records in the TOE. |
| Archive an election event | "Archive Election Event" (`election-event-archive`) | Only the election board holds it. Used in the preparation phase for an election event imported with wrong data. Archiving deletes nothing: the data and audit records stay. An archived election event never opens voting and is never unarchived. |
| Trustees only: take part in the key ceremony and the counting | role `trustee` with "Trustee Ceremony" (`trustee-ceremony`); "Sign: confirm custody" (`sign-key-ceremony`) and "Sign: authorize tally custody" (`sign-tally-key`) where those protected actions need signatures | See [Trustee Procedures](09-trustee-procedures.md). |

The board role holds no permission to create or change users, roles,
elections, contests, candidates, areas, voters, signing rules, trusted
issuers, certificate checks or background audit policy. The administrator role
holds none of the board's permissions on election data and audit records
(the Handbook for Election Administrators lists them). The administrator
role keeps, and the election board role never holds:
"Signatures: edit protected actions" (`signing-rules-write`),
"Signatures: import and remove trusted issuers" (`signing-issuers-write`),
"Signatures: register certificates" (`signing-certificates-register`), "Signatures: revoke
certificates" (`signing-certificates-revoke`), "Signatures: edit certificate
checks" (`signing-checks-write`), the user and role permissions ("Create User"
`user-create`, "Edit User" `user-write`, "Create Role" `role-create`, "Edit
Role" `role-write`, "Assign Role" `role-assign`, "Edit User Permission"
`user-permission-write`),
the attack detection rates (`attack-detection-write`) and "Edit Tenant"
(`tenant-write`).

No role holds "Edit Results Publication" (`publish-results-write`), neither
the administrator role nor the election board role. The results website is
never enabled: the import refuses a package that enables it.

:::warning
Whoever can edit roles or role assignments can give any account the election
board's permissions. If the board finds a board permission on an account that
is not a board member's personal account, or an administrator account with a
board permission, it stops all security-critical actions, cancels open signing
requests and treats the finding as a security-relevant event (see
[Security-Relevant Events for the Election Board](11-security-relevant-events.md)).
:::

## Trustees

- A trustee is a board member who holds one share of the election key. The
  trustee's account has the role `trustee` and, in the user field "Act as
  Trustee", the trustee's name as registered in Settings > "TRUSTEES".
- The TOE takes the trustee name from the account that is signed in. A trustee
  can therefore act only for their own share.
- Each trustee's share is used by that trustee's instance of the trustee
  server (`braid`), a part of the TOE that runs on the protected server: one instance
  per trustee. Each instance signs in with its own machine account (named
  `trustee1` and `trustee2`). These machine accounts belong to the
  trustee server instances: no person signs in with them, and they are not
  personal accounts of board members. Their credentials are set when the TOE
  is deployed, as the Deployment Guide describes.

:::warning
Never sign in with a machine account of a trustee server instance, and never
give its credentials to anyone. Whoever uses it acts as that trustee towards the
platform.
:::

## When a person joins, changes role or leaves

### A person joins the election board

1. The election organizer appoints the person and gives the administrators
   their identity at the external authentication server.
2. The administrators create the personal account, link it to the external
   authentication server and assign the election board role (and the role
   `trustee` and "Act as Trustee" for a trustee).
3. The election organizer issues the person's signing certificate on a security
   token and hands it over in person.
4. The administrators register the certificate to the person on the
   election event's Signatures tab under "Certificates" ("Register a
   certificate"). In the evaluated configuration a certificate signs only
   after this registration ("Registering a certificate to a person" is "Only
   when someone who can register certificates registers it").
5. The new member signs in once and checks that the admin portal shows the
   functions of the board. At the first signature the certificate checks must
   show "Registered to you".

### A person leaves, loses the role or loses their security token

Do this at once, and steps 2 and 3 on the same day at the latest, also during
the execution phase. It applies also when a board member changes function:

1. Tell the administrators.
2. The administrators disable the account (clear "Enabled") and remove all its
   roles. Disabling the account stops new sign-ins with it. Ask the person to
   log out; a session also ends after 15 minutes without interaction. The
   account itself is deleted only after the retention period of the election
   records, so that the records still name the person.
3. On the Signatures tab under "Certificates", the administrators revoke the
   registration of the person's signing certificate with "Revoke" and enter
   the "Reason". A registration is never deleted. A revoked certificate cannot sign any more;
   signatures it already made still count. Signing requests it signed that
   are not complete are cancelled ("A certificate that signed it was
   revoked"); start them again. The election organizer also revokes the
   person's approval certificate at its certification authority and adds the
   revocation list to its next configuration package; from then on the TOE
   counts no approval made with that certificate.
4. Check on the Signatures tab under "Protected actions" that every protected
   action can still reach its "Signatures needed". If it cannot, the election
   organizer appoints a replacement before the next security-critical action.
5. Check the Logs tab for actions of that account after the time it should
   have stopped acting.

### A trustee leaves

The trustee's share stays valid after the person leaves. Before the end of the
post-processing phase:

1. Follow the steps above for the account.
2. The trustee hands over the trustee workstation's keys file, all its
   backups and the saved custody request to the election board, which keeps
   them locked away under the control of at least two board members until the
   end of the post-processing phase.
3. Both trustees are needed to count the votes. The election organizer
   appoints a board member who is not a trustee as the replacement. The
   administrators give that person's account the role `trustee` and, in "Act
   as Trustee", the name of the trustee who left. For the counting, two board
   members hand the keys file and the saved custody request to the
   replacement, who authorises the counting with them on a trustee
   workstation as
   [Authorising the counting](09-trustee-procedures.md#authorising-the-counting)
   describes, and then returns them to the board. Nobody ever holds the keys
   files of two trustees.

:::warning
A trustee who keeps their keys file after leaving can still help to decrypt
votes. Treat a trustee who leaves, or a keys file that is lost or exposed, as
a security-relevant event.
:::
