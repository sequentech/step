---
title: PostgreSQL and Trellis electoral log
---
<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The main/B4 electoral log uses a dedicated PostgreSQL database on the existing PostgreSQL server. Each client has a database and role named `<client>_electoral_log`, separate from Hasura, Keycloak and the bulletin board. This is a breaking change for fresh installations: there is no historical ImmuDB migration or release-branch backport.

The obsolete pgAudit reader, hidden UI and unused logEvent action are retired on main/B4. Their collector was removed in 2025. ImmuDB crates, images and Compose services are no longer part of this application. PostgreSQL pgAudit session logging is independent and remains configured in development.

## Storage and message integrity

`electoral-log` separates message/domain types, one `ElectoralLogStore` storage port, a small application service and a PostgreSQL adapter. Windmill and Harvest pass typed filters to that port. SQL, connection pooling and transactions stay inside the adapter. `packages/trellis` supplies the imported Trellis Merkle tree and proof implementation, plus a transactional journal adapter.

`electoral_log_boards` identifies each event board. `electoral_log_messages` stores every record under its board name, with a PostgreSQL-generated ID, a delivery ID, the existing indexed metadata and the original signed message in `BYTEA`. Existing board names include the environment, tenant and event identifiers. Every read, count, append and delete is scoped to that board. Each board also maps to a unique generation in `trellis_logs`. `trellis_leaves` stores ordered hashes with their source record IDs. Deleting an event deletes its Trellis log and cascades to its board, messages and leaves; later deliveries fail instead of silently recreating it.

The serialized message format and signing code are unchanged. Trellis additionally commits to a versioned encoding of the board name, delivery ID and complete stored record (including its database ID, signed bytes and outer metadata). The encoding is the UTF-8 JSON serialization of the tuple `("sequent-electoral-log-v1", board_name, LogEntry)`, hashed with SHA-256; Trellis applies its normal CT leaf hashing to that digest. Field order and encoding are part of this versioned format.

Inclusion proofs demonstrate membership at a checkpoint; consistency proofs link a saved checkpoint to a later root. A checkpoint contains `log_name`, `log_id`, `tree_size` and `root`. Save checkpoints outside the log database when comparing history across restarts or restores. Recreating a deleted board produces a new generation and cannot extend the old checkpoint. Normal event deletion still intentionally removes that event's stored log.

## Appends, retries and exports

An append commits its messages and Trellis leaves in the same PostgreSQL transaction, or commits neither. A duplicate delivery creates neither another message nor another leaf. A CSV import streams through one append transaction, so an invalid later row rolls back earlier rows without buffering the entire file. Export/import preserves the raw signed bytes; there is no historical-store copier.

The unique `(board_name, delivery_id)` key makes retrying the same delivery idempotent. Direct writers create the identity before retrying, and prepared password-change entries serialize it with the durable task payload. The existing RabbitMQ dispatcher carries the original Celery message ID into its processing task; events that produce multiple records use distinct output suffixes. Independently delivered events with identical contents remain distinct. Processing tasks acknowledge late and retry unexpected failures up to five times. A batch spanning several boards commits separately per board; retrying an already committed board is safe. This does not create a distributed transaction with Hasura, Keycloak or RabbitMQ, and exhausted retries still require operational attention.

Appends lock the board row before allocating IDs. Within a board, commit order therefore cannot leave a late-committing lower ID behind an export cursor. CSV and CLI exports page in ascending ID order; CLI cast-vote exports continue beyond the default page size. Exports can include newly committed entries while running and are not a historical snapshot. Do not delete an event while exporting its log. Filtered UI queries use a stable ID tie-breaker. Read and decoding errors propagate instead of returning a successful partial result.

## Proof processing and API

Harvest hosts the Trellis processor and proof routes; no separate service, queue or database is needed. Every second it loads at most 1,000 leaves per board, including previously committed leaves after a failed pass. Each Harvest instance maintains its own in-memory tree cache and rebuilds it from the ordered persisted journal after restart. Preserve both `trellis_logs` and `trellis_leaves` in backups. Tree memory grows with the number of entries, as in upstream Trellis.

These internal Harvest POST routes use the same JWT and `LOGS_READ` authorization as electoral-log listing:

| Route | JSON request | Response |
| --- | --- | --- |
| `/electoral-log/checkpoint` | `tenant_id`, `election_event_id` | `checkpoint` and `committed_size` |
| `/electoral-log/inclusion` | Scope above plus `record_id` | Stored record and inclusion proof; HTTP 202 while processing is pending |
| `/electoral-log/consistency` | Scope above plus saved `checkpoint` | Earlier/current checkpoints and consistency proof |

`checkpoint.tree_size` reports processed entries; `committed_size` also includes pending entries. These are direct Harvest routes; the existing Hasura list action and CSV export formats stay unchanged.

The CLI processes committed leaves before producing evidence. Redirect its JSON output to files:

```sh
cargo run -p electoral-log --bin electoral-log-admin -- checkpoint --board BOARD > checkpoint.json
cargo run -p electoral-log --bin electoral-log-admin -- inclusion --board BOARD --record-id ID > inclusion.json
cargo run -p electoral-log --bin electoral-log-admin -- consistency --board BOARD --checkpoint checkpoint.json > consistency.json
```

