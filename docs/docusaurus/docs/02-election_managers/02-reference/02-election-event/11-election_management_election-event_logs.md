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

## External system (Datafix) API entries

Every call an external voter-registry system makes to the Datafix API (`add-voter`,
`update-voter`, `delete-voter`, `mark-voted`, `unmark-voted`, `replace-pin`), and every
`SetVoted`/`SetNotVoted` request the platform sends back to it, is recorded as an
`ExternalApiRequest` entry.

- **Description** states the operation and its outcome only, for example
  `Inbound request UpdateVoter Succeeded.` or `Inbound request ReplacePin Failed.`
- **Message** carries the details in the operation string of the statement body:

  ```text
  voter_id="<datafix voter id>"; <Operation> <Outcome>[: <reason>] (<key>=<value>, ...)
  ```

  Values that come from free text (the voter id, the area name, the channel) and values read back
  from Keycloak (`area_id`, `previous_area_id`, `previous_birthdate`, which are attributes an
  administrator can edit) are written double-quoted, with `"` and `\` escaped, so a value
  containing `, ` or `=` is still read as a single value and not as another field. Only the fixed
  vocabulary (`none`, `unchanged`, `true`, `false`) and the validated `birthdate` of the request
  are written bare.

  | Operation | Values recorded on success |
  |---|---|
  | `AddVoter` | `area`, `area_id`, `birthdate` (`none` when not sent), `enabled=true` |
  | `UpdateVoter` | What the voter carried before the write (`previous_area`, `previous_area_id`, `previous_birthdate`, `previous_enabled`; `none` when the voter had no such value) next to what the request applied (`area`, `area_id`, `birthdate`, `enabled`); a field that was not sent is written as `unchanged` and keeps its value |
  | `DeleteVoter` | `enabled=false` and the `disable_comment` written to the voter (Datafix voters are disabled, never deleted) |
  | `MarkVoted` | `channel`, `enabled=false`, `disable_comment` |
  | `UnmarkVoted` | `previous_channel`, `channel=NONE`, and whether the account was re-enabled (`enabled`, `disable_comment`); `unchanged` means an administrator's disable was preserved |
  | `ReplacePin` | `temporary`, which follows the optional `temporary` setting of the event's `datafix:password_policy` and is `false` when it is not set (the PIN itself is never recorded) |

  A failed operation records the internal reason and the error code returned to the external
  system, for example
  `voter_id="123456"; ReplacePin Failed: Cannot replace pin because the user is disabled (error_code=invalid-request)`.
  The reason is only recorded here; the API reply carries the error code alone. An `UpdateVoter`
  that would change the area, the enabled state or another census attribute of a voter with an
  in-progress or valid online ballot is refused as before, and the entry names the attributes and
  the ballot state, for example
  `Cannot change area-id of a voter who has cast a ballot: The voter has a valid online vote (error_code=voter-voted-online)`.

  Before writing to Keycloak, an operation checks that the election event's user profile stores
  every attribute it is about to write (`area-id`, `tenant-id`, `dateOfBirth`, `voted-channel`,
  `disable-comment`). If the realm would silently drop one, the operation fails without changing
  the voter, the external system receives `internal-error`, and the entry names the attributes
  the realm does not store. An event that takes Datafix requests must therefore declare these
  attributes as editable by administrators, or allow administrators to edit unmanaged attributes.

  A request is attributed to the election event whose `datafix:id` annotation matches the Datafix
  id in the caller's token. If several election events of the tenant share that id, the request
  cannot be attributed to any of them and is rejected with `internal-error` before touching any
  voter, since only an administrator can fix the configuration. The failed entry is then recorded
  in the log of **every** event configured with that id, naming all of them in the reason, with
  the voter's username alone (no `user_id` or `area_id`, because the voter cannot be resolved
  without an event).
- **User id** and **username** identify the voter (the Datafix voter id is the username) and the
  message's `area_id` is the voter's area, as in Keycloak events. `user_id` and `area_id` are
  empty when the voter cannot be resolved, as in an `AddVoter` that failed before the voter was
  created.

The recorded attributes are plaintext voter attributes: none of them can be configured as a
secret attribute, so recording their values does not bypass the event's attribute storage
settings.

### Request bodies in the service logs

Besides the log entry, the API service records the full body of every inbound Datafix request in
its tracing output, so a rejected request can be debugged without reproducing it. The body holds
only the values listed above, which are already stored in the log entry and in the voter files
exchanged with the external system. The caller's token is kept out of the service logs.
