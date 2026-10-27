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
of the election board. This page lists the events the election board must act
on, by electoral phase, and explains the periodic self-tests.

For every event:

1. Do not change any data to make the problem go away. Keep the evidence: the
   time, the screen, the records on the Logs tab.
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
| An import is refused: the signature, a file or an approval is not valid, there are too few approvals, the revision is not newer than the last one imported, the package enables the results website or has an area with "Allow Early Voting" ticked, or the tenant has no required number of approvals | Error message of "Import Election Event", shown to the importing board member | Stop. Do not approve another file in its place. Ask the election organizer for a new package; if the required number is missing, have the administrators set it. |
| An import record names a configuration or revision that the board did not approve | Logs tab, record `ConfigurationPackageImported` | Do not use that election event. Archive it in the preparation phase with "Archive this Election Event", and investigate. |
| Election data that did not come from a signed configuration package, or an imported value that has changed | Data tab, "Areas" tab, Logs tab | Stop and approve nothing. The voting system accepts election data only as a signed configuration package and refuses every change of imported election data: archive the election event in the preparation phase and investigate. |
| A certificate was registered that the board did not expect, or "Registering a certificate to a person" is not "Only when someone who can register certificates registers it" | Record `SigningCertificateRegistered` on the Logs tab; Signatures tab, "Certificates" | Have the administrators revoke the registration with "Revoke" and correct the setting; check every signature it made. |
| The admin portal asks "Import Without Integrity Check?" | Dialog | Select "Go Back" and enter the digest. |
| An imported Data-tab setting differs from the evaluated configuration | Data tab | Do not approve. Ask the election organizer for a corrected signed package, import it as a new election event and archive the wrong one; administrators check the new values. |
| A protected action shows "Off" or fewer "Signatures needed" than the regulation fixes | Signatures tab | Do not approve; have the administrators correct the separate protected-action rule. |
| An administrator account holds a board permission, or an account that is not a board member's personal account does | Users and Roles | Stop all security-critical actions, cancel open signing requests, have the administrators correct the roles. |
| A signing request that the board did not start, or whose contents differ from the board's decision | Signatures tab, "Requests" | Abort it with "Cancel request". |
| A ceremony shows "Automatic Ceremony", or a trustee lacks "Custody Confirmed" | Keys tab | Stop; cancel the ceremony; do not continue with the election event. |
| The initialization report counts a ballot, or its hash differs from the `ElectionInitialized` record | Initialization report, Logs tab | Do not start voting. Keep the report. |
| Self-test failed, missing, expired or delayed | **Security and self-tests** | Preserve the run identifier and details, investigate the cause and follow the recovery procedure. A missing result is not success. |

## Events in the execution phase

