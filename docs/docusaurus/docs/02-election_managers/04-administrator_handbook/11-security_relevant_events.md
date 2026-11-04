---
id: security_relevant_events
title: Security-Relevant Events for Administrators
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## How to handle a security-relevant event

For every event of this chapter:

1. Act at once, in the way the table says.
2. Tell the election board, and the election organizer where the table says
   so, what you saw, with the date, the time and its time zone, and what you did.
   The board records it in the board's minutes
   ([Security-Relevant Events for the Election Board](../05-election_organizer_handbook/11-security-relevant-events.md)).
3. If the event may be caused by a flaw of the TOE, report it
   ([Environment Measures and Security Problems](12-environment_and_security_problems.md#reporting-a-security-problem)).

## In every phase

| Event | How you notice it | What you do |
|---|---|---|
| A person leaves or changes function | the election organizer tells you | The same day: disable the account, remove its roles, revoke the registration of its signing certificates, ask for the credentials at the external authentication server to be withdrawn, and check that each protected action still has enough signers ([Users, Roles and Role Separation](04-users_and_roles.md#when-a-person-leaves-or-changes-function)). |
| A board member's security token is lost or stolen | the board member or the board tells you | Revoke the certificate's registration at once: one revocation covers every election event of the tenant; the election organizer revokes the certificate at its CA. |
| An account holds a board permission and is not a board member's account, or an administrator account holds a board permission | your check of Users and Roles, or the election board tells you | Remove the account from the role that gives the permission; if that role is `admin`, stop and tell the operator ([The administrator role](04-users_and_roles.md#the-administrator-role)). Tell the election board and the election organizer; the board checks the electoral log for actions of that account. |
| An account reached the authentication failure threshold | the election board tells you: it sees the record on the election event's tab "Dashboard", under "Attack detections" | Ask the organisation that runs the external authentication server whose sign-ins failed, and find out with that person whether the attempts were theirs. If they were not, tell the election board. ([Authentication failure threshold](08-authentication_and_sessions.md#authentication-failure-threshold)) |
| An attack detection matched | the election board tells you | Tell the operator, and whoever runs the network if that is someone else, and agree the measures with the board ([Audit, Monitoring and Notifications](09-audit_and_monitoring.md#attack-detections)). |
| The header's "Version:" is not `10.0.0`, or the TOE software differs from the accepted delivery | your checks ([Checking the version of the TOE](03-admin_portal.md#checking-the-version-of-the-toe), [Verify the TOE software](07-self_tests.md#verify-the-toe-software)) | The system is not the evaluated TOE: do not start or continue an election event; tell the election board and the election organizer; report it. |
| The audit storage threshold is exceeded | an email, and the area "Security email delivery" of the tab "Security and self-tests" of the election event | Have the operator extend storage ([Storage](../../05-reference/06-deployment_guide/11-selftest_handover.md#storage)); never delete records. A fresh measurement at or below the threshold removes this refusal; the other phase, period and self-test checks still apply. |
| A server component crashed or was shut down | the tab "Security and self-tests" of the election event shows "Recovery required"; the operator identifies the component with the service checks of the Deployment Guide | Tell the election board the reason the tab shows and the time; it records them in the board's minutes. Ask the operator to read the log of the failed component and tell the board what it shows. Obtain the board's decision before the operator restarts it during execution. After the restart, [verify the TOE software](07-self_tests.md#verify-the-toe-software), then follow [Maintenance mode](10-modes_of_operation.md#maintenance-mode). |
| The browser reports a certificate problem for the admin portal or the external authentication server | the browser | Do not sign in; tell the operator. |
| A notification email does not arrive, or arrives for an event you did not expect | email | Check the account's "Email" and tell the operator; ask the election board to check the electoral log for the event. |

## Preparation phase

| Event | How you notice it | What you do |
|---|---|---|
| An import of the election board is refused: the digest, the signature or the approvals are not valid | the election board tells you | Open **Settings → Configuration packages** and check that the saved CA fingerprints and approval count still match your approved values ([Import of election data](05-tenant_and_event_settings.md#import-of-election-data)); do not change them. The board asks the election organizer for a new package. |
| A setting of an imported election event differs from the required value | your checks ([Tenant and Election Event Settings](05-tenant_and_event_settings.md)) | Stop before publication and tell the election board and the election organizer. The organizer includes the required value in a corrected signed package; the board imports a new election event and archives the wrong one. Check the new event without repairing imported values in the TOE. |
| The imported election data is wrong | the election board's check | Nobody corrects it in the TOE. The election organizer issues a corrected package, which the election board imports; the board archives the wrong election event. Check the settings of the new one. |
| The TOE refuses a change with a message that starts "The election event is locked down" | the message | The election event is in a lockdown period: change nothing. A needed change goes through a new configuration version, with the election board. |
| An electoral-log audit fails, is missing or is delayed | the election board tells you: it sees the task "Audit Electoral Log" in the tab "Tasks" of the election event | Tell the operator and help the board to find the cause. An electoral-log audit does not replace the six self-test checks ([Security and Self-Tests](07-self_tests.md)). |
| A ballot box is not empty when the execution phase is to start | the tab "Security and self-tests" of the election event shows "Recovery required" with the reason "Ballot box not empty at the start of the election" | Tell the election board; do not try to empty it; report it. |
| A scheduled opening or closing shows "Will be refused" or "Will run without signatures" | the election board tells you | The board reads the reason under the button "Why?". The state "Will run without signatures" means that the setting "Scheduled close without signatures" is "Run as system", which [Configuration the board must verify](05-tenant_and_event_settings.md#configuration-the-board-must-verify) forbids: treat it as a setting that differs from the required value. For the state "Will be refused", tell the board every setting you changed after the approval. |
| A signing request ends as "Failed" | the election board tells you | Check the protected action's rule and the certificate registrations on the Signatures tab with the board. |

## Execution phase

| Event | How you notice it | What you do |
|---|---|---|
| An electoral-log audit fails, is missing or is delayed | the election board tells you: it sees the task "Audit Electoral Log" in the tab "Tasks" of the election event | Tell the operator and help the board to find the cause. An electoral-log audit does not replace the six self-test checks ([Security and Self-Tests](07-self_tests.md)). |
| Suspected ballot-box integrity failure | **Unrecoverable failure** and the ballot-box check result, or an operator report | Support the election board; ask the operator to read the log of the failed component and tell the board what it shows; report it ([Modes of Operation and Failure Handling](10-modes_of_operation.md#an-unrecoverable-failure)). |
| The server clock is not synchronised | the self-test check "Clock synchronization" fails | Have the operator restore the time service ([Authenticated time](../../05-reference/06-deployment_guide/04-hosts_network_time.md#authenticated-time)). |

## Evaluation and post-processing phases

| Event | How you notice it | What you do |
|---|---|---|
| The audit of the electoral log reports FAILED | the election board tells you | Support the election board; it decides under the electoral regulation. Report it. |
| The board finds a record it cannot explain | the board's audit review | Help the board to find the cause. |
| A request to delete or archive an election event reaches you | anyone | Refuse: the administrator role holds neither permission. Only the election board archives an election event, or deletes one after the export with its required authorisations. |

## Checklist before the election board approves the configuration version

Fill in a copy of this checklist, on paper or as a file, naming the election
event and the configuration version. Work through one section at a time. A tick
records a completed check, not an approval: the election board approves the
configuration version. If a check fails, stop and
resolve it before asking for approval. Hand the completed copy to the election
board, which keeps it with the board's minutes.

### Verify the delivery and imported configuration {#1-verify-the-delivery-and-imported-configuration}

- ☐ The header shows "Version:" `10.0.0`.
- ☐ Signed configuration packages are required, with the organizer's checked
  CA certificates and at least two approvals.
- ☐ The board confirms that a board member used **Import Election Event** and
  checked the digest received from the organizer. Tell the operator that an
  election event was imported.
- ☐ [Verify the TOE software](07-self_tests.md#verify-the-toe-software) shows
  "Matches the accepted delivery", and the "SHA-256 of the accepted digest list"
  matches the value the operator gave you.
- ☐ The operator tells you that the checks of
  [Data correction, telephone channel, plugins and tenants](../../05-reference/06-deployment_guide/10-verification.md#data-correction-telephone-channel-plugins-and-tenants)
  passed: election work uses only the customer's tenant, no plugin is installed,
  and no telephone provider is configured.

### Check the accounts and permissions {#2-check-the-accounts-and-permissions}

- ☐ No account combines an administrator role with a board permission.
- ☐ "Role Data" of `admin` shows no "Sign:" permission, no **Edit Election
  State** and no **Create Election Event**
  ([The administrator role](04-users_and_roles.md#the-administrator-role)).
- ☐ No role has **Edit Results Publication** (`publish-results-write`),
  and no account has **Datafix account** (`datafix-account`) or **Manually Verify
  Voter** (`voter-manually-verify`).
- ☐ The seeded `admin` account is deleted.
- ☐ The roles `election-board` and `trustee` hold exactly the permissions of
  [The election board role](04-users_and_roles.md#the-election-board-role)
  and [The trustee role](04-users_and_roles.md#the-trustee-role).
- ☐ Each administrator and board member received the test email of
  [Check monitoring and authentication](#6-check-monitoring-and-authentication).

### Check the event settings with the board {#3-check-the-event-settings-with-the-board}

Compare the controls visible in **Data → Security settings**. For the other
rows, obtain the board's confirmation from its read-only Data view. Tick a row
only after that check; do not add a board permission to your account.

| Checked | Setting | Required value |
|---|---|---|
| ☐ | Ballot box sealing | Seal when voting closes |
| ☐ | Seal record access | Election board only |
| ☐ | Decoded ballots in results | Do not include |
| ☐ | Key ceremony scope | The election organizer's written value |
| ☐ | Ballot encryption | The election organizer's written value |
| ☐ | Voter ballot signatures | The election organizer's written value |
| ☐ | Date and time | The election organizer's written zones |
| ☐ | Receipts signed by the ballot box | Signed by the ballot box |
| ☐ | Period for checking cast ballots | Until a date: the end of the election period |
| ☐ | Scheduled close without signatures | Refuse |
| ☐ | Trustee participation | Trustees participate |
| ☐ | Batch Anonymity Policy | Refuse |
| ☐ | Voter Digital Certificate Policy | Disabled |
| ☐ | Enrollment; OTP | Both Disabled |
| ☐ | Show User Profile | Off |
| ☐ | Show Cast Vote Logs Tab | Hide Cast Vote Logs Tab |
| ☐ | Voting Channels Allowed | Online only |

### Obtain the board's checks for every election {#4-obtain-the-boards-checks-for-every-election}

| Checked | Setting | Required value |
|---|---|---|
| ☐ | Audit Button Display Options | Show |
| ☐ | Number of allowed votes | As fixed by the electoral regulation; 1 means no re-voting |
| ☐ | Initialize Report Policy | Required |
| ☐ | Grace Period Policy | No grace period; 0 seconds |
| ☐ | Allow Tally | Requires Voting Period End |
| ☐ | Voting Channels Allowed | Online only |

### Check protected actions and certificates {#5-check-protected-actions-and-certificates}

- ☐ Each protected action that
  [Security-critical authorisations required by the Security Target](06-signing_rules.md#security-critical-authorisations-required-by-the-security-target)
  and [Protected actions to switch on](06-signing_rules.md#protected-actions-to-switch-on)
  require and that is not a trustee row, including **Open voting** and **Close
  voting**, requires at least two signatures and allows only `election-board`.
- ☐ **Import election data**, **Approve a configuration version**, **Start
  counting votes**, **Export election execution data**, **Delete election
  execution data**, and **Read the voters' register in post-processing** each
  have the required board count. None is left undefined.
- ☐ **Initialize voting** and **Generate other election reports** are enabled.
- ☐ **Confirm custody (key ceremony)** and **Authorize tally custody (tally)**
  are enabled; no non-trustee action has **No limit** expiry.
- ☐ The complete SHA-256 fingerprint of each trusted issuer, copied with its
  copy icon, matches the value the election organizer stated in writing.
- ☐ Certificate registration is **Only when someone who can register
  certificates registers it**.
- ☐ **Check revocation lists** is on with **Don't accept signatures** where
  the CA publishes lists.
- ☐ **A certificate signs for one election only** is on.
- ☐ Every board member's certificate is registered and **Active**, or **Expires
  soon** with a validity that covers the required period
  ([Registered certificates](06-signing_rules.md#registered-certificates)).

### Check monitoring and authentication {#6-check-monitoring-and-authentication}

- ☐ Configure [Security and Self-Tests](07-self_tests.md): interval, timeout
  and both additional conditions.
- ☐ Confirm that the board knows to run **Before starting voting** after
  publication and initialization, **Before resuming voting** after a pause or
  recovery, and **Before counting** before it authorizes counting. A standalone electoral-log audit does not replace these checks.
- ☐ Ask the election board to create an empty election event with the entry "Create
  an Election Event" of the side menu and to archive it with the entry "Archive this
  Election Event" of its menu: the TOE then emails every administrator and board
  member ([Email notifications](09-audit_and_monitoring.md#email-notifications)).
  Confirm that each of them received it. If someone did not, tell the operator
  ([A security notification is not delivered](../../05-reference/06-deployment_guide/11-selftest_handover.md#a-security-notification-is-not-delivered)).
- ☐ The operator tells you that the checks of
  [Election event realms](../../05-reference/06-deployment_guide/10-verification.md#election-event-realms) passed, and
  your [login checks](08-authentication_and_sessions.md#what-you-check) pass.
- ☐ In **Settings → Attack detection**, **Failed logins per minute, all accounts**,
  **Requests per minute, external interfaces** and **Failed sign-ins per account
  before an alert** match the values agreed with the election organizer.

Note each exception and who resolves it on your copy. Do not mark an unavailable control as passed.
A completed checklist shows that the checks were made; it does not by itself prove
the evaluated configuration.
