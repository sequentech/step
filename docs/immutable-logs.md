<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Native board and audit storage

The cryptographic backend is the supplied `packages/wbraid` workspace. Its
protocol board holds canonical signed DKG and tally messages through the native
`b4v6` service. The application keeps lineage metadata in
`sequent_backend.protocol_board` and exports the original signed bytes.

Application electoral logs use `packages/electoral-log` with the independent
immudb storage backend. Set `IMMUDB_SERVER_URL`, `IMMUDB_USER` and
`IMMUDB_PASSWORD`. Application messages retain their sender and system
signatures from `voting-crypto`. Hasura and Keycloak database audit records
remain in immudb; they do not use the public cryptographic protocol board.

Each delivery writes its records and receipt in an immudb transaction. Stable
delivery IDs and payload hashes reject changed replays. Definite transaction
read conflicts can be retried; transport failures with uncertain commit status
are propagated so the caller can redeliver using the same receipt identity.
Query columns are closed enums and caller values are bound named parameters.

The crypto backend replacement does not replace audit persistence. Application
signatures and ballot-box seals remain separate from immudb's storage history.
The electoral-log client uses immudb SQL APIs; running immudb alone should not
be treated as evidence that a caller verified its cryptographic history proofs.
Protocol integrity comes from native signed messages and their verified
configuration bindings. Preserve audit backups alongside the application
database and signing keys.

Election deletion archives native protocol boards and retains their messages. The backend
explicitly deletes scoped metadata children before DKG parents inside the
application transaction. Foreign keys remain restrictive. Archival is an
idempotent operation in the independent store; retry deletion after a later
application transaction failure. Sealed ballot boxes continue to refuse deletion.

The immudb electoral-log database follows its existing lifecycle: deleting an
election event deletes that event's electoral-log database. Protocol-board
archival is independent of that audit-storage operation.

See [electoral-log tests](docusaurus/docs/07-developers/03-development-environment/testing/electoral-log.md).
