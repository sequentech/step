---
id: modes_of_operation
title: Modes of Operation and Failure Handling
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## The electoral phases

Open the election event's **Security and self-tests** tab to check its phase,
security state and reason. The server controls the phase; it is not an editable
setting. Administrators see the security status and administration settings.
Only the election board has access to election execution data and audit records.

| Phase | How it starts | What happens next |
| --- | --- | --- |
| Preparation | The board creates an authorization context or imports an approved election event. | Configure administrator settings, approved imports, signing rules, the key ceremony and initialization. The board approves and publishes the configuration. |
| Execution | The board or the approved schedule opens voting after the required checks pass. | Voters cast ballots within the approved period. The board may pause, resume or close voting with the required approvals. |
| Evaluation | The approved period ends or the board closes all elections. | Voting cannot reopen. The board checks the audit and authorizes counting. |
| Post-processing | The board selects **Start post-processing** in **Security and self-tests** and authenticates again. | Completed counting permits normal post-processing. An unrecoverable failure permits investigation. Board-approved read-out and export precede deletion. |

The first opening requires the approved time window, an empty ballot box,
initialization and a current **Before starting voting** result. A scheduled
opening uses its own election-start check. A diagnostic **On request** result
does not substitute for either. See [Security and Self-Tests](07-self_tests.md).

## Lockdown

Lockdown protects configuration settings during the agreed period. It is shown
as **Lockdown Status** on **Data** and is separate from the security state.
Lockdown alone does not stop voting.

The board configures the lockdown schedule before approval. The server also
enforces electoral phases: ending a lockdown does not permit rewriting approved
election data or returning from evaluation to execution. Administrators cannot
change the board's schedule. Follow the
[Handbook for Election Organizers](../05-election_organizer_handbook/01-about.md)
when a new configuration version is needed.

## Voting paused by the election board

A pause stops voting and invalidates the previous transition evidence. The board
records why it paused the election and decides whether resumption is allowed
under the electoral regulation. An administrator does not take that decision.

1. Resolve the reason for the pause and preserve the relevant records.
2. Select **Security and self-tests → Before resuming voting → Run self-tests**.
3. Wait for all six checks in the latest run to pass for the current state.
4. The board authorizes **Resume Voting** in **Publish**. The approved voting
   period and the required services must still be available.

A failed, pending, stale or differently purposed run cannot authorize resumption.
If the board tries too soon, the operation remains refused; wait for its required
check and then submit a new authorized operation.

## Maintenance mode

**Recovery required** means that a recoverable failure has stopped normal
voting. A failed required check, a stopped required service or an expired
service heartbeat causes refusal. A repair or restart does not reopen voting.
The system records the transition and queues a separate security notification.

1. Read the reason and the latest check results in **Security and self-tests**.
2. Tell the board and server operator. Record the event identifier, run
   identifier, time and reason; preserve the operating logs.
3. Correct the cause using AGD-04, for example the service, trusted certificate,
   authenticated time source or storage availability. Never replace evidence
   merely to make a test pass.
4. Run **Before resuming voting** and wait for the current six-check result.
5. The board reviews the investigation and authorizes **Resume Voting**. The
   server checks its preconditions again and runs an additional check after
   leaving maintenance.
6. Check **Security email delivery** and confirm that the responsible people
   received the notice. A mail failure is visible and does not remove the refusal.

## An unrecoverable failure

A detected ballot-box integrity error produces **Unrecoverable failure**.
Election execution cannot resume, even if a later diagnostic check passes.
Do not edit ballots, remove records, replace the approved baseline or delete
execution data to clear the state.

Preserve the evidence and notify the election board. The board follows the
electoral regulation and may enter post-processing for an authorized
investigation and export. This does not turn an incomplete or failed election
into a successfully counted election.

## A crash or shutdown of a server component

During execution, obtain the board's decision before an operator restarts a
component. Follow AGD-04 and verify the accepted image digests and service state.
An absent or expired required-service lease prevents voter operations.

Startup records the recovery requirement for an executing event. Follow the
maintenance procedure above, including the purpose-specific checks and board
resumption. Do not infer readiness merely from a green container status.

## The audit storage threshold

Above 80% used audit-storage capacity, the system refuses voter token issuance,
refresh and voter operations. Missing, stale, future-dated or unreadable storage
observations also refuse access. Staff access remains available for recovery.

Extend storage as AGD-04 describes; never delete audit records or election
execution data to make space. A fresh healthy observation removes the storage
refusal, but every other lifecycle and self-test gate still applies. See
[Audit, Monitoring and Notifications](09-audit_and_monitoring.md#the-audit-storage-threshold).

## Backups

The operational environment provides protected, tested backups. Follow AGD-04.
A restoration replaces current data with an older state and is not a normal way
to recover an executing election. Obtain the board's decision, preserve the
incident evidence and investigate the audit sequence before any resumption.
