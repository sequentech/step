---
id: election_management_election_event_logs
title: Logs
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The Logs tab provides a holistic view of all ongoing activities, offering detailed insights into the system's operations.

These are application-level actions being recorded. Access the system log to monitor activity across the entire platform, including voter events, Keycloak events, system events, and user events.

- Select **Columns** in order to hide/show different points of data per log entry.

## Immutability

The Logs tab is implemented using Immudb, a tamper-evident database. Each entry is inserted into a SQL table of actions. The immutability allows to verify that no change has been performed, for anyone with credentials to access to the immutable log.

## External system (Datafix) API entries

Every call an external voter-registry system makes to the Datafix API
(`add-voter`, `update-voter`, `delete-voter`, `mark-voted`, `unmark-voted`,
`replace-pin`), and every `SetVoted`/`SetNotVoted` request the platform sends
back to it, is recorded as an `ExternalApiRequest` entry.

- **Description** states the operation and its outcome only, for example
  `Inbound request UpdateVoter Succeeded.` or `Inbound request ReplacePin Failed.`
- **Message** carries the details in the operation string of the statement body:

  ```text
  voter_id="<datafix voter id>"; <Operation> <Outcome>[: <reason>] (<key>=<value>, ...)
  ```

  Values that come from free text (the voter id, the area name, the channel)
  and values read back from Keycloak (`area_id`, `previous_area_id`,
  `previous_birthdate`, which are attributes an administrator can edit) are
  written double-quoted, with `"` and `\` escaped, so a value containing
  `, ` or `=` is still read as a single value and not as another field. Only
  the fixed vocabulary (`none`, `unchanged`, `true`, `false`) and the
  validated `birthdate` of the request are written bare.

  | Operation | Values recorded on success |
  |---|---|
  | `AddVoter` | `area`, `area_id`, `birthdate` (`none` when not sent), `enabled=true` |
  | `UpdateVoter` | What the voter carried before the write (`previous_area`, `previous_area_id`, `previous_birthdate`, `previous_enabled`; `none` when the voter had no such value) next to what the request applied (`area`, `area_id`, `birthdate`, `enabled`); a field that was not sent is written as `unchanged` |
  | `DeleteVoter` | `enabled=false` and the `disable_comment` written to the voter (Datafix voters are disabled, never deleted) |
  | `MarkVoted` | `channel`, `enabled=false`, `disable_comment` |
  | `UnmarkVoted` | `previous_channel`, `channel=NONE`, and whether the account was re-enabled (`enabled`, `disable_comment`); `unchanged` means an administrator's disable was preserved |
  | `ReplacePin` | `temporary` (the PIN itself is never recorded) |

  A failed operation records the internal reason and the error code returned to
  the external system, for example
  `voter_id="123456"; ReplacePin Failed: Cannot replace pin because the user is disabled (error_code=invalid-request)`.
  The reason is only recorded here; the API reply carries the error code alone.

  Before writing to Keycloak, an operation checks that the election event's
  user profile stores every attribute it is about to write (for example
  `dateOfBirth`, `voted-channel` or `disable-comment`). If the realm would
  silently drop one, the operation fails without changing the voter, the
  external system receives `internal-error`, and the entry names the
  attributes the realm does not store.

  A request is attributed to the election event whose `datafix:id` annotation
  matches the Datafix id in the caller's token. If several election events of
  the tenant share that id, the request cannot be attributed to any of them
  and is rejected with `internal-error` before touching any voter, since only
  an administrator can fix the configuration. The failed entry is then
  recorded in the electoral log of **every** event configured with that id,
  naming all of them in the reason, with the voter's username alone (no
  `user_id` or `area_id`, because the voter cannot be resolved without an
  event).
- **User id** and **username** identify the voter (the Datafix voter id is the
  username) and the message's `area_id` is the voter's area, as in Keycloak
  events. A field the entry has no value for is left out of the message
  instead of being written as `null`: `election_id` always, because these
  operations apply to the whole election event, and `user_id`/`area_id` when
  the voter cannot be resolved, as in an `AddVoter` that failed before the
  voter was created.

### Request bodies in the service logs

Besides the electoral log entry, the API service records the full body of
every inbound Datafix request in its tracing output, so a rejected request can
be debugged without reproducing it. This data is deliberately not treated as
sensitive: the voter id is an anonymous registry identifier, and the same
values are already stored in the electoral log entry described above and in
the voter files exchanged with the external system. Only the caller's token is
kept out of the service logs.
