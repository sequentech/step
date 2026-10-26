---
id: execution
title: Execution Phase Procedures
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

In the execution phase the voters cast their votes. The election board starts
the election execution, monitors it, interrupts and resumes it when needed, and
terminates it.

## Starting the election execution

Voting can be opened by a scheduled task or a board request. The complete
ST self-test and automatic start/resume failure gates are not implemented.
The actions below must not be treated as evidence that those gates ran.
See [Self-Test Settings](../04-administrator_handbook/07-self_tests.md).

**At the scheduled start.** The election dates of the configuration package
contain the scheduled event "Start Voting Period" for the election period
start time. When it is reached, the TOE opens voting by itself.

**With "Start Voting".** If the electoral regulation has the election board
open voting itself:

1. At or after the election period start time, open the election event's
   Publish tab and select "Start Voting".
2. Read the "Confirm Action" dialog ("You are about to start voting period.
   Are you sure you want to continue?") and select "Confirm". If the admin
   portal sends you to the external authentication server, log in there again
   (see [Confirming an action by authenticating again](04-secure-use.md#confirming-an-action-by-authenticating-again)).
3. The TOE creates a signing request for "Open voting". Each required board
   member authenticates again at the external authentication server and
   signs.
4. Inspect the resulting voting status. No complete ST self-test result is produced by this action.

The required self-test and maintenance safeguards must be completed before
this source can establish the evaluated start-of-election behavior. A green
log audit or an initialization report alone does not satisfy that requirement.

After the start, check that:

- the Dashboard shows "Electoral phase" Execution and the elections as open;
- the Logs tab has the opening record (`ElectionVotingPeriodOpen` or
  `ElectionEventVotingPeriodOpen`) with the time and the board members who
  authorised it;
- the board has recorded the available integrity audit result and the unresolved complete self-test requirement.

If the election event enters maintenance mode instead, follow
[Maintenance mode](12-modes-and-failures.md#maintenance-mode).

## Monitoring the election

At least one board member watches the election throughout the execution
phase, and checks the following at least once in every configured background audit interval:

| Where | What to look at | Act when |
|---|---|---|
| Dashboard | "Electoral phase", the state of each election, the election execution data that the Dashboard shows the board (turnout and cast votes), the ballot boxes ("Show ballot boxes") | The Dashboard shows "Maintenance mode", or an election is in a state the board did not decide |
| **Tasks** | The periodic **Audit Electoral Log** task completed without findings | A failed, delayed or missing task; this is not a six-check self-test result |
| Monitoring tab, "Attack detections" | Brute-force attempts on accounts and denial-of-service attempts on the external interfaces | Any detection |
| Logs tab | The audit records of the election event (see [Security-Relevant Events for the Election Board](11-security-relevant-events.md)) | Any record of a kind listed there as security-relevant |
| Email | Notices from the TOE to the election board, for example on maintenance mode or on the audit storage threshold | Any notice |

The events to act on, with what to do, are in
[Security-Relevant Events for the Election Board](11-security-relevant-events.md).

## Self-test results

The implemented result is **Audit Electoral Log** in **Tasks**. Success means
the stored log and signed checkpoint checks completed without findings.
Failure includes a detected inconsistency, an operational error or a timeout.
The complete six-check ST suite is not implemented; there are no per-test
results or automatic maintenance transitions to review.

## Running a self-test on request

Use **Logs → Audit** for the implemented background integrity check. The
request returns promptly; follow its task identifier in **Tasks**. You may
leave the page while it runs. Avoid repeated requests while an audit is active.

[Self-Test Settings](../04-administrator_handbook/07-self_tests.md) describes
the actual scope, interval, timeout, failure response and missing ST checks.
This operation must not be recorded as completion of the full self-test suite.

## Interrupting the election execution

The election board interrupts the election execution when the electoral
regulation calls for it, for example because of a suspected attack or a
problem reported by voters.

1. On the Publish tab, select "Pause Voting".
2. Read "You are about to pause voting period. Are you sure you want to
   continue?" and select "Confirm". If the admin portal sends you to the
   external authentication server, log in there again.
3. Voting is suspended: no voter can open or continue an election action. The
   Logs tab shows the interruption record (`ElectionVotingPeriodPause` or
   `ElectionEventVotingPeriodPause`).

The election event stays in the execution phase. Record the reason and the
time in the board's minutes.

## Maintenance mode

The TOE puts the election event into maintenance mode by itself when it finds a
recoverable failure during the execution phase. Voting is then suspended, the
Dashboard shows "Maintenance mode" with the reason, and the election board and
the administrators receive an email. What to do is in
[Maintenance mode](12-modes-and-failures.md#maintenance-mode).

## Resuming the election execution

The election execution is resumed only by the election board, and only when:

- the cause of the interruption or of maintenance mode has been found and
  removed;
- the resumption is still within the time frame that the electoral regulation
  sets for continuing an interrupted election;
- the board has decided to resume under the electoral regulation.

Steps:

1. Review the available **Audit Electoral Log** task and the investigation record. The required complete self-test and automatic resume gate remain unimplemented.
2. Select "Resume Voting" and confirm, as for "Start Voting".
3. The TOE creates a signing request for "Open voting". Each required board
   member authenticates again at the external authentication server and
   signs.
4. Inspect the resulting voting status. This action does not produce a complete ST self-test result or a `MaintenanceModeLeft` event.

If the time frame for continuing has passed, do not resume: terminate the
election execution and decide under the electoral regulation how the
election continues.

## Terminating the election execution

The election execution ends at the end of election time, or earlier when the
election board terminates it.

**At the scheduled end.** The election dates contain the scheduled event "End
Voting Period" for the end of election time. When it is reached, the TOE closes
voting by itself. The Dashboard then says "Closed by the scheduled close of
polls."

**With "Stop Voting".** Termination is a security-critical action:

1. On the Publish tab, select "Stop Voting".
2. The TOE shows the confirmation notice "Confirm Action", for example: "You
   are about to stop voting in every election. Their ballot boxes are then
   sealed: no ballot can be added, changed or deleted, and voting cannot start
   again. Are you sure you want to continue?". Read it and select "Confirm".
   If the admin portal sends you to the external authentication server, log
   in there again.
3. The TOE creates a signing request for "Close voting". Each required board
   member authenticates again at the external authentication server and
   signs.
4. The TOE closes voting. The Dashboard says "Closed by" with the names of the
   signers and the signing code.

In both cases the TOE then seals the ballot boxes. Open "Show ballot boxes" on
the Dashboard: each ballot box goes from "Sealing now" to "Sealed, publishing"
and then "Sealed". The election event is now in the evaluation phase.

:::warning
- Termination cannot be undone: voting cannot start again in a closed
  election. Check before you confirm that the end of election time has been
  reached or that the electoral regulation calls for the termination.
- If a ballot box shows "Not sealed: incident" or stays at "Sealing overdue",
  follow [A ballot box is not sealed](12-modes-and-failures.md#a-ballot-box-is-not-sealed).
:::