| Event | Where it shows | What the election board does |
|---|---|---|
| Suspected ballot-box integrity failure | Audit/seal verification or other observed evidence | Preserve evidence and follow [An unrecoverable failure](12-modes-and-failures.md#an-unrecoverable-failure); the durable **Unrecoverable failure** state cannot be cleared by a later passing run. |
| Self-test failed, missing, expired or delayed | **Security and self-tests** | Preserve the run identifier and details, investigate the cause and follow the recovery procedure. A missing result is not success. |
| An account reached the authentication failure threshold | Record `AuthenticationFailureThresholdReached` on the Logs tab for voters' accounts; for board members' and administrators' accounts, in the tenant's electoral log | The TOE locks the account for a while. For a voter: watch for more such records. For a board member or administrator: ask the person whether it was them; if not, treat it as an attack on that account. |
| A brute-force or denial-of-service detection | Monitoring tab, "Attack detections" | Request **Logs → Audit** and inspect its Tasks result. Decide under the regulation whether to interrupt the election execution. |
| The audit storage threshold was exceeded | Email; record `AuditStorageThresholdExceeded` on the Logs tab | Voters can no longer log in. Have the administrators provide storage, then decide under the regulation whether the election period suffices. |
| A change of state that the board did not decide: an opening, interruption or termination | Dashboard, Logs tab (`ElectionVotingPeriodOpen`, `…Pause`, `…Close`) | Interrupt the election execution and investigate. |
| A change of users, roles, role assignments, permissions or signing rules (for example `SigningRuleChanged`, `SigningPermissionChanged`) | Logs tab | If the board did not decide it, interrupt the election execution and investigate. The protected actions and their required authorisations can be changed only in the preparation phase. |
| A signing request ended "Failed", or a signature was refused (`SigningSignatureRefused`) | Signatures tab, Logs tab | Read the reason in the log; start the action again only when the cause is clear. |
| An attempt to publish a configuration version after the preparation phase, or a signing request for "Approve a configuration version" | Signatures tab, Logs tab | The TOE refuses publication after the preparation phase. Abort any such request with "Cancel request", and investigate who started it. |
| An import of election data that the board did not approve | Record `ConfigurationPackageImported` on the Logs tab | An import creates a new election event; it does not change the running one. Do not use the new election event; archive it with "Archive this Election Event", and investigate. |
| A board member's security token or a trustee's keys file is lost or exposed | Report of the person | Have the administrators revoke its registration with "Revoke" at once (see [A person leaves, loses the role or loses their security token](03-board-roles-and-permissions.md#a-person-leaves-loses-the-role-or-loses-their-security-token)); for a keys file see [If your keys file is lost or exposed](09-trustee-procedures.md#if-your-keys-file-is-lost-or-exposed). |
| Voters report that they received no receipt, cannot find their ballot, or that "Audit ballot" showed other choices than theirs | Reports from voters | Record each report, request **Logs → Audit**, and decide under the regulation whether to interrupt the election execution. Forward reports of a suspected flaw. |
| A certificate warning in a browser, or a password form of the TOE | Board member's or voter's report | Stop using that connection. Have the operators check the TLS configuration. |

## Events in the evaluation phase

| Event | Where it shows | What the election board does |
|---|---|---|
| The audit of the electoral log fails | Tasks tab, "Audit Electoral Log" | Do not count yet; investigate the findings. |
| A ballot box is "Not sealed: incident", or stays "Sealing overdue" (`BallotBoxSealFailed`) | Dashboard, "Show ballot boxes"; Logs tab | Follow [A ballot box is not sealed](12-modes-and-failures.md#a-ballot-box-is-not-sealed). |
| A ballot box does not match its seal at the counting (`TallyBallotBoxRejected`) | Tally tab, Logs tab | Stop the counting; treat it as an integrity error of the ballot box. |
| The tally is offered without the trustees' custody proofs, or as an automatic ceremony | Tally tab | Select "Cancel Tally"; do not publish any result. |
| The tally fails | Tally tab | Read the logs of the tally; start a new tally only when the cause is clear. |

## Events in the post-processing phase

| Event | Where it shows | What the election board does |
|---|---|---|
| The election-verifier reports `invalid` or `incomplete` | Report of the board, an observer or a voter | See [Reading the result](10-election-verifier.md#reading-the-result). |
| An exported file or its password is lost or exposed | Report | Record it; inform the persons whose data it holds as the law requires. |
| The export or the deletion fails | Tasks tab | Read the reason; do not delete before a readable export exists. |
| "Publish to results website" is offered | Tally tab | Do not use it. The import refuses a package that enables the results website, and no role holds "Edit Results Publication" (`publish-results-write`): have the administrators correct the roles, and treat it as a security-relevant event. |

## Self-test intervals

Administrators configure **Automatic self-tests** in **Security and self-tests**
during preparation. The board checks the saved values before approving the
configuration and reviews actual completion times throughout execution.

### Which events the regular self-test finds

The six checks cover ballot-box integrity, approved election data, audit
recording and integrity, trusted HTTPS channels, approved access-control
configuration and clock synchronization. Expand a run to read each result and
its details. A stale or missing current success blocks voter access; changing
the interval is not a way to dismiss a failure.

### Secure values for the interval

Use the interval fixed by the electoral regulation, between 1 and 60 minutes.
The suite time limit must leave at least 30 seconds before the next interval.
Account for queue delay and check duration when choosing the interval. Review
[Self-Test Settings](../04-administrator_handbook/07-self_tests.md) with the
administrator and operator.

### Additional self-test conditions

Select **After a configuration import**, **After pausing voting** and **Before
counting** during preparation. Before-counting tests, when selected, must pass
before a results tally can start. Startup, scheduled start, manual start,
resumption and leaving maintenance checks remain mandatory; these additional
settings cannot disable them.

## Notifications

The TOE queues separate email notices to the election board and administrators when the election
event enters maintenance mode or stops after an unrecoverable failure, and when
the audit storage threshold is exceeded. Check before the execution phase that
every board member's account has a working email address, and that at least
one board member reads the email throughout the execution phase.


Check **Security email delivery** as well as the mailbox. **Accepted by mail
transport** means the mail server accepted the message; it does not prove that
a person read it. Investigate **Delivery failed** or **Delivery outcome unknown**
with the operator. Continue monitoring the security state even when email is
unavailable.
