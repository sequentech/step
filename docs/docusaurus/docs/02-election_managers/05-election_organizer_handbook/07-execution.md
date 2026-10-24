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

The election execution starts in one of two ways. In both, the TOE first runs
the self-test and checks that the intermediate ballot box and the ballot box
are empty, and it opens voting only if both succeed (see
[Conditions for the start of the execution phase](05-electoral-phases.md#conditions-for-the-start-of-the-execution-phase)).

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
4. The TOE runs the self-test and the empty-box check, then opens voting.

The TOE refuses "Start Voting" before the election period start time, before
the initialization report of the election exists, and when the self-test or the
empty-box check fails.

After the start, check that:

- the Dashboard shows "Electoral phase" Execution and the elections as open;
- the Logs tab has the opening record (`ElectionVotingPeriodOpen` or
  `ElectionEventVotingPeriodOpen`) with the time and the board members who
  authorised it;
- the "Self-tests" tab shows the successful self-test of the start.

If the election event enters maintenance mode instead, follow
[Maintenance mode](12-modes-and-failures.md#maintenance-mode).

## Monitoring the election

At least one board member watches the election throughout the execution
phase, and checks the following at least once in every self-test interval:

| Where | What to look at | Act when |
|---|---|---|
| Dashboard | "Electoral phase", the state of each election, the election execution data that the Dashboard shows the board (turnout and cast votes), the ballot boxes ("Show ballot boxes") | The Dashboard shows "Maintenance mode", or an election is in a state the board did not decide |
| "Self-tests" tab | The regular self-tests: one run per interval, every test passed | A run failed, or no run appears for longer than the interval |
| Monitoring tab, "Attack detections" | Brute-force attempts on accounts and denial-of-service attempts on the external interfaces | Any detection |
| Logs tab | The audit records of the election event (see [Security-Relevant Events for the Election Board](11-security-relevant-events.md)) | Any record of a kind listed there as security-relevant |
| Email | Notices from the TOE to the election board, for example on maintenance mode or on the audit storage threshold | Any notice |

The events to act on, with what to do, are in
[Security-Relevant Events for the Election Board](11-security-relevant-events.md).

## Self-test results

The self-test checks six things. The "Self-tests" tab shows, for each run, the
time, what triggered it and the result of each test. Each run is also an audit
record (`SelfTestCompleted`).

| Test | What it checks | If it fails, the TOE | The election board |
|---|---|---|---|
| 1. Ballot boxes | Integrity and consistency of the intermediate ballot box and the ballot box: checkpoints, seals and the sequence of the stored votes | Treats it as an unrecoverable data integrity error: stops the election execution of the election event for good and keeps the ballot boxes unreadable and unmodifiable | Follows [An unrecoverable failure](12-modes-and-failures.md#an-unrecoverable-failure) |
| 2. Imported election data | That the published configuration still matches the digests of the approved configuration | Enters maintenance mode | Compares the configuration version on the Publish tab with the approved one, and has the administrators check the settings; resumes only when a new self-test passes |
| 3. Audit generation | That a test record can be written to the electoral log, read back, and that the chain of records is intact | Enters maintenance mode | Has the administrators check the storage of the electoral log; runs "Audit" on the Logs tab |
| 4. TLS interface | That the connection offered to remote endpoints meets the required TLS profile | Enters maintenance mode | Has the operators check the certificates and the TLS configuration, as the Deployment Guide describes |
| 5. Access-control configuration | That permissions, roles and signing rules still match their recorded digests | Enters maintenance mode | Checks Users and Roles and the Signatures tab with the administrators; treats any change nobody approved as a security-relevant event |
| 6. Clock | That the server clock is synchronised with the trusted time server | Enters maintenance mode | Has the operators check the time service |

A failed test always means that the TOE cannot vouch for that property at that
moment. Never resume an election whose last self-test failed: the TOE refuses
it, and the cause must be found first.

## Running a self-test on request

Any board member can run the self-test at any time with "Run self-test" on the
Dashboard, and then read the result on the "Self-tests" tab. Run it:

- shortly before the start (see
  [The self-test before the start](06-preparation.md#the-self-test-before-the-start));
- when anything suggests a problem: a voter's or observer's report, an
  unexpected record in the Logs tab, an attack detection, a notice from the
  operators;
- after the administrators or operators have worked on the platform;
- at the points the electoral regulation fixes.

The TOE also runs it by itself: at the start-up of the TOE, when the election
period start time is reached, before "Start Voting", before "Resume Voting",
after leaving maintenance mode, at the regular interval during the execution
phase ("Self-test interval (minutes)"), and under the additional conditions
(after each import of election data, after each pause of voting, before each
start of counting).

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

1. Run "Run self-test" and check that every test passed.
2. Select "Resume Voting" and confirm, as for "Start Voting".
3. The TOE creates a signing request for "Open voting". Each required board
   member authenticates again at the external authentication server and
   signs.
4. The TOE runs the self-test again and resumes voting only if it passes. When
   it resumes from maintenance mode, the Logs tab shows `MaintenanceModeLeft`.

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
