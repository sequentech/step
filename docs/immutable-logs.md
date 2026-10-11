<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Native board and audit storage

The cryptographic backend is the supplied `packages/wbraid` workspace. Its
protocol board holds canonical signed DKG and tally messages through the native
`b4v6` service. The application keeps lineage metadata in
`sequent_backend.protocol_board` and exports the original signed bytes.

Application electoral logs use `packages/electoral-log` and the same native
opaque board storage in an independent private PostgreSQL database. Set
`WBRAID_AUDIT_DATABASE_URL` and `WBRAID_AUDIT_PG_PASSWORD`; the public protocol
server must have no access to this database. Application messages retain their
sender and system signatures. Database audit collector records retain their
original contents and are scoped by tenant, event and source table.

Each delivery appends one immutable batch with its receipt. Stable delivery IDs
and payload hashes reject changed replays. A transaction lock serializes writers
per board. The SQL query index is derived from the native blobs: if indexing fails
after publication, the next request recovers it before checking receipts. Query
columns are closed enums and caller values are bound PostgreSQL parameters.

The native store does not supply the retired database's Merkle verification
protocol. Electoral-log integrity comes from the application signatures and
ballot-box seals; protocol integrity comes from native signed messages and their
verified configuration bindings. Protect database credentials and preserve
native blobs in backups alongside the application database and signing keys.

Election deletion archives native boards and retains their messages. The backend
explicitly deletes scoped metadata children before DKG parents inside the
application transaction. Foreign keys remain restrictive. Archival is an
idempotent operation in the independent store; retry deletion after a later
application transaction failure. Sealed ballot boxes continue to refuse deletion.

See [electoral-log tests and collector usage](docusaurus/docs/07-developers/03-development-environment/testing/electoral-log.md).
