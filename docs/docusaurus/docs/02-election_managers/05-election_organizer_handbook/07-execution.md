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

Voting opens only within the approved election period, after initialization,
empty-box checks and the six required security checks. Open the event's
**Security and self-tests** tab to review the current phase, safety state and
**Recent self-test runs**. A queued task is not a passed test.
See [Self-Test Settings](../04-administrator_handbook/07-self_tests.md).

**At the scheduled start.** The election dates of the configuration package
contain the scheduled event "Start Voting Period" for the election period
start time. When it is reached, the TOE requests the scheduled-start tests. It opens voting
after they pass and the other opening prerequisites hold. If a test fails, the
board must investigate and authorize recovery; a later pass does not silently
resume the election.

**With "Start Voting".** If the electoral regulation has the election board
open voting itself:

1. In **Security and self-tests**, select **Before starting voting** under
   **Purpose of the test**, then **Run self-tests**. Wait for all six checks
   to show **Passed**. At or after the approved start time, open the
   Publish tab and select "Start Voting".
2. Read the "Confirm Action" dialog ("You are about to start voting period.
   Are you sure you want to continue?") and select "Confirm". If the admin
   portal sends you to the external authentication server, log in there again
   (see [Confirming an action by authenticating again](04-secure-use.md#confirming-an-action-by-authenticating-again)).
3. The TOE creates a signing request for "Open voting". Each required board
   member authenticates again at the external authentication server and
   signs.
4. Inspect the voting status and the current safety state. If the request
   says another test is needed, let that task finish, review all six results
   and authorize a fresh opening request. A missing or expired test cannot
   be bypassed with an earlier signature.

Opening checks are short-lived and bound to the current election state.
Changing that state or waiting more than five minutes requires a fresh check.

After the start, check that:

- **Security and self-tests** shows **Election execution** and **Operational**,
  and the Publish tab shows the elections as open;
- the Logs tab has the opening record (`ElectionVotingPeriodOpen` or
  `ElectionEventVotingPeriodOpen`) with the time and the board members who
  authorised it;
- all six current self-test results show **Passed** and their completion time
  is within the configured interval.

If the election event enters maintenance mode instead, follow
[Maintenance mode](12-modes-and-failures.md#maintenance-mode).

## Monitoring the election

At least one board member watches the election throughout the execution
phase, and checks the following at least once in every configured self-test interval:

| Where | What to look at | Act when |
|---|---|---|
| Dashboard and Publish | Election voting status, turnout and ballot boxes | An unexpected state, count or seal; use **Security and self-tests** for the authoritative phase and recovery reason |
| **Security and self-tests** | Current phase, safety state, six results and their times | **Recovery required**, **Unrecoverable failure**, any failed, missing or expired result, or a status-read error |
| Monitoring tab, "Attack detections" | Brute-force attempts on accounts and denial-of-service attempts on the external interfaces | Any detection |
| Logs tab | The audit records of the election event (see [Security-Relevant Events for the Election Board](11-security-relevant-events.md)) | Any record of a kind listed there as security-relevant |
| Email | Notices from the TOE to the election board, for example on maintenance mode or on the audit storage threshold | Any notice |

The events to act on, with what to do, are in
[Security-Relevant Events for the Election Board](11-security-relevant-events.md).

## Self-test results

Expand the latest entry under **Recent self-test runs** in **Security and
self-tests**. Check its purpose, completion time and all six named results:
ballot-box integrity, approved election data, audit recording and integrity,
trusted HTTPS channels, approved access-control configuration and clock
synchronization. Each row includes the result and details.

**Earlier election state** means that an old result cannot authorize the current
transition. **Queued or running** and **Not completed** are not successful
results. If the page cannot refresh its status, do not act on old displayed data.

## Running a self-test on request

1. Open **Security and self-tests**.
2. Select **On request** under **Purpose of the test**.
3. Select **Run self-tests** once. The request queues background work; you may
   leave the page while it runs.
4. Return and use **Refresh status**. Review the six completed results and their
   details. Investigate any failure before deciding how to continue.

**On request** checks do not replace the purpose-specific checks required before
starting, resuming or, when configured, counting. The separate **Logs → Audit**
operation remains available for a detailed electoral-log audit.

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
**Security and self-tests** shows **Recovery required** with the reason.
The TOE records the transition and queues email to the election board and
administrators. Check **Security email delivery**; a queued message does not
prove it has arrived. What to do is in
[Maintenance mode](12-modes-and-failures.md#maintenance-mode).

## Resuming the election execution

The election execution is resumed only by the election board, and only when:

- the cause of the interruption or of maintenance mode has been found and
  removed;
- the resumption is still within the time frame that the electoral regulation
  sets for continuing an interrupted election;
- the board has decided to resume under the electoral regulation.

Steps:

1. Review the reason and investigation record. In **Security and self-tests**,
   select **Before resuming voting** and **Run self-tests**. Wait for all six
   results to show **Passed** for the current state.
2. Select "Resume Voting" and confirm, as for "Start Voting".
3. The TOE creates a signing request for "Open voting". Each required board
   member authenticates again at the external authentication server and
   signs.
4. Check **Election execution**, **Operational** and the resumed voting
   status. Review `MaintenanceModeLeft` when leaving recovery. A further
   **After maintenance** suite runs automatically; investigate any failure.

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
