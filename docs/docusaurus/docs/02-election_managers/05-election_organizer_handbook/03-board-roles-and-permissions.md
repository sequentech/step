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
under Users and Roles > "Users". A fresh installation provides the <code>election-board</code> role.
The administrators assign it to personal board accounts as the Handbook for
Election Administrators describes. The election
board checks them before the preparation phase ends and after every change.

## Administrator and election board are separate

- The **administrator** role configures the TOE and the settings of the
  election events, manages users, roles, signing rules and background audit policy,
  and operates the platform. It does not authorise board actions or grant access to election execution data. The administrators see an election event's settings, never
  its election data or its audit records; only the election board reads,
  audits and exports the audit records.
- The **election board** role is the supplied <code>election-board</code> role.
  It holds the permissions in the next section; use this role for a fresh installation.
  Each board member has one personal account with this role, linked to the
  external authentication server.
- **Trustees** are board members whose account also has the role `trustee`.
- No account holds both the administrator role and the election board role.

The token issuer removes election-board and election-data permissions from an
account that has an administrator capability. Giving an administrator a second
board role therefore does not enable board actions. Use separate personal
accounts for the two responsibilities; an administrator cannot change their
own role membership or edit or delete a group that supplies their own permissions.
Another administrator must make those changes.

Before issuing a board account, check that none of its roles, including inherited
roles, grants:

<ul>
<li><code>admin-user</code>, <code>tenant-write</code> or <code>self-test-settings-write</code>;</li>
<li><code>user-create</code>, <code>user-write</code>, <code>role-create</code> or <code>role-write</code>;</li>
<li><code>signing-rules-write</code>, <code>signing-issuers-write</code> or <code>signing-checks-write</code>;</li>
<li><code>signing-certificates-register</code> or <code>signing-certificates-revoke</code>.</li>
</ul>

If board functions are absent, ask an independent administrator to correct the
account and sign in again. Do not remove permissions from your own role to try
to change your classification.

## Permissions of the election board

Each row names the function, the permissions it needs (label in Users and
Roles, then the technical name) and what to watch.