An inclusion bundle contains the precise checkpoint it uses. Save that `inclusion.checkpoint` for later independent comparison; if the log grows between commands, a previously saved checkpoint may differ from a newly generated inclusion proof. The verification commands are offline and need no database environment variables:

```sh
cargo run -p electoral-log --bin electoral-log-admin -- verify-inclusion --checkpoint checkpoint.json --proof inclusion.json
cargo run -p electoral-log --bin electoral-log-admin -- verify-consistency --checkpoint checkpoint.json --proof consistency.json
```

Use the bundled Step schema and CLI for provisioning. The retained upstream source-table polling server and setup tools are optional developer tools behind `upstream-service`; Step does not run them.

## Configuration

| Variable | Meaning |
| --- | --- |
| `ELECTORAL_LOG_PG_HOST` | Existing PostgreSQL server hostname |
| `ELECTORAL_LOG_PG_PORT` | PostgreSQL port, usually `5432` |
| `ELECTORAL_LOG_PG_USER` | Dedicated database role |
| `ELECTORAL_LOG_PG_PASSWORD` | Secret for that role |
| `ELECTORAL_LOG_PG_DATABASE` | Dedicated database name |
| `ELECTORAL_LOG_PG_SSLMODE` | `disable`, `require` (default), or `verify-full` |
| `ELECTORAL_LOG_PG_SSLROOTCERT` | Optional CA PEM path for `verify-full` |

`require` requires encryption; `verify-full` also verifies the server certificate and hostname. Mount the CA file when the issuer is absent from the image trust store. `disable` is intended for the internal local development PostgreSQL connection. Each adapter uses a pool of at most eight connections; Harvest also uses a dedicated pool for proof processing and a ten-second connection timeout. No connection string or password is logged.

The application role owns the dedicated database in the supplied provisioning configuration. It therefore has administrative capability over its own history; it is not a cryptographically enforced append-only principal. Do not grant this role access to other application databases.

## Initialization and rollout

For fresh Compose installations, the existing `postgres` service creates the role/database and applies `packages/electoral-log/schema.sql` through `.devcontainer/postgresql/init-electoral-log.sh`. It uses the same server as Hasura. The development, remote and airgap Compose definitions pass the connection settings to the API/workers; airgap packaging includes the initialization script and schema.

PostgreSQL entrypoint scripts run automatically only for an empty data directory. For an existing development volume, first refresh the service's configured environment/mounts, then initialize explicitly without deleting the volume:

```sh
docker exec postgres sh /docker-entrypoint-initdb.d/20-electoral-log.sh
```

The script creates missing roles/databases and does not rotate an existing role's password. A password mismatch must be reconciled explicitly. Alternatively, initialize the schema as the dedicated owner with `psql --set ON_ERROR_STOP=1 --file packages/electoral-log/schema.sql`, connected to the new database, or run `cargo run -p electoral-log --bin electoral-log-admin -- init` with the variables above. `create-board --board <name>` and `delete-board --board <name>` administer an individual event board. Normal event creation/deletion already performs these operations.

For cloud deployments, the companion GitOps modules create the password, role, database and backup privileges on AWS/GCP. Apply client-secrets before client-postgres-init, then initialize the schema as the owner. Beyond's new-environment templates provide the endpoint/database/role and the `electoral-log-db-credentials` ExternalSecret mapping. Existing environments require those overrides when they are explicitly upgraded; the shared release defaults remain untouched. Backup jobs discover databases by client prefix, and the new database is included in the explicit backup grant list and future table/sequence grants.

Stop old log producers and drain pending tasks before deploying all main/B4 producers and consumers together. Task payloads and storage configuration change. The Trellis schema targets a fresh electoral-log database; it does not backfill an earlier PostgreSQL-only deployment. Create fresh election events in the new installation; old ImmuDB boards are not imported. Keep old storage and backups until retention requirements permit removal. Switching an old application image back does not transfer newly written PostgreSQL records to ImmuDB, so rollback after new writes needs an explicit operational decision.

The internal Harvest path is `/electoral-log`; the existing listElectoralLog Hasura action uses the logs-read permission. CLI `export-cast-votes` reads the `ELECTORAL_LOG_PG_*` settings instead of accepting ImmuDB server/username/password flags.

## Focused verification

Use a disposable PostgreSQL database, setting `ELECTORAL_LOG_TEST_DATABASE_URL` without printing credentials:

```sh
cd packages
CARGO_BUILD_JOBS=1 cargo test -p electoral-log --test postgres -- --ignored --test-threads=1
```

Run `CARGO_BUILD_JOBS=1 cargo test -p trellis --lib` for the imported tree tests. The PostgreSQL integration tests also cover atomic message/leaf writes, pending proofs, duplicate deliveries, saved-checkpoint verification, changed-record rejection, restart catch-up and board recreation. They exercise isolation, complete raw rows, atomic rollback, concurrent idempotency, distinct same-content events, visibility rules, counts and pagination. The existing PostgreSQL-backed Windmill CI job runs them explicitly. Ordinary message/signature unit tests continue to cover the unchanged signed format. Targeted mutation checks and independent review outcomes are recorded in `docs/design/electoral-log-postgres-implementation.md`; their results are scoped to the changed code, not a whole-repository mutation score.

The `generate-logs` Windmill binary also reads `ELECTORAL_LOG_PG_*` and `ENV_SLUG`; its TOML config now only needs the election-name map. It pages through the same storage port and propagates failures.
