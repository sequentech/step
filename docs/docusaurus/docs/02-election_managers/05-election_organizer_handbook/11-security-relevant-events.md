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
on, by electoral phase, and explains how the regular self-tests find events
between two checks.

For every event:

1. Do not change any data to make the problem go away. Keep the evidence: the
   time, the screen, the records on the Logs tab.
2. Record the event in the board's minutes: time, what was seen, by whom, and
   the board's decision.
3. Decide under the electoral regulation, for example to interrupt the
   election execution, to order further self-tests, or to resume.
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
| A setting differs from the evaluated configuration, or a protected action shows "Off" or fewer "Signatures needed" than the regulation fixes | Data tab, Signatures tab | Do not approve; have the administrators correct it. |
| An administrator account holds a board permission, or an account that is not a board member's personal account does | Users and Roles | Stop all security-critical actions, cancel open signing requests, have the administrators correct the roles. |
| A signing request that the board did not start, or whose contents differ from the board's decision | Signatures tab, "Requests" | Abort it with "Cancel request". |
| A ceremony shows "Automatic Ceremony", or a trustee lacks "Custody Confirmed" | Keys tab | Stop; cancel the ceremony; do not continue with the election event. |
| The initialization report counts a ballot, or its hash differs from the `ElectionInitialized` record | Initialization report, Logs tab | Do not start voting. Keep the report. |
| A self-test fails, or the start is refused because a ballot box is not empty | "Self-tests" tab, Dashboard ("Maintenance mode") | Follow [Maintenance mode](12-modes-and-failures.md#maintenance-mode). The election event stays in the preparation phase. |

## Events in the execution phase

| Event | Where it shows | What the election board does |
|---|---|---|
| Maintenance mode entered (`MaintenanceModeEntered`) | Dashboard "Maintenance mode" with the reason, email to the board and administrators, Logs tab | Follow [Maintenance mode](12-modes-and-failures.md#maintenance-mode). |
| Unrecoverable failure: the ballot box test of the self-test failed | Dashboard, "Self-tests" tab, email | Follow [An unrecoverable failure](12-modes-and-failures.md#an-unrecoverable-failure). |
| A regular self-test is missing for longer than the interval | "Self-tests" tab | Select "Run self-test". If no result appears, interrupt the election execution and call the administrators. |
| An account reached the authentication failure threshold | Record `AuthenticationFailureThresholdReached` on the Logs tab for voters' accounts; for board members' and administrators' accounts, in the tenant's electoral log | The TOE locks the account for a while. For a voter: watch for more such records. For a board member or administrator: ask the person whether it was them; if not, treat it as an attack on that account. |
| A brute-force or denial-of-service detection | Monitoring tab, "Attack detections" | Run "Run self-test". Decide under the regulation whether to interrupt the election execution. |
| The audit storage threshold was exceeded | Email; record `AuditStorageThresholdExceeded` on the Logs tab | Voters can no longer log in. Have the administrators provide storage, then decide under the regulation whether the election period suffices. |
| A change of state that the board did not decide: an opening, interruption or termination | Dashboard, Logs tab (`ElectionVotingPeriodOpen`, `…Pause`, `…Close`) | Interrupt the election execution and investigate. |
| A change of users, roles, role assignments, permissions or signing rules (for example `SigningRuleChanged`, `SigningPermissionChanged`) | Logs tab | If the board did not decide it, interrupt the election execution and investigate. The protected actions and their required authorisations can be changed only in the preparation phase. |
| A signing request ended "Failed", or a signature was refused (`SigningSignatureRefused`) | Signatures tab, Logs tab | Read the reason in the log; start the action again only when the cause is clear. |
| An attempt to publish a configuration version after the preparation phase, or a signing request for "Approve a configuration version" | Signatures tab, Logs tab | The TOE refuses publication after the preparation phase. Abort any such request with "Cancel request", and investigate who started it. |
| An import of election data that the board did not approve | Record `ConfigurationPackageImported` on the Logs tab | An import creates a new election event; it does not change the running one. Do not use the new election event; archive it with "Archive this Election Event", and investigate. |
| A board member's security token or a trustee's keys file is lost or exposed | Report of the person | Have the administrators revoke its registration with "Revoke" at once (see [A person leaves, loses the role or loses their security token](03-board-roles-and-permissions.md#a-person-leaves-loses-the-role-or-loses-their-security-token)); for a keys file see [If your keys file is lost or exposed](09-trustee-procedures.md#if-your-keys-file-is-lost-or-exposed). |
| Voters report that they received no receipt, cannot find their ballot, or that "Audit ballot" showed other choices than theirs | Reports from voters | Record each report, run "Run self-test", and decide under the regulation whether to interrupt the election execution. Forward reports of a suspected flaw. |
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

During the execution phase the TOE runs its self-test regularly, every
"Self-test interval (minutes)" (1 to 60 minutes, 15 by default). The
administrators set it in the preparation phase, as the electoral regulation
fixes it. The interval decides how late some security-relevant events are
found.

### Which events the regular self-test finds

| Security-relevant event | Found by | Found at the latest |
|---|---|---|
| Stored votes changed, removed, or out of sequence; a checkpoint or seal that does not match | Self-test 1 (ballot boxes) | At the next regular self-test, so after at most one interval |
| The published configuration no longer matches the approved one | Self-test 2 (imported election data) | After at most one interval |
| Audit records can no longer be written, or the chain of records is broken | Self-test 3 (audit generation) | After at most one interval |
| The connection to remote endpoints no longer meets the TLS profile | Self-test 4 (TLS interface) | After at most one interval |
| Permissions, roles or signing rules changed | Self-test 5 (access-control configuration) | After at most one interval |
| The server clock is no longer synchronised | Self-test 6 (clock) | After at most one interval |
| A crash or shutdown of a server part of the TOE | The TOE itself | Without waiting for a self-test: maintenance mode |
| Failed logins reaching the threshold, attack patterns | The TOE's records and "Attack detections" | When they happen |
| Audit storage above the threshold | The TOE's storage check, every minute | Within one minute |

Between two regular self-tests, an event of the first six kinds may remain
unnoticed, and voting may continue during that time. The shorter the interval,
the fewer votes can be cast between the event and its detection, and the
sooner the election board can interrupt the election execution. The board
therefore:

- checks the "Self-tests" tab at least once per interval, and treats a missing
  run as an event;
- runs "Run self-test" at once whenever it suspects an event between two runs,
  instead of waiting for the next regular one;
- relies on the self-tests that the TOE adds by itself: when the election
  period start time is reached, before "Start Voting" and "Resume Voting",
  after leaving maintenance mode, and under the additional conditions.

### Secure values for the interval

| Use case | Duration of the election period | Interval |
|---|---|---|
| Election held on a single day, for example a council election with a polling day | One day or less | 5 to 15 minutes |
| Any election with high load, for example many voters in the last hours before the end of election time | Any | 5 to 15 minutes |
| Election over several days, for example a staff or student representative election over one or two weeks | More than one day, up to 14 days | 15 to 30 minutes |
| Long election period, for example an election open for several weeks | More than 14 days | 30 to 60 minutes, never more than 60 |

When in doubt, take the shorter value: a shorter interval costs some server
load, a longer one leaves more time in which an event remains unnoticed. The
interval cannot be changed after the preparation phase, so for an election with
high load only in some hours, choose the value for those hours, and add runs
with "Run self-test" during them.

### Additional self-test conditions

The administrators select the "Additional self-test conditions" in the
preparation phase. In the evaluated configuration all three are selected for
every election event. Check the setting with the administrators before the
execution phase, and do not start the election execution while one is missing:

- after each import of election data;
- after each pause of voting;
- before each start of counting.

## Notifications

The TOE emails the election board and the administrators when the election
event enters maintenance mode or stops after an unrecoverable failure, and when
the audit storage threshold is exceeded. Check before the execution phase that
every board member's account has a working email address, and that at least
one board member reads the email throughout the execution phase.
