---
id: self_tests
title: Self-Test Settings
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## When the TOE tests itself

The TOE runs a self-test of each election event:

- when the TOE starts up;
- when the start time of the election period is reached;
- before the election board starts voting with "Start Voting";
- before the election board resumes voting with "Resume Voting";
- after the election event leaves maintenance mode;
- at the regular interval during the execution phase ("Self-test interval
  (minutes)");
- under the "Additional self-test conditions" you select;
- on request: "Run self-test" on the election event's Dashboard.

## What the self-test checks

| Test | What the TOE checks |
|---|---|
| 1. Ballot boxes | the integrity and consistency of the intermediate ballot box and the ballot box: their checkpoints, seals and sequencing |
| 2. Imported election data | the integrity, authenticity and consistency of the imported election data: the published configuration against the digests the election board approved |
| 3. Audit generation | a test record is written to the electoral log, read back, and the chain of records is verified |
| 4. TLS interface | a TLS handshake against the public endpoint meets the TLS profile of the Deployment Guide |
| 5. Access-control configuration | the access rules and permissions, the roles and the signing rules against their recorded digests |
| 6. Clock | the server clock is synchronised with the time service |

A failed test 1 is an unrecoverable failure: the TOE stops the election
execution permanently. Any other failed test is a recoverable failure: the TOE
puts the election event into maintenance mode
([Modes of Operation and Failure Handling](10-modes_of_operation.md)).

## The settings

Set the self-test settings of each election event in the preparation phase,
in the "Settings" section of the election event's "Self-tests" tab, before the
election board approves the configuration version. Only the
administrator role holds the permission to change them
(`self-test-settings-write`), and the TOE accepts changes only in the
preparation phase. Each change is recorded in the election event's electoral
log.

| Setting | Values | Default | Required value |
|---|---|---|---|
| "Self-test interval (minutes)" | 1 to 60 | 15 | the value the electoral regulation fixes; see the secure values below |
| "Additional self-test conditions" | after each import of election data; after each pause of voting; before each start of counting | — | all three selected, for every election event |

## Secure values for the interval

During the execution phase, an event that only a self-test finds can stay
unnoticed until the next regular run, and voting continues during that time.
The interval therefore decides how late such an event is found, and how many
votes can be cast in between. Choose the interval with the election organizer,
by use case and by the duration of the election period:

| Use case | Duration of the election period | Interval |
|---|---|---|
| Election held on one day, for example a council election with one polling day | one day or less | 5 to 15 minutes |
| Any election with high load, for example many voters in the last hours before the end of election time | any | 5 to 15 minutes |
| Election over several days, for example a staff or student representative election over one or two weeks | more than one day, up to 14 days | 15 to 30 minutes |
| Long election period, for example an election open for several weeks | more than 14 days | 30 to 60 minutes, never more than 60 |

When in doubt, take the shorter value: a shorter interval costs some server
load, a longer one leaves more time in which an event remains unnoticed. The
interval cannot be changed after the preparation phase. For an election with
high load only in some hours, choose the value for those hours; the election
board adds runs with "Run self-test" when it needs them.

## Security-relevant events and the interval

| Security-relevant event | When the TOE finds it |
|---|---|
| Stored votes changed, removed or out of sequence; a checkpoint or seal that does not match | at the next run of test 1, after at most one interval |
| The published configuration no longer matches the approved one | at the next run of test 2, after at most one interval |
| Audit records cannot be written, or their chain is broken | at the next run of test 3, after at most one interval |
| The public endpoint no longer meets the TLS profile | at the next run of test 4, after at most one interval |
| Permissions, roles or signing rules changed | at the next run of test 5, after at most one interval |
| The server clock is no longer synchronised | at the next run of test 6, after at most one interval |
| A server part of the TOE that receives, stores or processes the votes crashes or is shut down during the execution phase | at once, without a self-test: maintenance mode |
| Election data was imported | right after the import (additional condition) |
| Voting was paused | right after the pause (additional condition) |
| Counting is about to start | before the start of counting (additional condition) |
| Voting is to be resumed, or maintenance mode is to be left | before the resumption; voting resumes only if the self-test succeeds |

The additional conditions close the moments in which the TOE changes state
outside the regular rhythm: they test the TOE right after new election data
arrives, right after voting stops for a pause, and right before the votes are
counted, so that a counting never starts on ballot boxes or a configuration
that failed a test. Select all three for every election event.

## The results

Each run is recorded in the electoral log of the election event and appears on
the election event's tab "Self-tests", with its time, its trigger and the
result of each test. Reading the tab needs `self-test-read`; "Run self-test"
needs `self-test-run`. The administrator role and the "Election board" role
hold both.

During the execution phase:

- check the "Self-tests" tab at least once per interval, and treat a missing
  run as a security-relevant event
  ([Security-Relevant Events for Administrators](11-security_relevant_events.md));
- when you suspect an event between two runs, tell the election board; you or
  a board member select "Run self-test" at once, instead of waiting for the
  next regular run.

:::warning
A longer interval than the secure values, or a missing additional condition,
lets a manipulation of the ballot boxes, the configuration or the access rules
go unnoticed for longer while voters continue to cast votes. Never select an
interval above 60 minutes, and never leave an additional condition unselected.
:::

## Other functions that verify the TOE

Beside the self-test, these functions let authorised users verify the
integrity of the TOE's data and of the TOE itself:

| Function | What it verifies | Where |
|---|---|---|
| "Audit" of the electoral log | every log entry against its Merkle log, and the published checkpoints; the result is the task "Audit Electoral Log" | election event, tab "Logs", used by the election board only ([Audit, Monitoring and Notifications](09-audit_and_monitoring.md)) |
| Ballot box seal verification | each sealed ballot box against its seal | when voting closes and at counting, as the Handbook for Election Organizers describes |
| Package verification | the signature, the approvals and the digest of each imported configuration package | at each import by the election board ([Tenant and Election Event Settings](05-tenant_and_event_settings.md#import-of-election-data)) |
| Image digest check | the running images of the TOE against `image-digests.txt` of the verified TOE delivery | [Secure Use of the Admin Portal](03-admin_portal.md) and the Deployment Guide |
