---
id: modes_failures
title: Operating Modes, Failures and Recovery for the Election Board
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

An election event is always in one of the modes below. This page says how the
election board recognises each mode, what the TOE does in it, and how the board
returns the election event to a secure state after a failure or an operational
error.

## Modes of operation

The column "How it shows" names what the election event's tab "Security and
self-tests" shows, unless it says otherwise.

| Mode | When | Voters | Election board | How it shows |
|---|---|---|---|---|
| Preparation | From the import until the start of the election execution | Cannot open an election action | Approves the configuration packages and imports them, checks the import and the settings, publishes and approves the configuration version, sets the lockdown period, runs the key ceremony and the initialization report | "Preparation" |
| Normal operation | From the start until the termination of the election execution | Vote, check and audit ballots | Monitors, reviews integrity audits, interrupts, terminates | "Election execution" and "Operational"; the elections are open |
| Interrupted | After the button "Pause Voting" | Cannot open or continue an election action | Investigates, reviews integrity audits, resumes or terminates | The status of the elections is "Paused"; the tab "Dashboard" shows the text "Voting is paused. …" |
| Maintenance mode | Recoverable failure, expired checks, or a required service stopping or restarting | Election requests are refused; ballot-box access is blocked during the execution phase | Investigates and authorizes recovery after fresh tests | "Recovery required", with the reason |
| Unrecoverable failure | A ballot-box integrity mismatch, or a verified integrity finding in the electoral log | Election requests and execution-phase ballot-box access remain blocked | Preserves evidence, terminates and exports for investigation | "Unrecoverable failure" |
| Audit storage restriction | The storage of the audit data is more than 80 % full (see [The audit storage restriction](#the-audit-storage-restriction)) | Cannot log in; their requests are refused | Continues to work | Email; the record `AuditStorageThresholdExceeded` on the tab "Logs" |
| Evaluation | After the termination | Cannot open an election action | Checks the audit records, counts the votes | "Evaluation"; on the tab "Dashboard", the status "Sealed" of the ballot boxes |
| Ballot box not sealed | A ballot box could not be sealed at the termination (see [A ballot box is not sealed](#a-ballot-box-is-not-sealed)) | Cannot open an election action | Records the evidence, decides; that election cannot be counted | On the tab "Dashboard", the status "Not sealed: incident" |
| Post-processing | After the button "Start post-processing" | Cannot open an election action | Reads, exports, publishes the verification data, deletes | "Post-processing" |

The phases follow each other in this order only. The TOE never returns an
election event to an earlier phase.

## Maintenance mode

Read the current state on the election event's tab "Security and self-tests". The
state "Recovery required" means that the election event cannot continue until the cause has been removed,
current checks pass and the board authorizes recovery.

### When the TOE enters maintenance mode

The election event enters maintenance mode, and the tab shows the reason, in these
cases:

| Cause | Reason on the tab |
|---|---|
| A self-test failed, other than the two results of [An unrecoverable failure](#an-unrecoverable-failure). This includes a difference from the approved baseline, in the election data or the access control: restore the exact approved values and verify again | "Self-test failed" |
| A required self-test is missing or its validity has expired | "The required self-test failed or its validity interval expired" |
| A required service of the TOE stops or restarts | "A required service stopped or restarted" |
| A required service of the TOE sends no heartbeat for 45 seconds. A late heartbeat cannot silently reopen the election event | "A required service heartbeat expired" |
| The external monitoring of requests is unavailable or out of date | "Required external request monitoring is unavailable or its observation expired" |
| A ballot box is not empty at the start of the election | "Ballot box not empty at the start of the election" |
| The check of the TOE software finds a difference ([Verifying the TOE software](07-execution.md#verifying-the-toe-software)) | "TOE software differs from the accepted delivery" |

### What the TOE does in maintenance mode

During the execution phase the TOE refuses voter operations and user access to ballot
boxes, pauses active voting and records `MaintenanceModeEntered`. It preserves
the electoral phase and stored data. It emails the board members and the
administrators ([Email notifications](#email-notifications)). In the area "Security
email delivery" of the tab, check whether mail
was accepted by the transport, failed or has an unknown delivery outcome.

### Returning to a secure state

To return the election event to a secure state:

1. Write the reason and the time of the failed run in the board's minutes, and keep
   the evidence.
2. Have the administrators and the operators investigate and correct the cause, as
   the Deployment Guide describes. Do not alter ballots or logs to make a test pass.
   If a server component restarted, run the check of
   [Verifying the TOE software](07-execution.md#verifying-the-toe-software): the
   self-tests do not cover it.
3. On the election event's tab "Security and self-tests", select **Before resuming
   voting**, then **Run self-tests**. Wait for all six current results to show
   **Passed**.
4. Decide whether the electoral regulation permits continuation within the
   remaining approved voting period.
5. Select the individual election, open its tab "Publish", select the button "Start
   Voting" and then "Start Online Voting", and obtain the required fresh board authorizations.
   This resumes that paused channel. A passing test by itself never resumes voting.
6. Check the state "Operational", the online status "Open" of both the election
   event and the resumed election, the record `MaintenanceModeLeft` on the tab "Logs"
   and the result of the automatic run with the purpose "After maintenance". Other
   paused elections remain paused. A new failure blocks access again.

### Maintenance mode at the start

A new election event shows the state "Recovery required", with the reason "Startup
self-tests are required", until voting opens for the first time. Before a first
opening with the button "Start Voting", use **Before starting voting**. A scheduled
start opens voting only from this initial state: after a failure, in maintenance
mode or during a board pause the TOE refuses it, and voting reopens only through
the election board. The
start time, initialization report and empty ballot boxes remain required. The
normal initialization tally of an empty box does not prevent opening; an actual
results tally or existing ballot stops the start, and the election event enters
maintenance mode with the reason "Ballot box not empty at the start of the election".

## An unrecoverable failure

### When an unrecoverable failure happens

Two self-test results give the state "Unrecoverable failure" on the election event's
tab "Security and self-tests": a ballot-box integrity mismatch and a verified
integrity finding in the electoral log. Read the specific failed check and its
details.

### What the TOE does after an unrecoverable failure

During the execution phase, voter operations and user ballot-box access remain blocked.
The failure is durable: changing settings, restarting services or obtaining a
later passing result cannot restore the operational state. The TOE records the
failure and emails the board members and the administrators
([Email notifications](#email-notifications)).

### What the election board does after an unrecoverable failure

Keep the evidence and have the operators investigate. Do not repair or remove
ballots, audit entries or failure records. Terminate the election execution
([Terminating the election execution](07-execution.md#terminating-the-election-execution)).
In the evaluation phase, the board may select the button "Start post-processing" on
the election event's tab "Security and self-tests" to export investigation data
even when no result could be determined. This does not declare the election
result valid or permit voting to restart.

## The audit storage restriction

The TOE measures the used capacity of the audit storage every 30 seconds. Above
80 %, or when it has no current measurement:

- it refuses new sessions and requests of voters, so voters cannot log in or
  continue;
- it writes the record `AuditStorageThresholdExceeded`, or
  `AuditStorageObservationUnavailable`, and emails the election board and the
  administrators ([Email notifications](#email-notifications));
- the election board and the administrators can continue to work.

The election board:

1. records the time of the email in the board's minutes;
2. has the administrators and the operators provide more storage at once; when
   fresh measurements show sufficient space, it checks the election event's security
   state and completes board recovery if it shows "Recovery required";
3. decides under the electoral regulation whether to interrupt the election
   execution with the button "Pause Voting" until the storage is extended, and whether the
   remaining election period is enough for every voter to vote.

## Email notifications

The TOE emails every enabled account, except service accounts, that has an email
address and holds the permission "Edit Tenant" (`tenant-write`) or "Edit Election
State" (`election-state-write`): that is, the administrators and the board
members. It sends an email when:

- an election event enters maintenance mode or has an unrecoverable failure, with
  the reason;
- an election event leaves maintenance mode;
- the electoral phase of an election event changes;
- election execution data is exported;
- an election event is archived;
- the audit storage threshold is exceeded, or the TOE has no current measurement
  of the audit storage.

The TOE sends no email for an attack detection or for an account reaching the
authentication failure threshold. The election board sees these on the tab
"Dashboard" of the election event and in the electoral log.

The TOE uses the protected relay configuration supplied during installation. The
area "Security email delivery" of the tab "Security and self-tests" of the
election event shows queued, sent, failed and uncertain outcomes. That the mail
relay accepted a message does not prove that the recipient read it.

## A ballot box is not sealed

At the termination, the TOE seals each ballot box. The switch "Show ballot boxes" on
the election event's tab "Dashboard" shows the status of each one. This section is the runbook for a failed
seal that the admin portal refers to.

| Status | Meaning | What to do |
|---|---|---|
| "Sealing at" a time | The ballot box is sealed at that time | Wait. |
| "Sealing now" | The ballot box is being sealed, which takes up to a minute | Wait. |
| "Sealed, publishing" | The ballot box is locked and its seal is on the bulletin board; its seal record is being published | Wait. If it stays for more than a few minutes, have the operators check the TOE. The ballot box is sealed already. |
| "Sealed" | The ballot box is locked and its seal record is published | Nothing. |
| "Sealing overdue" | The seal is late; the reason is shown under the status | See [Sealing overdue](#sealing-overdue). |
| "Not sealed: incident" | The ballot box could not be sealed | See [Not sealed: incident](#not-sealed-incident). |

### Sealing overdue

No ballot is accepted after the seal deadline, but the ballot box is not
sealed yet. Read the reason under the status:

- "… is still enabled and not closed: stop it to seal the ballot box.", or a
  similar reason naming a voting channel that is not closed: stop that channel
  with the button "Stop Voting" on the election event's tab "Publish".
- "… ballots aren't on the bulletin board yet …": wait; if it does not change,
  have the operators check the TOE.
- "The last attempt couldn't …; it is retried every minute.", "Last tried at …"
  or "Not tried yet": have the operators check the TOE. The next attempt
  after the cause is removed seals the ballot box.

Record in the board's minutes how long each ballot box was overdue and why.

### Not sealed: incident

**What you see.** The row of the ballot box shows the status "Not sealed: incident"
with a reason, and the card says that a ballot box could not be sealed, that it
stays locked and that the incident is in the logs. The election event's tabs
"Dashboard" and "Publish" show the banner "This is an incident: each of these ballot
boxes stays locked and can't be tallied. Follow the runbook for a failed seal." The
tab "Logs" has a record `BallotBoxSealFailed` at level ERROR. The TOE sends no email
for it: watch the tab "Dashboard" after every termination.

**Why it happens.**

| Reason | Meaning |
|---|---|
| "A ballot does not match its Ballot ID." | A stored ballot was altered or corrupted after it was cast. |
| "A ballot has no content or no Ballot ID." | A stored ballot is incomplete. |
| "A ballot has an unknown voting channel." | A stored ballot names a channel that does not exist. The stored data was altered or corrupted. |
| "A ballot has no cast-vote record on the bulletin board." | The electoral log lacks the record of a stored ballot. The log lost a record or was altered. |
| "The election event has no bulletin board." | There is nowhere to post the seal. |
| "A seal for this ballot box is already on the bulletin board." | A seal was removed from the TOE's stored data after it was made. |

**What the TOE does.** The ballot box stays locked: no ballot can be added,
changed or deleted, so the evidence is preserved. The failure is final for that
ballot box: there is no retry and no way to unseal it. The election cannot be
counted, because counting needs every ballot box of the election sealed. Other
ballot boxes and elections are not affected.

**What to do.** To handle a failed seal:

1. Ask the operators whether a database of the TOE was restored or
   switched over around the time of the seal. A restore of two databases to
   different points in time can look like tampering. Write the answer in the board's
   minutes.
2. Do not change the data. Nobody edits ballots, seals or databases to make the
   seal pass.
3. Write the evidence in the board's minutes: the election, the area, the reason
   and the time from the card. Keep the record `BallotBoxSealFailed` with them: export
   it with the button "Export" on the tab "Logs", which asks for the period, the time
   zone and the format, CSV or PDF. Have the operators keep a backup of the TOE's
   databases as they are.
4. Have the operators investigate the cause, and report it as a security
   problem (see
   [Reporting a security problem](02-organizer-duties.md#reporting-a-security-problem)).
5. Decide under the electoral regulation what happens to the election, for
   example that it is held again. The TOE offers no way around a failed seal.

If the seal record of a sealed ballot box cannot be opened and the admin portal
shows the message "The seal record document is missing: report it as an incident.", proceed
in the same way.

## A restore from a backup

A restore replaces the election data, the election execution data and the
electoral log with the older state of a backup; the operators make backups
only outside the execution phase. The operators restore a backup only after the
election board has decided so under the electoral regulation, and they tell the
administrators and the election board of every restore. The deployment helper
refuses a restore while an election event is in the execution phase, and one
that would change an electoral phase; for a recovery in the execution phase, the
operators ask Sequent
([Restore an authorized backup on the existing cluster](../../05-reference/06-deployment_guide/11-selftest_handover.md#restore-an-authorized-backup-on-the-existing-cluster)).

After every restore, the election board checks the tab "Logs" of each election
event concerned against the board's minutes and records the restore in them.

## A signing request that does not complete

A security-critical action runs only when its signing request is complete.

| Status | What happened | What to do |
|---|---|---|
| "Expired" | The required authorisations were not given in time | The action did not run. Start it again if the board still wants it. |
| "Cancelled" | A board member aborted it, or what it signs or its rule changed | The action did not run. Check why; start it again only if the board still wants it. |
| "Failed" | All authorisations were given, but the action failed | Read the reason on the election event's tab "Logs". Treat it as a security-relevant event; start the action again only when the cause is clear. |

If the required authorisations cannot be reached because board members are
absent or their certificates were revoked, the election organizer appoints
replacements (see
[When a person joins, changes role or leaves](03-board-roles-and-permissions.md#when-a-person-joins-changes-role-or-leaves)).

## When the admin portal is not available

- After 15 minutes without interaction the session ends. Sign in again.
- If the admin portal cannot be reached during the execution phase, the board
  can neither monitor nor interrupt the election execution. Call the operators
  at once and write the time in the board's minutes. When the admin portal is back,
  check the election event's tabs "Dashboard", "Tasks" and "Logs" for the time it was
  not available.

## Operational errors

| Error | Consequence | What to do |
|---|---|---|
| A wrong file was chosen at the import | The TOE refuses it, unless it is a validly signed package with the required approvals | Check the import record `ConfigurationPackageImported` against the configuration and revision the board approved. Use only the file and digest that the election organizer gave; never import without the integrity check. |
| A validly signed and approved package with wrong data was imported, or election data was changed by hand | The election event holds data that the board did not decide | Do not approve its configuration version. The election organizer issues a corrected package as a new revision, the board approves it, a board member imports it as a new election event, and the board archives the wrong one in the preparation phase (see [Importing the election data](06-preparation.md#importing-the-election-data)). |
| A security-critical action was started by mistake | A signing request waits for authorisations | Abort it with the button "Cancel request" (see [Aborting](04-secure-use.md#aborting)). |
| The election execution was interrupted by mistake | Voters cannot vote | Resume it as in [Resuming the election execution](07-execution.md#resuming-the-election-execution). |
| The election execution was terminated too early | Voting cannot start again in that election | Write it in the board's minutes; decide under the electoral regulation. |
| The export was made without the option "Encrypt with Password", or its password was kept with the file | The election execution data may be readable by others | Destroy the unprotected file, make a new protected export, treat it as a security-relevant event. |
| An election event was deleted | The election execution data in the TOE is gone | Only the exported copies remain. Keep them as the electoral regulation requires. The TOE refuses every deletion unless the election execution data has been exported and the required authorisations are reached; only the election board holds the permission "Delete Election Event" (`election-event-delete`). The deletion also removed the election event's audit records in the TOE. |
| A session was left open on an unattended workstation | Anyone at the workstation can act with that member's permissions | Log out at once; check the election event's tabs "Logs" and "Signatures" for actions in the meantime. |
