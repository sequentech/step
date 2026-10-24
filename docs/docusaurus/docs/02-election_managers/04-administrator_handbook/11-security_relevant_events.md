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
| A server component crashed or was shut down | the state check of the Deployment Guide ("Operating the servers"); maintenance mode in the execution phase | Have it restarted as the Deployment Guide describes, in the execution phase only when the election board has decided so, then check the version and the image digests. Outside the execution phase, run the self-test; in it, the self-test and "Resume Voting" are the election board's ([Modes of Operation and Failure Handling](10-modes_of_operation.md)). |
| The browser reports a certificate problem for the admin portal or the external authentication server | the browser | Do not sign in; tell the person who runs the servers. |
| A notification email does not arrive, or arrives for an event you did not expect | email | Check the account's "Email" and tell the person who runs the servers; ask the election board to check the electoral log for the event. |

## Preparation phase

| Event | How you notice it | What you do |
|---|---|---|
| An import of the election board is refused: the digest, the signature or the approvals are not valid | the election board tells you | Check with a backup that `configuration_signing` still has your values; do not change it. The board asks the election organizer for a new package ([Tenant and Election Event Settings](05-tenant_and_event_settings.md#import-of-election-data)). |
| A setting of an imported election event differs from the required value | your checks ([Tenant and Election Event Settings](05-tenant_and_event_settings.md)) | Set the required value before the configuration version is approved, and tell the election board and the election organizer so that the next package is correct. |
| The imported election data is wrong | the election board's check | Nobody corrects it in the TOE. The election organizer issues a corrected package, which the election board imports; the board archives the wrong election event. Check the settings of the new one. |
| The TOE refuses a change with "The election event is locked down: …" | the message | The election event is in a lockdown period: change nothing. A needed change goes through a new configuration version, with the election board. |
| A self-test fails | "Self-tests" tab; maintenance mode at the start | Find and remove the cause; the election board starts again ([Modes of Operation and Failure Handling](10-modes_of_operation.md#maintenance-mode)). |
| A ballot box is not empty when the execution phase is to start | maintenance mode | Tell the election board; do not try to empty it; report it. |
| A scheduled opening or closing shows "Will be refused" or "Will run without signatures" | the election board tells you | The schedule does not match the approved configuration version. Tell the board every setting you changed after the approval. |
| A signing request ends as "Failed" | the election board tells you | Check the protected action's rule and the certificate registrations on the Signatures tab with the board. |

## Execution phase

| Event | How you notice it | What you do |
|---|---|---|
| A regular self-test fails | maintenance mode; email | Find and remove the cause with the person who runs the servers; the election board resumes with "Resume Voting" ([Modes of Operation and Failure Handling](10-modes_of_operation.md#maintenance-mode)). |
| A regular self-test run is missing from the "Self-tests" tab | your check, at least once per interval | Tell the election board at once; a board member or you select "Run self-test". |
| You suspect a manipulation between two regular self-tests | any observation | Tell the election board; select "Run self-test" at once instead of waiting for the next run ([Self-Test Settings](07-self_tests.md#security-relevant-events-and-the-interval)). |
| The ballot box test fails: an unrecoverable failure | email | Support the election board; keep all server logs; report it ([Modes of Operation and Failure Handling](10-modes_of_operation.md#an-unrecoverable-failure)). |
| The server clock is not synchronised | self-test 6 fails | Restore the time service as the Deployment Guide describes. |

## Evaluation and post-processing phases

| Event | How you notice it | What you do |
|---|---|---|
| The audit of the electoral log reports FAILED | the election board tells you | Support the election board; it decides under the electoral regulation. Report it. |
| The board finds a record it cannot explain | the board's audit review | Help the board to find the cause from your written record of changes. |
| A request to delete or archive an election event reaches you | anyone | Refuse: the administrator role holds neither permission. Only the election board archives an election event, or deletes one after the export with its required authorisations. |

## Checklist before the election board approves the configuration version

For each election event, check and tick:

- "Version" in the header is `Sequent Voting Platform - uniWAHL Version, v10.0.0`, and every running image digest is in `image-digests.txt`.
- The tenant's signed configuration packages are `required`, with the election organizer's CA certificates and at least 2 approvals.
- The election board has confirmed that a board member imported the election event with "Import Election Event" and the digest from the election organizer.
- No account holds both the administrator role and a board permission; the administrator role holds no permission of [Permissions on election data and audit records](04-users_and_roles.md#permissions-on-election-data-and-audit-records); no role holds "Edit Results Publication" (`publish-results-write`) and no account holds "Datafix account" (`datafix-account`); the seeded account `admin` is deleted; every board member holds "Create Election Event", "Signatures: cancel requests", "Export Logs" and "Edit Reports", and the read-only user and role permissions.
- Election event, tab "Data": "Ballot Box Seal Policy" "Seal at close"; "Ballot Box Seal Record" "Restricted"; "Receipts signed by the ballot box" "Signed by the ballot box"; "Period for checking cast ballots" "Until a date" at the end of the election period; "Scheduled close without signatures" "Refuse"; "Keys/Tally Ceremonies Policy" "Manual Ceremonies"; "Batch Anonymity Policy" "Refuse"; "Voter Digital Certificate Policy" "Disabled"; "Enrollment" and "OTP" "Disabled"; "Show User Profile" off; "Show Cast Vote Logs Tab" "Hide Cast Vote Logs Tab"; "Voting Channels Allowed" "Online" only.
- Each election: "Audit Button Display Options" "Show"; "Number of allowed votes" as the electoral regulation fixes (1 = no re-voting); "Initialize Report Policy" "Required"; "Grace Period Policy" "No grace period" with 0 seconds; "Allow Tally" "Requires Voting Period End"; "Voting Channels Allowed" "Online" only.
- Election work happens only in the customer's tenant; the person who runs the servers has confirmed in writing, for this election event, that no plugin is installed, no telephone provider is configured and only the customer's tenant holds election work.
- Signatures tab: every protected action the tab lists, including "Open voting" and "Close voting", has "Signatures needed" set, at least 2, with only "Election board" in "Who can sign"; "Approve a configuration version", "Initialize voting", "Generate other election reports", "Confirm custody (key ceremony)" and "Authorize tally custody (tally)" are on; no expiry is "No limit".
- Certificates: the election organizer's trusted issuers with checked "SHA-256"; "Registering a certificate to a person" "Only when someone who can register certificates registers it"; "Check revocation lists" on with "Don't accept signatures" where the CA publishes lists; "A certificate signs for one Post only" on; every board member's certificate registered and "Active".
- Self-test settings: "Self-test interval (minutes)" from the secure values; all three "Additional self-test conditions" selected.
- Settings > "SECURITY": "Authentication failure threshold" set (default 5); the login checks of [Authentication and Session Settings](08-authentication_and_sessions.md#what-you-check) pass.
- Settings, section "Attack detection": "Failed logins per minute, all accounts" and "Requests per minute, external interfaces" set with the election organizer.
- Every administrator's and board member's account has a working "Email".
