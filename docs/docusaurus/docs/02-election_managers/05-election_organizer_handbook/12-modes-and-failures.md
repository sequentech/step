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

| Mode | When | Voters | Election board | How it shows |
|---|---|---|---|---|
| Preparation | From the import until the start of the election execution | Cannot open an election action | Approves the configuration packages and imports them, checks the import and the settings, publishes and approves the configuration version, sets the lockdown period, runs the key ceremony and the initialization report | **Security and self-tests: Preparation** |
| Normal operation | From the start until the termination of the election execution | Vote, check and audit ballots | Monitors, reviews integrity audits, interrupts, terminates | **Election execution**, **Operational**; elections open |
| Interrupted | After "Pause Voting" | Cannot open or continue an election action | Investigates, reviews integrity audits, resumes or terminates | The elections are "Paused"; "Voting is paused." |
| Maintenance mode | Recoverable failure, expired checks, or a required service stopping or restarting | Election requests are refused; ballot-box access is blocked during execution | Investigates and authorizes recovery after fresh tests | **Security and self-tests: Recovery required**, with the reason |
| Unrecoverable failure | An unrecoverable integrity failure is recorded | Election requests and execution-phase ballot-box access remain blocked | Preserves evidence, terminates and exports for investigation | **Security and self-tests: Unrecoverable failure** |
| Audit storage restriction | The storage of the audit data is more than 80 % full (see [The audit storage restriction](#the-audit-storage-restriction)) | Cannot log in; their requests are refused | Continues to work | Email; `AuditStorageThresholdExceeded` |
| Evaluation | After the termination | Cannot open an election action | Checks the audit records, counts the votes | **Security and self-tests: Evaluation**; ballot boxes "Sealed" |
| Ballot box not sealed | A ballot box could not be sealed at the termination (see [A ballot box is not sealed](#a-ballot-box-is-not-sealed)) | Cannot open an election action | Records the evidence, decides; that election cannot be counted | "Not sealed: incident" |
| Post-processing | After "Start post-processing" | Cannot open an election action | Reads, exports, publishes the verification data, deletes | **Security and self-tests: Post-processing** |

The phases follow each other in this order only. The TOE never returns an
election event to an earlier phase.

## Maintenance mode

Read the current state in **Security and self-tests**. **Recovery required**
means that the election cannot continue until the cause has been removed,
current checks pass and the board authorizes recovery.

### When the TOE enters maintenance mode

A recoverable self-test failure, a missing or expired current test, or a required
backend service stopping or restarting prevents continued voter access.
A missing heartbeat becomes a refusal when its 45-second validity expires;
a late heartbeat cannot silently reopen the election. An orderly shutdown
removes admission before attempting the per-event state updates.

### What the TOE does in maintenance mode

During execution the TOE refuses voter operations and user access to ballot
boxes, pauses active voting and records `MaintenanceModeEntered`. It preserves
the electoral phase and stored data. It queues a notice to authorized board
members and administrators. In **Security email delivery**, check whether mail
was accepted by the transport, failed or has an unknown delivery outcome.

### Returning to a secure state

1. Record the reason, time and self-test identifier. Preserve the electoral log
   and operational evidence.
2. Have the administrator and operator investigate and correct the cause using
   AGD-04. Do not alter ballots or logs to make a test pass.
3. In **Security and self-tests**, select **Before resuming voting**, then
   **Run self-tests**. Wait for all six current results to show **Passed**.
4. Decide whether the electoral regulation permits continuation within the
   remaining approved voting period.
5. Select the individual election (Post), then **Publish → Start Voting →
   Start Online Voting**, and obtain the required fresh board authorizations.
   This resumes that paused channel. A passing test by itself never resumes voting.
6. Check **Operational**, **Open** for both the event and the resumed Post,
   `MaintenanceModeLeft` and the automatic **After maintenance** result.
   Other paused Posts remain paused. Verify the resumed Post is available in
   the voter interface. A new failure blocks access again.

### Maintenance mode at the start

A new event starts with **Recovery required** until its opening checks and
board-authorized or scheduled opening complete. Before a manual first opening,
use **Before starting voting**. The start time, initialization report and empty
ballot boxes remain required. The normal initialization tally of an empty box
does not prevent opening; an actual results tally or existing ballot does.

## An unrecoverable failure

### When an unrecoverable failure happens

An unrecoverable ballot-box or related integrity failure produces
**Unrecoverable failure**. Read the specific failed check and its details.

### What the TOE does after an unrecoverable failure

During execution, voter operations and user ballot-box access remain blocked.
The failure is durable: changing settings, restarting services or obtaining a
later passing result cannot restore the operational state. The TOE records the
failure and queues a security notice.

### What the election board does after an unrecoverable failure

Preserve the evidence and have the operator investigate. Do not repair or remove
ballots, audit entries or failure records. Terminate the execution under the
required board procedure. In evaluation, the board may select **Start
post-processing** in **Security and self-tests** to export investigation data
even when no result could be determined. This does not declare the election
result valid or permit voting to restart.

## The audit storage restriction

The storage monitor checks capacity regularly; the backend refreshes its
admission state every 15 seconds. Above 80 %, or when a current trustworthy
capacity observation is unavailable:

- it refuses new sessions and requests of voters, so voters cannot log in or
  continue;
- it writes the record `AuditStorageThresholdExceeded` and emails the election
  board and the administrators;
- the election board and the administrators can continue to work.

The election board:

1. records the time of the email in the minutes;
2. has the administrators and the operators provide more storage at once; when
   fresh observations show sufficient space, check the election safety state
   and complete board recovery if it shows **Recovery required**;
3. decides under the electoral regulation whether to interrupt the election
   execution with "Pause Voting" until the storage is extended, and whether the
   remaining election period is enough for every voter to vote.

## A ballot box is not sealed

At the termination, the TOE seals each ballot box. "Show ballot boxes" on the
Dashboard shows the state of each one. This section is the runbook for a failed
seal that the admin portal refers to.

| Status | Meaning | What to do |
|---|---|---|
| "Sealing at" a time | The ballot box is sealed at that time | Wait. |
| "Sealing now" | "Being sealed: this takes up to a minute." | Wait. |
| "Sealed, publishing" | "The ballot box is locked and its seal is on the bulletin board. Its seal record is being published." | Wait. If it stays for more than a few minutes, have the operators check the platform. The ballot box is sealed already. |
| "Sealed" | The ballot box is locked and its seal record is published | Nothing. |
| "Sealing overdue" | The seal is late; the reason is shown under the status | See [Sealing overdue](#sealing-overdue). |
| "Not sealed: incident" | The ballot box could not be sealed | See [Not sealed: incident](#not-sealed-incident). |

### Sealing overdue

No ballot is accepted after the seal deadline, but the ballot box is not
sealed yet. Read the reason under the status:

- "… is still enabled and not closed: stop it to seal the ballot box.", or a
  similar reason naming a voting channel that is not closed: stop that channel
  with "Stop Voting" on the Publish tab.
- "… ballots aren't on the bulletin board yet …": wait; if it does not change,
  have the operators check the platform.
- "The last attempt couldn't …; it is retried every minute.", "Last tried at …"
  or "Not tried yet": have the operators check the platform. The next attempt
  after the cause is removed seals the ballot box.

Record in the minutes how long each ballot box was overdue and why.

### Not sealed: incident

**What you see.** The ballot box row says "Not sealed: incident" with a reason,
and the card says "A ballot box could not be sealed: it stays locked, and the
incident is in the logs." The Dashboard and the Publish tab show the banner
"This is an incident: each of these ballot boxes stays locked and can't be
tallied. Follow the runbook for a failed seal." The Logs tab has a
`BallotBoxSealFailed` record at level ERROR. The TOE sends no email for it:
watch the Dashboard after every termination.

**Why it happens.**

| Reason | Meaning |
|---|---|
| "A ballot does not match its Ballot ID." | A stored ballot was altered or corrupted after it was cast. |
| "A ballot has no content or no Ballot ID." | A stored ballot is incomplete. |
| "A ballot has an unknown voting channel." | A stored ballot names a channel that does not exist. The stored data was altered or corrupted. |
| "A ballot has no cast-vote record on the bulletin board." | The electoral log lacks the record of a stored ballot. The log lost a record or was altered. |
| "The election event has no bulletin board." | There is nowhere to post the seal. |
| "A seal for this ballot box is already on the bulletin board." | A seal was removed from the database after it was made. |

**What the TOE does.** The ballot box stays locked: no ballot can be added,
changed or deleted, so the evidence is preserved. The failure is final for that
ballot box: there is no retry and no way to unseal it. The election cannot be
counted, because counting needs every ballot box of the election sealed. Other
ballot boxes and elections are not affected.

**What to do.**

1. Ask the operators whether a database of the platform was restored or
   switched over around the time of the seal. A restore of two databases to
   different points in time can look like tampering. Record the answer.
2. Do not change the data. Nobody edits ballots, seals or databases to make the
   seal pass.
3. Record the evidence: the election, the area, the reason and the time from
   the card, and the `BallotBoxSealFailed` record (export it with "Export in
   CSV" or "Export in PDF"). Have the operators keep a backup of the platform's
   databases as they are.
4. Have the operators investigate the cause, and report it as a security
   problem (see
   [Reporting a security problem](02-organizer-duties.md#reporting-a-security-problem)).
5. Decide under the electoral regulation what happens to the election, for
   example that it is held again. The TOE offers no way around a failed seal.

If the seal record of a sealed ballot box cannot be opened and the admin portal
says "The seal record document is missing: report it as an incident.", proceed
in the same way.

## A restore from a backup

A restore replaces the election data, the election execution data and the
electoral log with the older state of a backup; the operators make backups
only outside the execution phase. The operators restore a backup outside the
execution phase, and in it only after the election board has decided so under
the electoral regulation, and they tell the administrators and the election
board of every restore.

After every restore, the election board checks the Logs tab of each election
event concerned against its minutes and records the restore in them. A restore
in the execution phase removes the votes cast since the backup: decide under
the electoral regulation what happens to the election.

## A signing request that does not complete

A security-critical action runs only when its signing request is complete.

| Status | What happened | What to do |
|---|---|---|
| "Expired" | The required authorisations were not given in time | The action did not run. Start it again if the board still wants it. |
| "Cancelled" | A board member aborted it, or what it signs or its rule changed | The action did not run. Check why; start it again only if the board still wants it. |
| "Failed" | All authorisations were given, but the action failed | Read the reason on the Logs tab. Treat it as a security-relevant event; start the action again only when the cause is clear. |

If the required authorisations cannot be reached because board members are
absent or their certificates were revoked, the election organizer appoints
replacements (see
[When a person joins, changes role or leaves](03-board-roles-and-permissions.md#when-a-person-joins-changes-role-or-leaves)).

## When the admin portal is not available

- After 15 minutes without interaction the session ends. Sign in again.
- If the admin portal cannot be reached during the execution phase, the board
  can neither monitor nor interrupt the election execution. Call the operators
  at once and record the time. When the admin portal is back, check the
  Dashboard, the Tasks and Logs tabs for the time it was not
  available.

## Operational errors

| Error | Consequence | What to do |
|---|---|---|
| A wrong file was chosen at the import | The TOE refuses it, unless it is a validly signed package with the required approvals | Check the import record `ConfigurationPackageImported` against the configuration and revision the board approved. Use only the file and digest that the election organizer gave; never import without the integrity check. |
| A validly signed and approved package with wrong data was imported, or election data was changed by hand | The election event holds data that the board did not decide | Do not approve its configuration version. The election organizer issues a corrected package as a new revision, the board approves it, a board member imports it as a new election event, and the board archives the wrong one in the preparation phase (see [Importing the election data](06-preparation.md#importing-the-election-data)). |
| A security-critical action was started by mistake | A signing request waits for authorisations | Abort it with "Cancel request" (see [Aborting](04-secure-use.md#aborting)). |
| The election execution was interrupted by mistake | Voters cannot vote | Resume it as in [Resuming the election execution](07-execution.md#resuming-the-election-execution). |
| The election execution was terminated too early | Voting cannot start again in that election | Record it; decide under the electoral regulation. |
| The export was made without "Encrypt with Password", or its password was kept with the file | The election execution data may be readable by others | Destroy the unprotected file, make a new protected export, treat it as a security-relevant event. |
| An election event was deleted | The election execution data in the TOE is gone | Only the exported copies remain. Keep them as the electoral regulation requires. The TOE refuses every deletion unless the election execution data has been exported and the required authorisations are reached; only the election board holds "Delete Election Event". The deletion also removed the election event's audit records in the TOE. |
| A session was left open on an unattended workstation | Anyone at the workstation can act with that member's permissions | Log out at once; check the Logs tab and the Signatures tab for actions in the meantime. |
