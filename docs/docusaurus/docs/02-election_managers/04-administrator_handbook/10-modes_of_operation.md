---
id: modes_of_operation
title: Modes of Operation and Failure Handling
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## The electoral phases

Each election event is in one electoral phase, shown as "Electoral phase" on
its Dashboard. The TOE sets the phase; nobody edits it. Your Dashboard shows the
election event's phase and settings, not election execution data such as
turnout, cast votes or ballot boxes: only the election board sees those.

| Phase | How it starts | What the TOE allows | What the administrator does |
|---|---|---|---|
| Preparation | when a member of the election board imports the election event | the board's import of election data; settings, signing rules and self-test settings; the board's "Publish" of a configuration version, which the TOE refuses in every other phase; the board's archiving of a wrongly imported election event; the key ceremony and the initialization report | verifies imported Data-tab values and configures the separate administrator settings as [Tenant and Election Event Settings](05-tenant_and_event_settings.md) describes, then tells the board when they are complete; the board publishes the configuration version and approves it |
| Execution | at the start time of the election period the TOE runs the self-test and checks that the ballot boxes are empty, and opens voting only if both succeed; or the election board starts voting with "Start Voting", which the TOE accepts only at or after the start time, after a successful self-test, with empty ballot boxes and once the required initialization report exists | voters cast votes; the board can interrupt, resume and terminate the election execution | monitors the TOE and supports the board; changes no settings |
| Evaluation | at the end of election time, or when the election board terminates the election execution with "Stop Voting" and its required authorisations; voting cannot be reopened | the counting of the votes by the board, with its required authorisations | supports the board's audit review; changes no settings |
| Post-processing | the election board selects "Start post-processing" on the Dashboard | the export of election execution data and the board's read-out, with the required authorisations; after the export, the deletion | supports the board; changes no settings |

If the start of the execution phase fails, because the self-test fails or a
ballot box is not empty, the election event enters maintenance mode and stays
in the preparation phase (see below).

## Lockdown

Lockdown protects the approved configuration while the election takes place.
While the election event is locked down, the TOE refuses:

- generating a new publication;
- changing signing rules ("The election event is locked down: signing rules
  change only through a new configuration version.");
- deleting the election event;
- changing or deleting voters;
- changing the monitoring configuration.

Lockdown is not maintenance mode: it does not stop voting.

Lockdown starts and ends only through the scheduled events "Start Lockdown
Period" and "End Lockdown Period". The election board creates them at the times
the election organizer decides: the start after the configuration version is
approved and before the election period starts. The board's procedure is in the
[Handbook for Election Organizers](../05-election_organizer_handbook/01-about.md).
The administrator role holds no scheduled event permission, and no other
scheduled event is created: the election dates come only with the imported
election data. "Lockdown Status" on the tab "Data" shows the current state; do
not set it there.

## Voting paused by the election board

In the execution phase the election board can interrupt the election execution
with "Pause Voting" and resume it with "Resume Voting", a security-critical
action with its required authorisations. While voting is paused, no voter can start or continue an
election action. With the additional self-test condition "after each pause of
voting", the TOE runs the self-test right after the pause; before the
resumption it runs the self-test again and resumes voting only if it succeeds
([Self-Test Settings](07-self_tests.md)). While voting is paused, change no
setting and never resume voting yourself; if a self-test fails, the election
event enters maintenance mode.

## Maintenance mode

### When the TOE enters maintenance mode

The TOE puts an election event into maintenance mode on a recoverable failure:

- a self-test fails, other than the ballot box test;
- a ballot box is not empty when the execution phase is to start;
- a server part of the TOE that receives, stores or processes the votes
  crashes or is shut down while the election event is in the execution phase.

### What the TOE does

- It suspends voting: no voter can start or continue an election action.
- Nobody can read or modify the intermediate ballot box or the ballot box.
- The Dashboard shows "Maintenance mode" with the reason.
- It writes a record to the electoral log and emails the election board and
  the administrators.

### What the administrator does

1. Read the reason in the notification email and on the Dashboard, and the
   result of the failed self-test on the "Self-tests" tab.
2. Find and remove the cause together with the person who runs the servers,
   using the Deployment Guide: for example restore the time synchronisation,
   the TLS endpoint or a stopped component (in the execution phase only when
   the election board has decided so). Never change a setting, a role or a
   signing rule of the election event to make a test pass.
3. Tell the election board what the cause was and what was done, in writing.
4. Do not leave maintenance mode yourself: only the election board does.

### Leaving maintenance mode

- **In the execution phase**, the election board resumes with "Resume Voting",
  a security-critical action with its required authorisations. The TOE first
  runs the self-test and resumes voting only if it succeeds, and records that
  the election event left maintenance mode.
- **In the preparation phase**, after a failed start, the election board
  starts again with "Start Voting" once the cause is removed. The TOE runs the
  self-test and the check of the empty ballot boxes, opens voting only if both
  succeed, and records that the election event left maintenance mode. The
  scheduled start does not retry by itself.

## An unrecoverable failure

A failed ballot box test, that is, a data integrity error of the intermediate
ballot box or the ballot box, is an unrecoverable failure. The TOE then:

- stops the election execution of the election event permanently: voting
  cannot be resumed;
- keeps the ballot boxes unreadable and unmodifiable;
- writes a record to the electoral log and emails the election board and the
  administrators.

The election board decides under the electoral regulation. It can still end
the election execution, start post-processing without an election result, in
which case the Dashboard shows that no result was determined, and export the
data for investigation. Support the board, keep every server log, and report
the failure as a security problem
([Environment Measures and Security Problems](12-environment_and_security_problems.md#reporting-a-security-problem)).

## A crash or shutdown of a server component

If a server part of the TOE that receives, stores or processes the votes
crashes or is shut down during the execution phase, the election event enters
maintenance mode (see above). In the execution phase the component is
restarted only when the election board has decided so, and the self-test and
"Resume Voting" are the election board's. Outside the execution phase, restart
the component as the Deployment Guide describes, then check:

- the "Version" in the header of the admin portal and the image digests
  ([Secure Use of the Admin Portal](03-admin_portal.md));
- that a self-test runs at the start-up and succeeds;
- with the election board, which checks the electoral log, the records around
  the time of the crash; read the platform's operating logs only as the
  Deployment Guide describes.

## The audit storage threshold

When the storage of the audit data is more than 80 % full, the TOE refuses
voters' sessions and requests until space is available again
([Audit, Monitoring and Notifications](09-audit_and_monitoring.md#the-audit-storage-threshold)).
Extend the storage as the Deployment Guide describes; never delete audit
records or election execution data to make space.

## Backups

Backups of the servers, their protection and their restoration are measures of
the operational environment. Make and test them as the Deployment Guide
describes. A restoration replaces election data and election execution data
with an older state: never restore a backup of an election event that is in
the execution phase without the election board's decision, and tell the
election board of every restoration, so that it can check the electoral log.
