---
id: election_management_election_event_logs
title: Logs
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The Logs tab lists everything recorded for the election event: voter events, Keycloak events, system
events and what administrators do in it. Its subtitle reads: "Everything done in this election event.
Sign-ins and changes outside it are in Settings > Logs."

Sign-ins to the Admin Portal, staff accounts and roles, tenant settings, and creating, importing,
archiving and deleting election events are recorded in the tenant log instead; see
[Settings > Logs](../user-manual/settings/settings_logs.md).

## What administrators' changes look like

A change an administrator makes in the election event is an `AdminChange` entry with the person, the
object, the action and each changed field with its old and new value. This covers:

- changes the Admin Portal saves directly to the event's records: the event itself and its
  elections, contests, candidates, areas, scheduled events, reports and the like;
- voters created, edited, enabled, disabled, deleted, imported and exported, and a voter's password
  reset by an administrator;
- numbers added to or removed from the phone blacklist;
- changes to the event's login settings: its realm attributes, password policy and voter
  authentication.

A change made to the event's realm directly in Keycloak's admin console, outside the Admin Portal, is
a `KeycloakAdminEvent` entry. Other actions keep their own statement kinds, as before (for example
publications, voting periods, key ceremonies and tallies).

Secret values are never logged, and texts longer than 64 KB are logged as SHA-256 hashes; see
[Secrets and long texts](../user-manual/settings/settings_logs.md#secrets-and-long-texts).

## The list

- Select **Columns** in order to hide/show different points of data per log entry. **User Id** and
  **Message** start hidden, and so does **Created** when the list is narrower than 1,500 pixels.
- Select a row to open the entry in a popup. Below 1,000 pixels each entry takes two lines and opens
  the same popup.
- **Event Type** is `USER` or `SYSTEM`, and **Log Type** is `INFO` or `ERROR`.

### Filters

**Created from**, **to** and **Timezone** are always shown. The range includes both ends, to the
minute, in the chosen timezone. **Add filter** offers **User Id**, **Username**, **Statement
Timestamp from** and **to**, **Statement kind**, **Event Type** (`USER` or `SYSTEM`), **Log Type**
(`INFO` or `ERROR`) and, for the messages sent to voters, **Channel** and **Status**.

### The entry popup

The popup is titled **Log entry** and the entry's number, and closes with **Close** or the Escape
key. It shows the description, the statement kind, event type and log type, then who and when (in the
log's timezone, your own time when it differs, and UTC), what changed with its **Old and new
values**, and the **Log and signatures** section with the key that signed the entry. It is the same
popup as in Settings > Logs; [The entry popup](../user-manual/settings/settings_logs.md#the-entry-popup)
describes each section.

## Export

Select **Export** to open the **Export logs** dialog. It needs the `logs-export` permission.

:::note
Earlier releases checked `report-write` for this export. After upgrading, give `logs-export` to the
roles that export an election event's logs.
:::

| Field | Choices |
|---|---|
| **Scope** | **The whole election event**, **One election**, **One area** or **One account**. An election or an area is chosen from the event's; an account needs its **Username**. |
| **From**, **To** | Optional. The range of **Created**, both ends included, to the minute. |
| **Timezone** | The timezone the range is read in and the file's local times are written in. When the event shows each election's entries in that election's timezone, **Each row's election timezone** keeps every row in its own. |
| **Format** | **CSV**, **SQL** or **PDF**. |

The export runs as an **Export Activity Logs Report** task. The whole election event as CSV or PDF is
the activity logs report, as before. An export of one election, one area or one account, and every
SQL export, has the columns described in
[Settings > Logs](../user-manual/settings/settings_logs.md#exporting): the SQL file is a PostgreSQL
script with each entry's signed statement, signatures and public keys, which an auditor can load and
verify.

Every export is recorded in the event's log as a `LogsExported` entry with its format, scope, range,
number of entries and the SHA-256 of the file.

## Integrity

Log entries are stored in PostgreSQL. Each entry is also committed to a Merkle log for its election event. Through the platform's API and command-line tools, operators can obtain inclusion proofs for individual entries and consistency proofs showing that the log has only been extended since an earlier checkpoint.

These proofs show tampering only relative to a checkpoint saved outside the log. The platform publishes a signed checkpoint when voting closes and when each results tally completes, stored separately from the log and recorded in it as an "Electoral log checkpoint published" entry. You can also save checkpoints independently, for example at the start and end of voting, and verify later proofs against them.

Deleting an election event no longer deletes its log. Before anything is removed, the tenant log records who deleted the event, with the last entry and the state hash of the event's log. The log's database is then closed and kept; it can no longer be read from the Admin Portal.

## Audit

An audit checks that every log entry matches its Merkle log, that the stored Merkle tree is complete and consistent, and that every published checkpoint is correctly signed and part of the log's history. An audit runs automatically when a results tally completes; its result appears in the tally logs.

- Select **Audit** to start an audit manually. This requires the logs-read and electoral-log-audit permissions.
- The audit runs as an "Audit Electoral Log" task. It succeeds when the log is consistent and fails when it finds a problem, listing each finding in the task logs. Audits only report problems; they never change the log.
