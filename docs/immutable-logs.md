<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Election logs on main/B4

The bulletin board and the electoral log use PostgreSQL in separate logical databases. The bulletin board stores protocol artifacts; the electoral log stores signed records of election operations. The electoral log uses the dedicated `<client>_electoral_log` database on the existing PostgreSQL server.

See the [PostgreSQL electoral-log developer guide](docusaurus/docs/07-developers/03-development-environment/electoral-log-postgres.md) for architecture, schema, retries, exports, configuration and the breaking-change rollout.

Message signatures authenticate signed statements. Trellis commits the stored records and metadata to a per-board Merkle log in the same PostgreSQL transaction. Inclusion and consistency proofs can be verified against saved checkpoints using the electoral-log CLI or Harvest proof API. Keep checkpoints independently when comparing earlier history, and preserve the Trellis journal with database backups. Normal event deletion intentionally deletes its log. There is no legacy ImmuDB migration in this main-only change.
