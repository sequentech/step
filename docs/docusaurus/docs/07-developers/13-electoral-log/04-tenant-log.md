---
id: tenant-log
title: Tenant Log
sidebar_label: Tenant log
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Tenant log

Each election event has an electoral log. Activity that belongs to no election event, such
as an administrator signing in or deleting an election event, had no log to go to: Keycloak
events of the tenant realm were dead-lettered, and tenant-level requests left only a task
row. The tenant log is where that activity goes. It uses the electoral log's message
format, signatures, Merkle log and reading code; the [design](01-electoral-log-design.md)
describes those.

## 1. Where it is stored

- A tenant's log is a board of the **base database** (`ELECTORAL_LOG_PG_DATABASE`), next to
  the catalog of the event databases. `EventDatabases::tenant_logs()` returns the store and
  creates the log tables there on first use: the part of
  `packages/electoral-log/schema.sql` before the ballot box, so the base database gets no
  ballot-box tables.
- The board is named `<ENV_SLUG>tenant<tenant id as 32 hexadecimal digits>`
  (`get_tenant_board`). It names the whole tenant and no event, so the router of event
  boards (`event_of_board`) refuses it and it can never land in an event's database.
- Reads select one board (`LogScope::Board`), so tenants sharing the base database do not
  see each other's entries.
- The log's signing key is a secret of the tenant with no election event,
  `boards/<board>/protocol-manager`, like an event board's key.

`open_tenant_log` (Windmill, `services/tenant_log.rs`) creates the key, the board and the
first entry the first time a tenant has something to record. The key is committed before
the board is created, under a transaction-level advisory lock, so two workers never create
two keys and no entry is signed with a key that was not kept. The first entry has a fixed
delivery ID, so it is stored once.

## 2. Entries

| Statement kind | Body | Written for |
| --- | --- | --- |
| `TenantLogOpened` | `TenantActivity` | The first entry of a log: its board and public key |
| `TenantAdminAction` | `TenantActivity` | A request to Harvest that belongs to no election event |
| `KeycloakUserEvent` | `KeycloakUserEvent` | A Keycloak event of the tenant realm |

- `StatementBody::TenantActivity(TenantActivityEntry)` and the statement types
  `TenantLogOpened` and `TenantAdminAction` are appended to their enums, so stored entries
  keep decoding. As for a signing entry, the entry sets the head's kind, event type, log type
  and description.
- `details_json` of a `TenantAdminAction` holds `action`, `outcome` (`succeeded`, `denied`
  or `failed`), `status` (the HTTP status), `user_id`, `username` and `details` (what the
  request named). They are inside the signed statement; the unsigned `user_id` and
  `username` columns are for filtering.
- The head's event is `Generic Event` (`GENERIC_EVENT`), as for every statement that belongs
  to no election event.
- Entries are signed with the log's key as sender and system, as Keycloak events of an
  election event are.

## 3. How an entry gets there

Both producers enqueue a `LogEventInput` on `electoral_log_event_queue` with an empty
`election_event_id` (the Keycloak listener sends the string `"null"` for a missing value).
The batch task that already stores election events' entries (`tasks/electoral_log.rs`) sends
those to the tenant's log instead of dead-lettering them.

```text
Harvest route ── TenantActivityRecorder ──┐
                                           ├─► electoral_log_event_queue ─► batch task ─► tenant log
Keycloak tenant realm ── event listener ──┘
```

- **Harvest.** A handler of a tenant-level route takes a `TenantActivityRecorder` and calls
  `record(&claims, TenantAction::…, details)` before authorizing the caller. Once the
  request is answered, the `tenant_activity` fairing adds the status and the outcome and
  enqueues the event with `message_type` `tenant_activity`. A handler that records nothing
  queues nothing. Routes shared with voters (`/create-user`, `/edit-user`, `/delete-user`,
  `/delete-users`, `/import-users`, `/export-users`) record only when the request names no
  election event.
- **Keycloak.** `custom-event-listener` recognizes the tenant realm (`tenant-<tenant id>`)
  and sends its events with no election event. For that realm it leaves out token
  housekeeping (`CODE_TO_TOKEN`, `REFRESH_TOKEN`, `INTROSPECT_TOKEN`, `USER_INFO_REQUEST`,
  `CLIENT_LOGIN`, `TOKEN_EXCHANGE`, `PERMISSION_TOKEN`); their `_ERROR` types are kept.
- **Windmill.** `build_tenant_event` opens the tenant's log once per batch and builds the
  record. An event for a tenant that doesn't exist, a body that is not a `TenantActivity`,
  a `tenant_activity` that names an election event and an internal record without one are
  dead-lettered; the rest of the batch is stored. A database failure fails the batch, which
  is retried; delivery IDs make the retry idempotent.

To record a new tenant-level action, add a variant to `TenantAction`
(`packages/harvest/src/services/tenant_activity.rs`) with its sentence, and call
`activity.record` in the route.

## 4. Reading

- Hasura action `listTenantLog(limit, offset, filter, order_by)` calls Harvest's
  `POST /tenant-log`, which needs `tenant-logs-read` on the caller's tenant and lists with
  the electoral log's filters and limits (`list_tenant_log`). The tenant comes from the
  session, never from the request.
- The Admin Portal reads it as the react-admin resource `tenant_log` in **Settings > Logs**
  (`SettingsLogs`).

## 5. Limits

- The entry of a request is queued after the request is answered. If the task-queue
  database cannot be reached at that moment the entry is lost, and Harvest logs
  `tenant activity: <action> could not be queued`. A transactional outbox would close that
  gap.
- Changes that the Admin Portal writes through Hasura without Harvest, such as tenant
  settings, are not recorded.
- The tenant log has no checkpoints published to Hasura, no export and no audit action yet.
- The console's reader role is granted on event databases, not on the base database.
- With the listener on the tenant realm, a sign-in to the Admin Portal enqueues its event
  as a voter's sign-in does. Under the default `ELECTORAL_LOG_PUBLISH_FAILURE_POLICY`
  (`fail-request`) the sign-in fails while the task-queue database cannot be reached or is
  not set up yet; `log-and-continue` lets it through and loses the entry
  ([Task queues](../03-development-environment/task-queues-pgmq.md)).

## 6. Enabling it for an existing tenant

- **Permission.** Create the realm role `tenant-logs-read` in the tenant realm and give it
  to the groups that may read the log, together with `settings-menu`.
- **Sign-ins.** Add `custom-event-listener` to the tenant realm's event listeners
  (**Realm settings > Events > Event listeners** in Keycloak). The development realm and
  the COMELEC template already list it.
- **Hasura.** Apply the metadata, which adds the `listTenantLog` action.

Nothing is migrated: the log, its key and its first entry are created when the tenant first
has something to record.

## 7. Tests

| What | Where |
| --- | --- |
| Statement, wire bytes and signatures | `packages/electoral-log/tests/{signed_messages,wire_types}.rs`, `tests/support/statement_tenant_activity_tests.rs` |
| Tenant boards in the base database (needs PostgreSQL) | `packages/electoral-log/tests/events.rs` |
| Entries, routing of a batch, listing (needs PostgreSQL) | `packages/windmill/src/services/tenant_log.rs`, `src/tasks/electoral_log.rs`, `src/services/electoral_log.rs` |
| Recorder, fairing, route and permission | `packages/harvest/tests/support/tenant_activity.rs`, `route_permissions.rs` |
| Realm parsing and the events kept | `CustomEventListenerProviderMessageTest` |
| The Logs tab | `SettingsLogs.stories.tsx`, `SettingsScreen.stories.tsx`, `test/journeys/settings/access.spec.ts` |
