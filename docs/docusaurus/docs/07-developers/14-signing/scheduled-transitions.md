---
id: signed_schedule_architecture
title: Signed scheduled transitions
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Scheduled voting transitions use the shared decision service
`windmill::services::scheduled_outcome`. It returns an outcome with the checks,
current and published values, deciding check and next step used by the executor,
prediction API and readable log. Keep these consumers aligned when adding a gate;
a prediction that ignores an execution refusal is a product defect.

## Trusted publication state

The configuration signing subject includes an append-only, defaulted schedule
list. Each transition binds the scheduled-event ID, processor, target, UTC
instant, local time, IANA zone and voting channels with a canonical fingerprint.
A request created before this field existed carries an empty list and covers no
schedule. Executed configuration approvals provide the trusted signed subject.
The lifecycle snapshot table retains publication policy/rule state, including
when the publication itself is deleted. These security tables are not exposed
for Hasura writes; presentation annotations are not an authorization source.

For each Post, resolve the applicable publication and approval, including
Post-level publication and event-wide schedule fallbacks. Compare both current
and published policy/rule requirements. Tightening applies immediately;
loosening waits for the applicable new approved publication. Initialization is
also target-specific. Never substitute one event-level requirement for every
Post's published requirement.

## Execution and deadlines

Scheduled tasks recheck authorization under the event signing lock in their
write transaction. Event-wide execution applies only to the eligible Post set,
with own-row precedence. Retried or stale tasks must not bypass row identity,
coverage, lateness, replay or initialization gates. An initialization retry may
wait only while the opening remains authorized and before its effective close.

Normal signed coverage has a 15-minute lateness limit. Separately,
`signing::actions::voting::enforce_signed_closes` enforces due closes retained in
the newest executed approval for a target. Own signed closes take precedence over
event-wide ones. A later already-due signed opening may supersede the close;
only a newer approved configuration can replace the signed schedule. Changes to
a live row do not extend the retained deadline. The scheduler tick dispatches
the close enforcement task in its own transaction. Replay records prevent
repeated transitions and duplicate audit steps.

Database writes and queue delivery have different failure boundaries. Stage
status changes, replay state and audit outbox entries in the same transaction;
request outbox delivery after commit. Do not mark an effect executed before its
transaction succeeds. The existing outbox delivery IDs provide idempotence when
a response is lost.

## Predictions and audit hooks

After a write affecting future outcomes, recompute and compare each prediction
with its previous explanation. This belongs inside the write transaction with
the actual actor. Hook points include schedule save/import/recompute, lifecycle
policy and signing-rule saves, publication, executed configuration approval,
lockdown changes and initialization eligibility changes. A failed transaction
must leave neither a partial schedule nor an outcome-change audit entry.

| Log kind | Evidence |
| --- | --- |
| `ScheduledOutcomeChanged` | Row identity and before/after explanations for a changed prediction; a prediction change is not an executed transition. |
| `SigningActionExecuted` | Fire-time result, deciding explanation and configuration authorization or unsigned/refused reason. |
| `ScheduleImported` | Applied CSV rows and the responsible actor. |
| `ScheduleRecomputeApplied` | Reviewed timezone-database changes applied to stored instants. |
| `ElectionInitialized` | Dated target initialization and report hash. |

Retain USER/SYSTEM outbox pairing and log-kind wire compatibility. No extra
outcome-change step is needed when the prediction is unchanged. Include the
current/published checks in structured details, and render their explanations
through the shared message keys rather than duplicating decision logic in UI
code.

See the [operator guide](../../02-election_managers/02-reference/02-election-event/18-signed-scheduled-transitions.md)
for decisions and recovery, [signing architecture](./01-signing-architecture.md)
for locks and delivery, and [platform guards](./platform-guards.md) for direct-write
trust boundaries.
