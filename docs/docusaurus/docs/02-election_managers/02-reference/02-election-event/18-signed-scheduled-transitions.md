---
id: signed_scheduled_transitions
title: Scheduled openings and closings with signatures
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

A configuration approval can authorize an exact scheduled voting opening or
closing. It signs the schedule's identity, target, instant, local time, timezone
and channels, together with lifecycle policies and the opening/closing signing
rules. An old approval that did not include the schedule does not cover it.
Read the row's predicted outcome and **Why?** explanation before its deadline.

## Both copies must allow the transition

Scheduled transitions are evaluated against the current settings and the trusted
published configuration for their target. Either copy can require signatures;
both must allow a close without signatures for that fallback to run. Initialization
requirements also apply to openings. A stricter current requirement applies
immediately. Looser settings wait for the next publication and its approval when
configuration approval is required.

| Signing requirement in either copy | Exact row covered by an executed configuration approval | Opening | Closing |
| --- | --- | --- | --- |
| Not required | Either | Runs, subject to other opening gates | Runs |
| Required | Yes, unchanged and eligible | Runs, subject to initialization | Runs, authorized by configuration |
| Required | No | Refused | Runs without closing signatures only if both copies allow **Run as system**; otherwise refused |

Before a publication exists, restrictive defaults apply: initialization per Post
and unsigned close **Refuse**. Publication and initialization remain prerequisites
for opening where required. Manual voting actions use the live signing rules.
A configuration approval's signers are its authorizers; they are not separate
signatures on a manual Close voting request.

Three examples explain changes between publication and the deadline:

1. **Tighten:** publish a configuration allowing unsigned closes, then change the
   current policy to Refuse. An uncovered close is refused immediately. Publishing
   again is not necessary to enforce the tighter restriction.
2. **Loosen:** publish Refuse, then change the current policy to Run as system.
   An uncovered close still refuses because the published copy is stricter.
   Publish and approve the changed configuration before relying on the fallback.
3. **Edit after signing:** approve an opening at 09:00, then edit it to 10:00.
   The edited row is no longer covered. Where signatures are required it refuses
   until the changed schedule is published and approved. Editing a signed close
   has the additional deadline protection described below.

## Signed close deadlines remain authoritative

An approved close remains an authoritative deadline for its signed target and
channels even if its live schedule row is edited, stopped, archived or deleted.
The scheduler enforces retained signed closes. A newer approved configuration
can replace that deadline, and a later signed opening that has already occurred
can supersede it. Editing the row does not extend
voting. A replacement row's refusal and enforcement of the retained signed close
are separate decisions; review both the live row and the published schedule.

Normal signed coverage rejects replay, changed channels and a transition that is
more than 15 minutes late. The authoritative signed close enforcement exists to
prevent a missed scheduler tick or a live-row edit from extending an already
signed deadline. The lateness bound is not a 15-minute grace period for voters.
An opening that is waiting for initialization cannot reopen or extend voting
past its effective close.

## What the interface and audit trail mean

| Display or entry | Meaning and next action |
| --- | --- |
| **Will run**, authorized by configuration | The exact eligible schedule is covered, or signatures are not required. Read Why? for other gates and the target. |
| **Will run without signatures** | Both copies permit the uncovered close. Complete a manual signed close beforehand if closing signatures are needed. |
| **Will be refused** | Why? names the blocking check and the next step, commonly publishing and approving the changed configuration or completing initialization. |
| **Why?** | Shows current and published requirements, coverage, deciding check and recovery action. Event-wide rows may have different outcomes for individual Posts. |
| Save/import notice | Explains how the proposed edit changes predictions, including loss of signed coverage. |
| Tightening/loosening notice | Says whether scheduled changes apply now or after the next approved publication. Manual actions follow current rules. |
| Configuration approval panel | Lists what the approval authorizes and changes from the previous approved configuration. Check the schedule, channels and policy changes before signing. |
| Scheduled transition result card | Records the actual opening/closing and its authorization, or that it closed without signatures. |
| `ScheduledOutcomeChanged` | Records before/after predictions when a write changes the outcome, with its explanation and actor. It is not evidence that the transition executed. |
| `SigningActionExecuted` | Records the scheduled transition's actual authorization or unsigned/refused result. Normal voting-period log entries record the status change. |
| Cancelled Close voting request | A schedule closed the Post before the waiting request completed. Partial signatures remain in history and do not count as a signed close. |

Predictions describe the state when read. Later changes can alter the outcome;
the executor checks again at fire time. The log preserves the explanation used
then. Review failed tasks and delivery status as well as prediction chips.

## Before, at and after a deadline

Before the deadline, review the target's current and published values, upcoming
outcome, retained signed close and initialization status. For an uncovered
opening, publish and approve the exact schedule. For a refused uncovered close,
complete the authorized manual close or approve the intended schedule. Do not
assume a pending request already authorizes anything.

At the deadline, check the actual voting state and scheduled result. A common
close does not automatically extend for late initialization. If a request was
cancelled by the schedule, inspect its history for the recorded partial signatures.

Afterward, use readable Logs entries and their details to establish the effective
configuration, target, channels and deciding check. For a refused opening,
correct the cause and schedule an eligible future opening before the effective
close; do not replay an expired opening. The
[initialization ceremony runbook](../../01-tutorials/11-admin_portal_tutorials_initialization-ceremony.md)
explains initialization recovery at event, Post and Post-and-country scope.

## Lockdown and known gaps

Signing-rule edits are refused during lockdown. Outside lockdown they affect
manual actions immediately; scheduled actions still require both copies to allow
them. Lockdown alone does not create schedule coverage.

- A manual Close voting request cannot yet be fully signed ahead of time and
  held for execution at its scheduled deadline. Configure and approve the
  scheduled close, or use the explicit unsigned-close fallback where authorized.
- The broader signing workflow still needs signed authorization for lifting
  lockdown or loosening a live rule. Current/published checks protect scheduled
  transitions; they do not change the manual action signing model. Review
  [platform guard coverage](../../../07-developers/14-signing/platform-guards.md)
  for direct-write boundaries.
- Predictions are not a service availability guarantee. Check scheduler/task and
  audit outbox health during operations. Retained close enforcement requires the
  worker to run; it is not an external time service.
