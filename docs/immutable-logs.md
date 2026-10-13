<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Native board and audit storage

The cryptographic backend is the supplied `packages/wbraid` workspace. Its
protocol board holds canonical signed DKG and tally messages through the native
`b4v6` service. The application keeps lineage metadata in
`sequent_backend.protocol_board` and exports the original signed bytes.

See the [electoral log design](docusaurus/docs/07-developers/13-electoral-log/01-electoral-log-design.md) for architecture, schema, retries, exports, proofs, audits, configuration and the breaking-change rollout.

Message signatures authenticate signed statements. Trellis commits the stored records and metadata to a per-board Merkle log in the same PostgreSQL transaction. Inclusion and consistency proofs can be verified against saved checkpoints using the electoral-log CLI or Harvest proof API. Keep checkpoints independently when comparing earlier history, and preserve the Trellis journal with database backups. Normal event deletion intentionally deletes its log. There is no legacy ImmuDB migration in this main-only change.

Election deletion archives native protocol boards and retains their messages. The backend
explicitly deletes scoped metadata children before DKG parents inside the
application transaction. Foreign keys remain restrictive. Archival is an
idempotent operation in the independent store; retry deletion after a later
application transaction failure. Sealed ballot boxes continue to refuse deletion.
