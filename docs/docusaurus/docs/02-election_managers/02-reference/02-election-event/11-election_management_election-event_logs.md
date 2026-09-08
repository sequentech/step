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
  voter_id=<datafix voter id>; <Operation> <Outcome>[: <reason>] (<key>=<value>, ...)
  ```

  | Operation | Values recorded on success |
  |---|---|
  | `AddVoter` | `area`, `area_id`, `birthdate` (`none` when not sent), `enabled=true` |
  | `UpdateVoter` | `area`, `area_id`, `birthdate`, `enabled`; a field that was not sent is written as `unchanged` |
  | `DeleteVoter` | `enabled=false` and the `disable_comment` written to the voter (Datafix voters are disabled, never deleted) |
  | `MarkVoted` | `channel`, `enabled=false`, `disable_comment` |
  | `UnmarkVoted` | `previous_channel`, `channel=NONE`, and whether the account was re-enabled (`enabled`, `disable_comment`); `unchanged` means an administrator's disable was preserved |
  | `ReplacePin` | `temporary` (the PIN itself is never recorded) |

  A failed operation records the internal reason and the error code returned to
  the external system, for example
  `voter_id=123456; ReplacePin Failed: Cannot replace pin because the user is disabled (error_code=invalid-request)`.
  The reason is only recorded here; the API reply carries the error code alone.
- **User id** and **username** identify the voter (the Datafix voter id is the
  username) and the message's `area_id` is the voter's area, as in Keycloak
  events. `election_id` stays empty because these operations apply to the
  whole election event.
