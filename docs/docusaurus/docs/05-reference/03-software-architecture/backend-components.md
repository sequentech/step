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

### Braid
- **Role**: Implements a verifiable mixnet for anonymizing votes.
- **Technologies**: Rust
- **Path**: `step/packages/braid`

---

### Strand
- **Role**: Core cryptographic library used by Braid.
- **Key Components**:
  - ElGamal & Distributed Key Generation
  - Shuffling (Mixnet)
  - ZKPs (Schnorr, Chaum-Pedersen)
  - Signature & Symmetric encryption
- **Technologies**: Rust
- **Path**: `step/packages/strand`

## Secret Voter Attribute Boundary

An election-event Keycloak User Profile attribute annotated with `sequent.secret=true` is encrypted
before Step writes it to Keycloak. Keycloak stores a versioned `seqenc:v1:` authenticated-encryption
envelope. By default Keycloak has neither the master key nor plaintext access. The explicit
encrypted-attribute login option described below grants the authentication extension the key so
it can decrypt the configured credential server-side; stored attributes remain encrypted.

The encryption key is derived from the existing Step `master_secret` and the tenant, election
event, voter ID, and attribute name. Consequently, moving an envelope to another voter, event, or
attribute does not produce a valid value. Envelopes are randomized, so secret fields cannot be
searched, sorted, filtered, or compared by their encrypted representation.

Harvest enforces the synchronous create, edit, reveal, and permission boundaries. Windmill uses the
same codec for voter imports, authorized opt-in exports, communications, and per-voter reports.
Ordinary user responses and default voter exports remove secret fields rather than returning ciphertext.

Creating a voter with secrets uses two audited writes: Keycloak first validates a disabled voter
with ciphertext under a provisional scope, then Harvest re-encrypts the values against the ID
returned in Keycloak's `Location` header and restores the requested enabled state. If finalization
fails, the voter stays disabled. Required fields are validated on both writes; blank input stays missing.

Every reveal, set, clear, import, decrypted export, and use of a secret attribute in a
communication or per-voter report is recorded in the election event's electoral log as an
admin-signed `VOTER_SECRET_ATTRIBUTE` entry. The entry names the administrator, the voter where
one is involved, the attribute names and, for generated reports, the document id. It never contains a value.
The entry is written before the action takes effect, so an action that cannot be audited does not
happen.

Secret-bearing reports use private storage and require secret-read permission at download, including
password retrieval. Ordinary document writers cannot update or delete their access metadata.
Delivery audit entries do not contain rendered subjects or bodies. Explicitly selecting
`EMAIL_TRANSPORT_NAME=Console` or `SMS_TRANSPORT_NAME=Console` enables test-only delivery: the full
rendered message, including decrypted secret values, is printed to the worker console instead of
being sent. Use synthetic data and do not enable Console transport in production. Unknown transport
names fail rather than silently falling back to Console. Real delivery transports do not log bodies.

Harvest and the Windmill workers use the configured `master_secret` for these operations. Keycloak
needs the same 32-byte key, encoded as 64 hexadecimal characters in its `MASTER_SECRET` environment
variable, to verify encrypted login attributes. The supplied Compose files pass one shared
`MASTER_SECRET` deployment value to Keycloak, Harvest and Windmill. Keycloak therefore receives
the key whenever it is configured for the stack, but encrypted-attribute login still requires an
explicit `SECRET_ATTRIBUTE` credential policy; ordinary `PASSWORD` verification remains the default.
For external-vault deployments, inject the same existing vault master key into Keycloak.
Never send the master key to Hasura, the Admin Portal, or browser clients.
Losing or replacing the master secret makes existing voter envelopes unreadable;
master-secret rotation therefore requires a coordinated re-encryption migration and must not be
performed as an isolated secret replacement.

Decrypted exports retain a task-bound authorization grant in PostgreSQL. Workers reload the trusted
task row and check document, tenant, event and expiry before decrypting. The lifetime is controlled
by `WINDMILL_SECRET_EXPORT_GRANT_TTL_SECONDS` (default 86400) on Harvest and Windmill.
Explicitly password-encrypted event archives may include classified S3 documents under the same
grant; ordinary archives still exclude them and unknown/uncommitted document objects.

### Election event Trash

Deleting an election event (the `delete_election_event` action, also used by
`step-cli delete-election-event`) moves it to Trash: Windmill archives it and
records `trashed_at`, `purge_after` and the archive state to restore in
`sequent_backend.election_event`. Its rows and its Keycloak realm, documents,
boards and logs are kept. The `list_trashed_election_events` and
`restore_election_event` actions list and restore the caller tenant's events in
Trash and need the `election-event-delete` permission. Hasura doesn't expose
election events in Trash, or the Trash columns, to any client role, and no
client role can delete an election event directly.

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
the permanent deletion fails, its database changes are rolled back, the event
stays claimed, and it is claimed again after an hour. The external steps
(boards, immudb, documents, Keycloak realm) run inside the deletion's
transaction but aren't transactional themselves, so a retry can find some of
them already gone; the existing deletions treat missing resources as deleted.

Upgrading: Windmill workers from before Trash delete election events
permanently when they receive `delete_election_event_t`. Upgrade or drain every
worker that consumes the short queue before the Admin Portal and Harvest of
this version are used to delete election events.

Rolling back: stop Beat (no new claims), let running
`purge_trashed_election_event_t` tasks finish, then roll back the
applications and finally the database migration. The migration's rollback is
refused while any election event's permanent deletion has started
(`purge_started_at` set), because its external data may already be gone.
Election events still in Trash and not claimed become ordinary archived
events.

Generated encrypted reports save the exact password used in a document-bound vault entry and
keep its id in access annotations (not the password). Downloads from Reports and Tasks use the
existing password dialog and permission-checked `get_document_password` API. Older encrypted
reports without this entry still show decryption instructions and use their previously supplied
password. Changing a report configuration does not change earlier documents' passwords.
