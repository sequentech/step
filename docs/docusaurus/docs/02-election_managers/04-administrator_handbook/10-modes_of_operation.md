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
| Preparation | when a member of the election board imports the election event | the board's import of election data; settings and signing rules; the board's "Publish" of a configuration version, which the TOE refuses in every other phase; the board's archiving of a wrongly imported election event; the key ceremony and the initialization report | verifies imported Data-tab values and configures the separate administrator settings as [Tenant and Election Event Settings](05-tenant_and_event_settings.md) describes, then tells the board when they are complete; the board publishes the configuration version and approves it |
| Execution | The board or a scheduled task opens voting. The complete ST self-test and automatic maintenance gates are not implemented; see the warning below. | Voters cast votes; the board controls voting status. | Monitors the TOE and supports the board. |
| Evaluation | at the end of election time, or when the election board terminates the election execution with "Stop Voting" and its required authorisations; voting cannot be reopened | the counting of the votes by the board, with its required authorisations | supports the board's audit review; changes no settings |
| Post-processing | the election board selects "Start post-processing" on the Dashboard | the export of election execution data and the board's read-out, with the required authorisations; after the export, the deletion | supports the board; changes no settings |

:::warning Required failure handling
The ST requires self-tests before opening and resuming voting and automatic
maintenance or permanent stopping on the specified failures. The complete
suite and these automatic state transitions are not implemented in this source
version. The Dashboard does not provide the previously described maintenance
screen. A successful background log audit does not close this requirement.
See [Self-Test Settings](07-self_tests.md#required-checks-still-to-be-completed).
:::

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

The election board controls voting status. Administrators must not resume voting
or change approved election settings to bypass a problem. An electoral-log audit
runs in the background and does not itself pause or resume voting.

Before a restart or resumption, the board records its decision under the
electoral regulation and obtains the required investigation results. Do not
assume an automatic complete self-test will run or refuse the resumption.

## Maintenance mode

The ST requires maintenance mode on recoverable failures and a controlled
return to operation. The complete mode, its Dashboard indicator and automatic
notifications are not implemented. This is an implementation requirement;
a manual pause and an operational checklist do not establish its compliance.

If an integrity check fails or a component stops:

1. Tell the election board and server operator immediately. Preserve the task
   identifier, its result and the operating logs.
2. The board decides whether to interrupt or terminate voting under the electoral
   regulation. Administrators do not take that decision on its behalf.
3. Investigate the failure with the operator. Check the delivered image digests,
   TLS endpoint and time synchronization using AGD-04.
4. The board checks the electoral log and can request **Logs → Audit**. Wait for
   the terminal **Audit Electoral Log** task result; a timeout or error is not a
   successful check.
5. Record what was found and what was corrected. Do not represent a restart as
   completion of the missing ST self-test and maintenance requirements.

## An unrecoverable failure

A ballot-box integrity failure requires the secure failure response defined by
the ST. That complete automatic response is not implemented as the self-test
state machine. Preserve every record and escalate to the election board;
do not repair evidence, delete data, or assume that a later green log audit
restores trust in the affected ballot box.

## A crash or shutdown of a server component

During election execution, restart a component only after the election board's
decision. Follow AGD-04 and verify the image digests and service status.
The board reviews the log records around the outage. Startup does not currently
run the complete ST self-test suite; keep that limitation explicit in the
incident and recovery record.

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
