---
id: backend-components
title: Backend Components
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Backend Services

### Harvest
- **Role**: Custom business logic that complements Hasura.
- **Responsibilities**:
  - Imports/Exports
  - Generating publications
  - Permissions and validation
  - Authorized secret voter-field create, edit, and reveal operations
  - Keys and Tally ceremonies
  - Election status monitoring
  - Third-party integrations
- **Path**: `step/packages/harvest`
- **Technologies**: Rust, Rocket, Hasura (GraphQL Actions)

---

### Windmill
- **Role**: Consumer service that handles background business logic and scheduled tasks.
- **Responsibilities**:
  - Ballots, Ceremonies, Tally Sheets
  - Audit Logs, Key Handling
  - User Management, Statistics
- **Subservices**:
  - `services/ballot_styles`: Ballot display
  - `services/ceremonies`: Key and tally ceremonies
  - `services/consolidation`: Audit logs, result packaging
  - `services/tally_sheets`: Manual tally support
  - `services/vault`: Vault secret management
  - `services/voter_secret_attributes`: Scoped encryption and decryption of secret voter fields
- **Path**: `step/packages/windmill`
- **Technologies**: Rust, Hasura, GraphQL, Postgres, AWS Secret Manager, HashiCorp Vault

#### Election event Trash

Deleting an election event (the `delete_election_event` action, also used by
`step-cli delete-election-event`) moves it to Trash. Harvest and Windmill refuse
it as before (locked down, voting open or paused, ballot box seals, elections
outside the user's permission labels); otherwise Windmill archives it and
records `trashed_at`, `purge_after` and the archive state to restore in
`sequent_backend.election_event`. Its rows and its Keycloak realm, documents,
boards and electoral log are kept. The `list_trashed_election_events` and
`restore_election_event` actions list and restore the caller tenant's events in
Trash and need the `election-event-delete` permission; restoring also needs
the permission labels of the event's elections. Hasura doesn't expose election
events in Trash, or the Trash columns, to any client role, and no client role
can delete an election event directly. The tenant log records each move to
Trash (`trash`), restore (`restore`) and permanent deletion (`delete`).

`WINDMILL_ELECTION_EVENT_TRASH_RETENTION_DAYS` (default 7) sets how many days an
election event stays in Trash. Set it on Windmill, which applies it, and on
Harvest, which reports it to the Admin Portal. Values must be whole days from 1
to 3650; any other value is refused, and election events can't be moved to
Trash until it is fixed. Changing it doesn't change the dates of events already
in Trash.

Beat's `scheduled_events` pass also claims up to 10 election events whose Trash
time is over (`purge_started_at`) and sends `purge_trashed_election_event_t`,
which deletes them permanently with the same deletion as before Trash existed.
The claim is committed first, so a claimed event can no longer be restored. If
the deletion's transaction fails, its database changes are rolled back, the
event stays claimed, and it is claimed again after an hour; external steps that
ran inside it (documents, Keycloak realm, protocol boards) aren't
transactional, so a retry can find some of them already gone. As for
deletions before Trash, the event's electoral-log database is retired after the
deletion commits; if that last step fails, the event is already gone and the
retirement isn't retried, which the failed task execution reports.

Upgrading: Windmill workers from before Trash delete election events
permanently when they receive `delete_election_event_t`. Upgrade or drain every
worker that consumes the short queue before the Admin Portal and Harvest of
this version are used to delete election events.

Rolling back: stop Beat (no new claims), let running
`purge_trashed_election_event_t` tasks finish, then roll back the applications
and finally the database migration. The migration's rollback is refused while
any election event's permanent deletion has started (`purge_started_at` set).
Election events still in Trash and not claimed become ordinary archived events.

---

### Beat
- **Role**: Publishes scheduled events and review board tasks to the queue.
- **Path**: `step/packages/beat`
- **Technologies**: Rust

---

### Velvet
- **Role**: Finalizes elections by decoding ballots, tallying, generating reports, and marking winners.
- **Path**: `step/packages/velvet`
- **Technologies**: Rust

---

### wbraid
- **Role**: Native cryptographic implementation, DKG, mixnet, proofs and signatures.
- **Path**: `step/packages/wbraid`
- **Integration**: `voting-crypto`, `protocol-board` and the native trustee runner.
- **Storage**: `b4v6` for protocol messages; independent immudb storage for application and database audit logs.

---