| Function | Permissions | What to watch |
|---|---|---|
| Open the election event and its tabs: data and monitoring | <ul><li>"Read Election Event" (<code>election-event-read</code>)</li><li>"View Election Event Data" (<code>election-event-data-tab</code>)</li><li>"View Election Data" (<code>election-data-tab</code>)</li><li>"Admin Dashboard View" (<code>admin-dashboard-view</code>) for the election-event Dashboard</li><li>"View Election Dashboard" (<code>election-dashboard-tab</code>)</li><li>"View Monitoring Dashboards" (<code>monitoring-view</code>)</li></ul> | Read access only. The Dashboard shows the board the election execution data (turnout, cast votes, ballot boxes); "Attack detections" on the Dashboard shows the attacks the TOE detected. |
| Open the election event and its tabs: keys, tally and publication | <ul><li>"View Election Event Keys" (<code>election-event-keys-tab</code>)</li><li>"View Election Event Tally" (<code>election-event-tally-tab</code>)</li><li>"View Election Event Publish" (<code>election-event-publish-tab</code>)</li><li>"View Election Publish" (<code>election-publish-tab</code>)</li><li>"Read Publish" (<code>publish-read</code>)</li><li>"Read Keys" (<code>keys-read</code>)</li></ul> | Read access only. |
| Open the election event and its tabs: logs, tasks and signatures | <ul><li>"View Election Event Logs" (<code>election-event-logs-tab</code>)</li><li>"View Election Event Tasks" (<code>election-event-tasks-tab</code>)</li><li>"Read Tasks Execution" (<code>tasks-read</code>)</li><li>"Election Event Signatures Tab" (<code>election-event-signatures-tab</code>)</li><li>"Signatures: read protected actions" (<code>signing-rules-read</code>)</li><li>"Signatures: read certificates" (<code>signing-certificates-read</code>)</li></ul> | Read access only. |
| Read the areas | <ul><li>"Read Area" (<code>area-read</code>)</li></ul> | Areas are election data: read only; the TOE refuses changes. |
| Read the voters' register | <ul><li>"Read Voter" (<code>voter-read</code>)</li><li>"View Election Event Voters" (<code>election-event-voters-tab</code>)</li></ul> | Read only. In the post-processing phase the read is a security-critical action and needs the required authorisations (see [Security-critical actions](05-electoral-phases.md#security-critical-actions)). |
| Check the accounts and their roles | <ul><li>"View users and roles" (<code>users-menu</code>)</li><li>"Read User" (<code>user-read</code>)</li><li>"Read Role" (<code>role-read</code>)</li><li>"Read User Permission" (<code>user-permission-read</code>)</li></ul> | Read access only; needed for the checks of the role assignments in this page. |
| Approve the configuration packages of the election data before a board member imports them | No permission in the TOE: the board member's own approval certificate from the election organizer's certification authority, used in the election organizer's package tool outside the TOE | Each approval by a different board member, after comparing the approval code aloud; at least the number the tenant requires (at least two); the importing member may be one of them. See [Approving a configuration package](06-preparation.md#approving-a-configuration-package). |
| Import the election data | <ul><li>"Create Election Event" (<code>election-event-create</code>)</li></ul> | Use a signed configuration package and the mandatory "Import election data" signing rule in a Preparation event; the operation waits for fresh board authorisations before importing. See [Importing the package](06-preparation.md#importing-the-package). |
| Publish configuration versions | <ul><li>"View Election Event Publish" (<code>election-event-publish-tab</code>)</li><li>"View Election Publish" (<code>election-publish-tab</code>)</li><li>"Read Publish" (<code>publish-read</code>)</li><li>"Edit Publish" (<code>publish-write</code>)</li><li>"Create Publish" (<code>publish-create</code>)</li><li>"Regenerate Publish" (<code>publish-regenerate</code>)</li><li>"Publish Changes" (<code>publish-changes</code>)</li></ul> | Preparation phase only; each configuration version needs the board's approval. See [Approving the configuration](06-preparation.md#approving-the-configuration). |
| Set the lockdown period | <ul><li>"View Election Event Scheduled" (<code>election-event-scheduled-tab</code>)</li><li>"Election Event Scheduled Event Columns" (<code>election-event-scheduled-event-columns</code>)</li><li>"Edit Scheduled Events" (<code>scheduled-event-write</code>)</li><li>"Create Scheduled Event" (<code>scheduled-event-create</code>)</li><li>"Delete Scheduled Event" (<code>scheduled-event-delete</code>)</li></ul> | Only "Start Lockdown Period" and "End Lockdown Period"; no other scheduled event. See [Setting the lockdown period](06-preparation.md#setting-the-lockdown-period). |
| Authorise the security-critical actions | <ul><li>"Sign: initialize voting" (<code>sign-initialize-voting</code>)</li><li>"Sign: open voting" (<code>sign-open-voting</code>)</li><li>"Sign: close voting" (<code>sign-close-voting</code>)</li><li>"Sign: generate election returns" (<code>sign-generate-election-returns</code>)</li><li>"Sign: generate other election reports" (<code>sign-generate-reports</code>)</li><li>"Sign: transmit results" (<code>sign-transmit-results</code>)</li><li>"Sign: approve a voter manually" (<code>sign-approve-voter</code>)</li><li>"Sign: approve a configuration version" (<code>sign-approve-configuration</code>)</li><li><code>sign-import-election-data</code></li><li><code>sign-start-tally</code></li><li><code>sign-export-execution-data</code></li><li><code>sign-delete-execution-data</code></li><li><code>sign-read-voters-register</code></li></ul> | Whoever holds a "Sign: …" permission counts as an authorising board member. Give it only to board members. |
| Follow, abort and export signing requests | <ul><li>"Signatures: read requests" (<code>signing-requests-read</code>)</li><li>"Signatures: cancel requests" (<code>signing-requests-cancel</code>)</li><li>"Signatures: export requests" (<code>signing-requests-export</code>)</li></ul> | Every board member holds them, so that any board member can abort a security-critical action. |
| Start, interrupt, resume and terminate the election execution | <ul><li>"Edit Election State" (<code>election-state-write</code>)</li><li>"Start Voting" (<code>publish-start-voting</code>)</li><li>"Pause Voting" (<code>publish-pause-voting</code>)</li><li>"Stop Voting" (<code>publish-stop-voting</code>)</li></ul> | Where the action is security-critical it also needs the required authorisations, each given after a fresh login at the external authentication server (see [Confirming an action by authenticating again](04-secure-use.md#confirming-an-action-by-authenticating-again)). |
| Run the key ceremony, the initialization report and the counting: ceremony permissions | <ul><li>"View Election Event Keys" (<code>election-event-keys-tab</code>)</li><li>"Read Keys" (<code>keys-read</code>)</li><li>"Admin Ceremony" (<code>admin-ceremony</code>)</li><li>"Create Ceremony" (<code>create-ceremony</code>)</li><li>"Export Ceremony" (<code>export-ceremony</code>)</li><li>"Transmission Ceremony" (<code>transmition-ceremony</code>)</li></ul> | Ceremonies only as manual ceremonies (see [The key ceremony](06-preparation.md#the-key-ceremony)). |
| Run the key ceremony, the initialization report and the counting: tally permissions | <ul><li>"View Election Event Tally" (<code>election-event-tally-tab</code>)</li><li>"Read Tally" (<code>tally-read</code>)</li><li>"Start Tally" (<code>tally-start</code>)</li><li>"Edit Tally" (<code>tally-write</code>)</li><li>"Read Tally Results" (<code>tally-results-read</code>)</li></ul> | Ceremonies only as manual ceremonies (see [The key ceremony](06-preparation.md#the-key-ceremony)). |
| Check the audit records | <ul><li>"View Election Event Logs" (<code>election-event-logs-tab</code>)</li><li>"Read Logs" (<code>logs-read</code>)</li><li>"Audit Electoral Log" (<code>electoral-log-audit</code>)</li><li>"Browse Electoral Log" (<code>electoral-log-console-read</code>)</li><li>"Query Electoral Log" (<code>electoral-log-console-query</code>)</li><li>"Read Electoral Log Personal Data" (<code>electoral-log-personal-data-read</code>) — only if the election organizer decides that the board needs it.</li></ul> | Only the election board reads and audits the audit records; needed at the beginning of the evaluation phase. |
| Export the electoral log | <ul><li>"Export Logs" (<code>logs-export</code>)</li><li>"Edit Reports" (<code>report-write</code>)</li></ul> | The export holds personal data of voters; keep it confidential. |
| Read and request security self-tests | <ul><li><code>self-test-read</code></li><li><code>self-test-run</code></li></ul> | **Security and self-tests** shows six results and the current phase. Changing test settings belongs to the administrator during preparation. |
| Export election execution data | <ul><li><code>election-execution-export</code></li><li>"Read Election Event" (<code>election-event-read</code>)</li><li>"Export Voter" (<code>voter-export</code>)</li><li>"Read Document Passwords" (<code>document-password-read</code>) — only if the election organizer decides that the board needs it.</li><li>"Reveal Secret Voter Fields" (<code>voter-secret-attribute-read</code>) — only if the election organizer decides that the board needs it.</li></ul> | Post-processing phase only, with the required authorisations. |
| Delete an election event and its election execution data | <ul><li>"Delete Election Event" (<code>election-event-delete</code>)</li></ul> | Only the election board holds it. The TOE refuses every deletion unless the election execution data has been exported and the required authorisations are reached. The deletion also deletes the election event's audit records in the TOE. |
| Archive an election event | <ul><li>"Archive Election Event" (<code>election-event-archive</code>)</li></ul> | Only the election board holds it. Used in the preparation phase for an election event imported with wrong data. Archiving deletes nothing: the data and audit records stay. An archived election event never opens voting and is never unarchived. |
| Trustees only: take part in the key ceremony and the counting | Role <code>trustee</code>, with these permissions: <ul><li>"Trustee Ceremony" (<code>trustee-ceremony</code>)</li><li>"Sign: confirm custody" (<code>sign-key-ceremony</code>) — where the protected action needs signatures.</li><li>"Sign: authorize tally custody" (<code>sign-tally-key</code>) — where the protected action needs signatures.</li></ul> | See [Trustee Procedures](09-trustee-procedures.md). |

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
the attack detection rates (`tenant-write`) and "Edit Tenant"
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
