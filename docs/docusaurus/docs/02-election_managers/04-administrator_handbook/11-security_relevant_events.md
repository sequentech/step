---
id: security_relevant_events
title: Security-Relevant Events for Administrators
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## How to handle a security-relevant event

For every event on this page:

1. Act at once, in the way the table says.
2. Tell the election board, and the election organizer where the table says
   so.
3. Write down what you saw, with the date, the time and its time zone, and what
   you did.
4. If the event may be caused by a flaw of the TOE, report it
   ([Environment Measures and Security Problems](12-environment_and_security_problems.md#reporting-a-security-problem)).

## In every phase

| Event | How you notice it | What you do |
|---|---|---|
| A person leaves or changes function | the election organizer tells you | The same day: disable the account, remove its roles, revoke the registration of its signing certificates in every election event, ask for the credentials at the external authentication server to be withdrawn, and check that each protected action still has enough signers ([Users, Roles and Role Separation](04-users_and_roles.md#when-a-person-leaves-or-changes-function)). |
| A board member's security token is lost or stolen | the board member or the board tells you | Revoke the certificate's registration at once in every election event; the election organizer revokes the certificate at its CA. |
| An account holds a board permission and is not a board member's account, or an administrator account holds a board permission | your check of Users and Roles, or the election board tells you | Remove the permission, tell the election board and the election organizer; the board checks the electoral log for actions of that account. Explain your changes from your written record. |
| An account reached the authentication failure threshold | the person cannot sign in and tells you, or the election board tells you | Find out with the person whether the attempts were theirs. If not, tell the election board and the organisation that runs the external authentication server. |
| An attack detection matched | the election board tells you | Tell the person who runs the servers and the network, and agree the measures with the board ([Audit, Monitoring and Notifications](09-audit_and_monitoring.md#attack-detections)). |
| The "Version" or an image digest differs from the TOE delivery | your check ([Secure Use of the Admin Portal](03-admin_portal.md)) | The system is not the evaluated TOE: do not start or continue an election; tell the election board and the election organizer; report it. |
| The audit storage threshold is exceeded | email | Extend the storage as the Deployment Guide describes; never delete records. Voters' access returns once space is available. |
| A server component crashed or was shut down | the state check of the Deployment Guide ("Operating the servers"); operator monitoring; automatic ST maintenance is not implemented | Have it restarted as the Deployment Guide describes, in the execution phase only when the election board has decided so, then check the version and the image digests. Ask the board to review the available log audit; a complete ST self-test is not available ([Modes of Operation and Failure Handling](10-modes_of_operation.md)). |
| The browser reports a certificate problem for the admin portal or the external authentication server | the browser | Do not sign in; tell the person who runs the servers. |
| A notification email does not arrive, or arrives for an event you did not expect | email | Check the account's "Email" and tell the person who runs the servers; ask the election board to check the electoral log for the event. |

## Preparation phase

| Event | How you notice it | What you do |
|---|---|---|
| An import of the election board is refused: the digest, the signature or the approvals are not valid | the election board tells you | Check with a backup that `configuration_signing` still has your values; do not change it. The board asks the election organizer for a new package ([Tenant and Election Event Settings](05-tenant_and_event_settings.md#import-of-election-data)). |
| A setting of an imported election event differs from the required value | your checks ([Tenant and Election Event Settings](05-tenant_and_event_settings.md)) | Stop before publication and tell the election board and the election organizer. The organizer includes the required value in a corrected signed package; the board imports a new election event and archives the wrong one. Check the new event without repairing imported values in the TOE. |
| The imported election data is wrong | the election board's check | Nobody corrects it in the TOE. The election organizer issues a corrected package, which the election board imports; the board archives the wrong election event. Check the settings of the new one. |
| The TOE refuses a change with "The election event is locked down: …" | the message | The election event is in a lockdown period: change nothing. A needed change goes through a new configuration version, with the election board. |
| An electoral-log audit fails, is missing or is delayed | The board checks **Tasks → Audit Electoral Log** | Inform the board and operator; preserve the task result and investigate. A log audit is not the complete ST suite ([Self-Test Settings](07-self_tests.md)). |
| A ballot box is not empty when the execution phase is to start | maintenance mode | Tell the election board; do not try to empty it; report it. |
| A scheduled opening or closing shows "Will be refused" or "Will run without signatures" | the election board tells you | The schedule does not match the approved configuration version. Tell the board every setting you changed after the approval. |
| A signing request ends as "Failed" | the election board tells you | Check the protected action's rule and the certificate registrations on the Signatures tab with the board. |

## Execution phase

| Event | How you notice it | What you do |
|---|---|---|
| An electoral-log audit fails, is missing or is delayed | The board checks **Tasks → Audit Electoral Log** | Inform the board and operator; preserve the task result and investigate. A log audit is not the complete ST suite ([Self-Test Settings](07-self_tests.md)). |
| The ballot box test fails: an unrecoverable failure | email | Support the election board; keep all server logs; report it ([Modes of Operation and Failure Handling](10-modes_of_operation.md#an-unrecoverable-failure)). |
| The server clock is not synchronised | AGD-04 synchronization check fails | Restore the time service as the Deployment Guide describes. |

## Evaluation and post-processing phases

| Event | How you notice it | What you do |
|---|---|---|
| The audit of the electoral log reports FAILED | the election board tells you | Support the election board; it decides under the electoral regulation. Report it. |
| The board finds a record it cannot explain | the board's audit review | Help the board to find the cause from your written record of changes. |
| A request to delete or archive an election event reaches you | anyone | Refuse: the administrator role holds neither permission. Only the election board archives an election event, or deletes one after the export with its required authorisations. |

## Checklist before the election board approves the configuration version

Work through one section at a time. Record the election event and configuration
version you checked. A tick records a completed check, not an approval: the
election board approves the configuration version after reviewing the evidence.
If a check fails, stop and resolve it before asking for approval.

### 1. Verify the delivery and imported configuration

- [ ] The header shows `Sequent Voting Platform - uniWAHL Version, v10.0.0`.
- [ ] Every running image digest matches `image-digests.txt`.
- [ ] Signed configuration packages are required, with the organizer's checked
  CA certificates and at least two approvals.
- [ ] The board confirms that a board member used **Import Election Event** and
  checked the digest received from the organizer.
- [ ] The operator confirms in writing that election work uses only the
  customer's tenant, no plugin is installed, and no telephone provider is configured.

### 2. Check the accounts and permissions

- [ ] No account combines an administrator role with a board permission.
- [ ] The administrator has none of the
  [permissions on election data and audit records](04-users_and_roles.md#permissions-on-election-data-and-audit-records).
- [ ] No role has **Edit Results Publication** (`publish-results-write`),
  and no account has **Datafix account** (`datafix-account`).
- [ ] The seeded `admin` account is deleted.
- [ ] Each board member has **Create Election Event**, **Signatures: cancel
  requests**, **Export Logs**, **Edit Reports**, and read-only user/role access.
- [ ] Each administrator and board member has a tested notification email address.

### 3. Check the election event's Data tab

Tick each row after comparing the displayed value.

| Checked | Setting | Required value |
|---|---|---|
| ☐ | Ballot Box Seal Policy | Seal at close |
| ☐ | Ballot Box Seal Record | Restricted |
| ☐ | Receipts signed by the ballot box | Signed by the ballot box |
| ☐ | Period for checking cast ballots | Until a date: the end of the election period |
| ☐ | Scheduled close without signatures | Refuse |
| ☐ | Keys/Tally Ceremonies Policy | Manual Ceremonies |
| ☐ | Batch Anonymity Policy | Refuse |
| ☐ | Voter Digital Certificate Policy | Disabled |
| ☐ | Enrollment; OTP | Both Disabled |
| ☐ | Show User Profile | Off |
| ☐ | Show Cast Vote Logs Tab | Hide Cast Vote Logs Tab |
| ☐ | Voting Channels Allowed | Online only |

### 4. Repeat these checks for every election

| Checked | Setting | Required value |
|---|---|---|
| ☐ | Audit Button Display Options | Show |
| ☐ | Number of allowed votes | As fixed by the electoral regulation; 1 means no re-voting |
| ☐ | Initialize Report Policy | Required |
| ☐ | Grace Period Policy | No grace period; 0 seconds |
| ☐ | Allow Tally | Requires Voting Period End |
| ☐ | Voting Channels Allowed | Online only |

### 5. Check protected actions and certificates

- [ ] Each non-trustee protected action, including **Open voting** and **Close
  voting**, requires at least two signatures and allows only **Election board**.
- [ ] **Approve a configuration version**, **Initialize voting** and
  **Generate other election reports** are enabled.
- [ ] **Confirm custody (key ceremony)** and **Authorize tally custody (tally)**
  are enabled; no non-trustee action has **No limit** expiry.
- [ ] Trusted issuer certificates match their independently checked SHA-256 fingerprints.
- [ ] Certificate registration is **Only when someone who can register
  certificates registers it**.
- [ ] **Check revocation lists** is on with **Don't accept signatures** where
  the CA publishes lists.
- [ ] **A certificate signs for one Post only** is on.
- [ ] Every board member's certificate is registered and **Active**.

### 6. Check monitoring and authentication

- [ ] Review [Self-Test Settings](07-self_tests.md) and its implementation scope.
  A successful electoral-log audit alone does not establish that the complete
  ST self-test suite or its lifecycle gates have passed.
- [ ] **Settings → SECURITY → Authentication failure threshold** is set
  (default 5), and the
  [login checks](08-authentication_and_sessions.md#what-you-check) pass.
- [ ] **Attack detection → Failed logins per minute, all accounts** and
  **Requests per minute, external interfaces** match the organizer's agreed values.

Record each exception and its owner. Do not mark an unavailable control as passed
or treat this checklist as evidence of complete PP/CC conformance.
