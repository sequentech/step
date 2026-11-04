---
id: security_events
title: Security-Relevant Events for the Election Board
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A security-relevant event is anything that may affect the integrity of the
election data or of the votes, the secrecy of the votes, or the authorisations
of the election board. This chapter lists the events the election board must act
on, by electoral phase, and explains the periodic self-tests. The tabs that the
tables name are tabs of the election event. On the tab "Logs", the column "Statement
kind" shows `LifecycleSecurityEvent` for the TOE's notices about its own state, such
as `PotentialAttackDetected`, and their column "Description" starts with that name.

For every event:

1. Do not change any data to make the problem go away. Keep the evidence with the board's minutes: the
   time, the screen, the records on the tab "Logs".
2. Record the event in the board's minutes: time, what was seen, by whom, and
   the board's decision.
3. Decide under the electoral regulation, for example to interrupt the
   election execution, to request further integrity audits, or to resume.
4. If the event may be caused by a flaw of the TOE, report it as
   [Reporting a security problem](02-organizer-duties.md#reporting-a-security-problem)
   describes.

## Events in the preparation phase

| Event | Where it shows | What the election board does |
|---|---|---|
| A package to approve differs from what the election organizer announced, or the approvers of a revision see different approval codes | The election organizer's package tool | Do not approve. Tell the election organizer. |
| An import is refused: the signature, a file or an approval is not valid, there are too few approvals, the revision is not newer than the last one imported, an area has the setting "Allow Early Voting" ticked while the election event's setting "Voting Channels Allowed" is "Online" only, or the tenant has no required number of approvals | Error message in the drawer "Import Election Event", shown to the importing board member | Stop. Do not approve another file in its place. Ask the election organizer for a new package; if the required number is missing, have the administrators set it. |
| An import record names a configuration or revision that the board did not approve | Tab "Logs", record `ConfigurationPackageImported` | Do not use that election event. Archive it in the preparation phase with the menu entry "Archive this Election Event", and investigate. |
| Election data that did not come from a signed configuration package, or an imported value that has changed | Tabs "Data", "Areas" and "Logs" | Stop and approve nothing. The TOE accepts election data only as a signed configuration package and refuses every change of imported election data: archive the election event in the preparation phase and investigate. |
| A certificate was registered that the board did not expect, or the check "Registering a certificate to a person" is not "Only when someone who can register certificates registers it" | Record `SigningCertificateRegistered` on the tab "Logs"; tab "Signatures", area "Certificates" | Have the administrators revoke the registration with the button "Revoke" and correct the setting; check every signature it made. |
| The admin portal asks in a dialog "Import Without Integrity Check?" | Dialog | Select the button "Go Back" and enter the digest. |
| An imported setting on the tab "Data" differs from the evaluated configuration | Tab "Data" | Do not approve. Ask the election organizer for a corrected signed package, import it as a new election event and archive the wrong one; administrators check the new values. |
| A protected action other than those that [Protected actions to switch on](../04-administrator_handbook/06-signing_rules.md#protected-actions-to-switch-on) leaves unused shows the value "Off", or fewer signatures in the field "Signatures needed" than the regulation fixes | Tab "Signatures" | Do not approve; have the administrators correct the separate protected-action rule. |
| An administrator account holds a board permission, or an account that is not a board member's personal account does | The side menu "Users and Roles" | Stop all security-critical actions, cancel open signing requests, have the administrators correct the roles. |
| A signing request that the board did not start, or whose contents differ from the board's decision | Tab "Signatures", area "Requests" | Abort it with the button "Cancel request". |
| The form of a key ceremony offers the switch "Automatic Ceremony", or a trustee has no tick in the column "Custody Confirmed" when the ceremony is reported complete | Tab "Keys" | Stop; cancel the ceremony; do not continue with the election event. |
| The initialization report counts a ballot, or its hash differs from the record `ElectionInitialized` | The initialization report; tab "Logs" | Do not start voting. Keep the report with the board's minutes. |
| Self-test failed, missing, expired or delayed | Tab "Security and self-tests" | Write the time of the run and its details in the board's minutes, investigate the cause and follow [Maintenance mode at the start](12-modes-and-failures.md#maintenance-mode-at-the-start). A missing result is not success. |
| The check of the TOE software shows the result "Differs from the accepted delivery" | Tab "Security and self-tests", section "TOE software integrity" | Do not start or resume voting. Tell the administrators and the election organizer, and follow [Maintenance mode](12-modes-and-failures.md#maintenance-mode). |

## Events in the execution phase

| Event | Where it shows | What the election board does |
|---|---|---|
| Suspected ballot-box integrity failure | The self-test check "Ballot-box integrity" on the tab "Security and self-tests", the audit on the tab "Tasks", or other observed evidence | Keep the evidence with the board's minutes and follow [An unrecoverable failure](12-modes-and-failures.md#an-unrecoverable-failure); a later passing run does not clear the state "Unrecoverable failure". |
| Self-test failed, missing, expired or delayed | Tab "Security and self-tests" | Write the time of the run and its details in the board's minutes, investigate the cause and follow [Maintenance mode](12-modes-and-failures.md#maintenance-mode). A missing result is not success. |
| The check of the TOE software shows the result "Differs from the accepted delivery" | Tab "Security and self-tests", section "TOE software integrity" | Do not start or resume voting. Tell the administrators and the election organizer, and follow [Maintenance mode](12-modes-and-failures.md#maintenance-mode). |
| An account reached the authentication failure threshold | Tab "Dashboard", dashboard "Attack detections", category `brute-force-account`, and the record `PotentialAttackDetected` on the tab "Logs": of the election event for voters' accounts, of every election event of the tenant for board members' and administrators' accounts | The TOE does not lock the account and sends no email. For a voter: watch for more such records. For a board member or administrator: ask the person whether it was them; if not, treat it as an attack on that account. |
| A brute-force or denial-of-service detection | Tab "Dashboard", dashboard "Attack detections" | Run the audit with the button "Audit" on the tab "Logs" and read its result on the tab "Tasks". Tell the administrators. Decide under the regulation whether to interrupt the election execution. |
| The audit storage threshold was exceeded, or the TOE has no current measurement of the audit storage | Email; record `AuditStorageThresholdExceeded` or `AuditStorageObservationUnavailable` on the tab "Logs" | Voters can no longer log in. Have the administrators provide storage, then decide under the regulation whether the election period suffices. |
| A change of state that the board did not decide: an opening, interruption or termination | Tabs "Dashboard" and "Logs" (records `ElectionVotingPeriodOpen`, `…Pause`, `…Close`) | Interrupt the election execution and investigate. |
| A change of signing rules, "Sign:" permissions, trusted issuers or certificate checks, or a certificate registration or revocation (records `SigningRuleChanged`, `SigningPermissionChanged`, `SigningIssuerChanged`, `SigningChecksChanged`, `SigningCertificateRegistered`, `SigningCertificateRevoked`) | Tab "Logs" | If the board did not decide it, interrupt the election execution and investigate. The protected actions and their required authorisations can be changed only in the preparation phase. |
| A new account of an administrator or board member, a new role assignment or a change of another role permission. The electoral log does not record these | The self-test check "Approved access-control configuration" fails, on the tab "Security and self-tests", and voting stops | Have the administrators restore the approved values and follow [Maintenance mode](12-modes-and-failures.md#maintenance-mode). If the board did not decide the change, investigate it. |
| A signing request ended with the status "Failed", or a signature was refused (record `SigningSignatureRefused`) | Tabs "Signatures" and "Logs" | Read the reason in the log; start the action again only when the cause is clear. |
| An attempt to publish a configuration version after the preparation phase, or a signing request for the protected action "Approve a configuration version" | Tabs "Signatures" and "Logs" | The TOE refuses publication after the preparation phase. Abort any such request with the button "Cancel request", and investigate who started it. |
| An import of election data that the board did not approve | A new election event under the side menu's tab "Active", and the record `ConfigurationPackageImported` on that new election event's tab "Logs" | An import creates a new election event; it does not change the running one. Do not use the new election event; archive it with the menu entry "Archive this Election Event", and investigate. |
| A board member's security token or a trustee's keys file is lost or exposed | Report of the person | Have the administrators revoke its registration with the button "Revoke" at once (see [A person leaves, loses the role or loses their security token](03-board-roles-and-permissions.md#a-person-leaves-loses-the-role-or-loses-their-security-token)); for a keys file see [If your keys file is lost or exposed](09-trustee-procedures.md#if-your-keys-file-is-lost-or-exposed). |
| Voters report that they received no receipt, cannot find their ballot, or that the button "Audit ballot" showed other choices than theirs | Reports from voters | Write each report in the board's minutes, run the audit with the button "Audit" on the tab "Logs", and decide under the regulation whether to interrupt the election execution. Forward reports of a suspected flaw. |
| A certificate warning in a browser, or a password form of the TOE | Board member's or voter's report | Stop using that connection. Have the operators check the TLS configuration. |

## Events in the evaluation phase

| Event | Where it shows | What the election board does |
|---|---|---|
| The audit of the electoral log fails | Tab "Tasks", task "Audit Electoral Log" | Do not count yet; investigate the findings. |
| The status of a ballot box is "Not sealed: incident", or stays "Sealing overdue" (record `BallotBoxSealFailed`) | Tab "Dashboard", with the switch "Show ballot boxes" on; tab "Logs" | Follow [A ballot box is not sealed](12-modes-and-failures.md#a-ballot-box-is-not-sealed). |
| A ballot box does not match its seal at the counting (record `TallyBallotBoxRejected`) | Tabs "Tally" and "Logs" | Stop the counting; treat it as an integrity error of the ballot box. |
| The tally is offered without the trustees' custody proofs, or as an automatic ceremony | Tab "Tally" | Select the button "Cancel Tally"; do not publish any result. |
| The tally fails | Tab "Tally" | Read the logs of the tally; start a new tally only when the cause is clear. |

## Events in the post-processing phase

| Event | Where it shows | What the election board does |
|---|---|---|
| The election verifier reports the status `invalid` or `incomplete` | Report of the board, an observer or a voter | See [Reading the result](10-election-verifier.md#reading-the-result). |
| An exported file or its password is lost or exposed | Report | Write it in the board's minutes; inform the persons whose data it holds as the law requires. |
| The export or the deletion fails | Tab "Tasks" | Read the reason; do not delete before a readable export exists. |
| The section "Publish to results website" is offered | Tab "Tally" | Do not use it. No role holds the permission "Edit Results Publication" (`publish-results-write`), so nobody can use it, whatever a package contains: have the administrators correct the roles, and treat it as a security-relevant event. |

## Self-test intervals

The administrators, who hold the permission "Self-tests: edit settings"
(`self-test-settings-write`), configure the section "Automatic self-tests" of the
election event's tab "Security and self-tests" during the preparation phase. The board checks
the saved values before approving the configuration and reviews actual completion
times throughout the execution phase.

### Which events the regular self-test finds

The six checks cover ballot-box integrity, approved election data, audit
recording and integrity, trusted HTTPS channels, approved access-control
configuration and clock synchronization. Expand a run to read each result and
its details. A stale or missing current success blocks voter access; changing
the interval is not a way to dismiss a failure.

| Result of a failed check | Which failures | State on the tab |
|---|---|---|
| Recoverable, maintenance mode | Every failed check except the two of the next row. A difference from the approved baseline, in the election data or the access control, is recoverable: restore the exact approved values and verify again | "Recovery required" |
| Unrecoverable | A ballot-box integrity mismatch, and a verified integrity finding in the electoral log | "Unrecoverable failure" |

Until the board approves a configuration version, a run at start-up or with the
purpose "On request" shows the two baseline checks as passed with the detail
"No board-approved baseline yet; nothing to compare": nothing has been approved yet
to compare with. The admin portal offers the other purposes only once the baseline
exists.

### Secure values for the interval

Use the interval fixed by the electoral regulation, between 1 and 60 minutes.
The suite time limit must leave at least 30 seconds before the next interval.
Account for queue delay and check duration when choosing the interval. Review
[Security and Self-Tests](../04-administrator_handbook/07-self_tests.md) with the
administrators and the operators.

### Additional self-test conditions

The administrators select both additional conditions, "After pausing voting" and
"Before counting", during the preparation phase. With the condition "Before counting" selected, a
run with that purpose must have passed at most five minutes before the board's
request to count, and must still be the latest finished run when counting executes. The runs at
start-up, at a scheduled start, before starting voting, before resuming voting,
after maintenance and at each interval always happen; these additional settings
cannot disable them.

## Notifications

The TOE emails the election board and the administrators about the events that
[Email notifications](12-modes-and-failures.md#email-notifications) lists. Check before the execution phase that
every board member's account has a working email address, and that at least
one board member reads the email throughout the execution phase.

Check the area "Security email delivery" of the election event's tab "Security and
self-tests" as well as the mailbox. It appears once a notification exists. The outcome
"Accepted by mail transport" means the mail server accepted the message; it does not
prove that a person read it. Investigate the outcomes "Delivery failed" and "Delivery
outcome unknown" with the operators. Continue monitoring the security state even when email is
unavailable.
