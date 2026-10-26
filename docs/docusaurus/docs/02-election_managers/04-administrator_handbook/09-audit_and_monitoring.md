---
id: audit_and_monitoring
title: Audit, Monitoring and Notifications
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## The electoral log

The TOE records the security-relevant events of the tenant and of each
election event in the electoral log. Each record has its time, its type and
the account that caused it, and the records are chained, so that a removed or
changed record is detected. Among others the TOE records:

- the import of election data, each publication and approval of a
  configuration version, and each signing request with its signatures;
- the start, interruption, resumption and termination of the election
  execution, the counting, the export and the deletion;
- creating, changing and deleting users, roles, role permissions and role assignments, signing
  rules and realm policies;
- audit and integrity events that the implemented producers record. The full
  self-test suite and maintenance-mode event producers remain unimplemented
  ([Self-Test Settings](07-self_tests.md));
- the voters' actions in the voting portal.

Nobody can change or delete records. Only the election board reads, audits and
exports the electoral log: as administrator you do not read it. You learn of
security-relevant events from the TOE's notifications
([Email notifications](#email-notifications)) and from the election board. For
operating the servers you read the platform's own operating logs, only as the
Deployment Guide describes; they are not the electoral log.

## The election board's tools for the audit review

The election board reads the electoral log in the election event's tab "Logs"
and in the Electoral Log console, checks it with "Audit" ("Audit electoral
log"), whose result appears as the task "Audit Electoral Log" in the tab
"Tasks", and exports it with "Export in CSV" or "Export in PDF". The
permissions for these tools belong only to the "Election board" role
([Users, Roles and Role Separation](04-users_and_roles.md#the-election-boards-permissions)).

Your part: give these permissions to the "Election board" role as that page
describes, and check that no administrator account holds any of them. Give
"Read Electoral Log Personal Data" (`electoral-log-personal-data-read`) only
where the election organizer decides so.

## Attack detections

The TOE compares its records of failed logins and of requests with two attack
patterns:

| Pattern | When it matches |
|---|---|
| Brute force | the failed logins of one account reach the authentication failure threshold, or the failed logins across accounts exceed "Failed logins per minute, all accounts" |
| Denial of service | the requests on the external interfaces exceed "Requests per minute, external interfaces" |

Both rates are tenant settings in the section "Attack detection" of the
tenant's "Settings", and only the administrator role changes them
(`attack-detection-write`). Set them with the election organizer in the
preparation phase, from the number of voters and the expected peak load: high enough that the normal peak
does not match, low enough that an attack does. On a match the TOE writes a
record to the electoral log and shows it in the Monitoring tab's "Attack
detections", which only the election board sees (`monitoring-view`) and
watches during the execution phase.

When the election board tells you of an attack detection, treat it as a
security-relevant event
([Security-Relevant Events for Administrators](11-security_relevant_events.md)):
tell the person who runs the servers and the network, and agree the measures
with the board.

## The audit storage threshold

The TOE checks every minute how much of the storage for the audit data is
used. Above 80 % it:

- refuses new sessions and requests of voters, so that no vote is cast that
  could not be recorded;
- writes a record to the electoral log;
- emails the election board and the administrators.

Administrators and board members can still sign in and work. Extend the
storage at once, as the Deployment Guide describes: never by deleting audit
records or election execution data. Voters can sign in again once the used
space is below the threshold.

## Email notifications

The TOE emails the election board and the administrators when:

- an election event enters maintenance mode, with the reason;
- an unrecoverable failure stops the election execution;
- the audit storage threshold is exceeded.

The TOE sends the emails to the "Email" of each account. Before the execution
phase, check that every administrator's and board member's account has the
person's working address, and agree with the election organizer who reads the
email throughout the execution phase.

## Supporting the election board's audit review

At the beginning of the evaluation phase the election board checks the audit
records. Support the board:

- make sure that every board member who checks the records holds the
  permissions of the Logs tab and the Electoral Log console;
- when the board shows you a record that an administrator caused, for example a
  change of a setting, a role or a signing rule, explain it from your written
  record, with the reason and the instruction of the election organizer;
- if the board finds a record it cannot explain, help it to find the cause and
  treat it as a security-relevant event.

Keep your own written record of every change you make, with its date, time and
reason, so that you can explain each record later.
