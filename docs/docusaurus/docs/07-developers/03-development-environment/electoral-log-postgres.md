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

## Merkle tree storage

Each board's Trellis log is an RFC 6962 Merkle tree stored in the electoral-log database. `trellis_leaves` holds the leaf hashes in order. `trellis_nodes` holds every complete internal node (perfect subtree), addressed by level and index; an append only adds nodes. `trellis_logs` holds the committed size and the current root. An append locks the log row, reads the right edge of the tree (the roots of the perfect subtrees that make up its size) from the stored nodes, checks that it produces the stored root, and inserts the new leaves and completed nodes in the same transaction as the messages before storing the new size and root. Each entry adds about 64 bytes of hashes: its leaf and, on average, one internal node.

Proofs are computed from stored nodes with no in-memory state and no background processing. Any Harvest instance can answer: in one read-only snapshot it checks the stored root against the right edge, reads the O(log n) nodes it needs with a few indexed queries, verifies the result and returns it. Every instance therefore reports the same committed checkpoint. Read proofs from the primary database; a lagging replica would answer for an older size. Preserve `trellis_logs`, `trellis_leaves` and `trellis_nodes` together in backups.

Logs created before nodes were stored have the empty root and no nodes. Reads and appends fail with a "must be rebuilt" error until `backfill-nodes` has run for the board; it recomputes nodes and root from the stored leaves in one transaction, and blocks appends to that board while it runs. Without `--board` it rebuilds every such log. On a log that already has a root, it also repairs damaged or missing nodes, but only when the stored root is the root of some prefix of the leaves, for example after an append that did not store its nodes. It refuses otherwise, so changed leaves are not turned into a consistent new history; published checkpoints remain the protection against an owner of the database rewriting it.

```sh
cargo run -p electoral-log --bin electoral-log-admin -- backfill-nodes --board BOARD
```

## Proof API

These internal Harvest POST routes use the same JWT and `LOGS_READ` authorization as electoral-log listing:

| Route | JSON request | Response |
| --- | --- | --- |
| `/electoral-log/checkpoint` | `tenant_id`, `election_event_id` | Current `checkpoint` |
| `/electoral-log/inclusion` | Scope above plus `record_id`, and optionally `trusted_checkpoint` | Stored record and inclusion proof; with `trusted_checkpoint`, also a consistency proof from it to the inclusion checkpoint |
| `/electoral-log/consistency` | Scope above plus saved `checkpoint` | Earlier/current checkpoints and consistency proof |

| Status | Meaning | Client action |
| --- | --- | --- |
| 200 | Evidence returned | Verify it |
| 400 | The checkpoint names another board | Check the request |
| 404 | Unknown board or record | Check the request |
| 409 | The supplied checkpoint is not in this board's history: another generation, a root the board never had at that size, or more entries than were ever committed | Treat as a possible fork or rollback; keep the checkpoint and investigate |
| 500 | Database error, or an integrity fault such as a record without a leaf, stored nodes that do not produce the stored root, or a log that must be rebuilt | Alert operators; run an audit |

These are direct Harvest routes; the existing Hasura list action and CSV export formats stay unchanged.

Verification is meaningful only against a checkpoint obtained and saved independently, such as the published checkpoints below. To verify a record against such a checkpoint while the board keeps growing, request inclusion with `trusted_checkpoint`. When the record is already covered by the trusted checkpoint, the inclusion proof is computed at that size and no consistency proof is needed; otherwise the bundle adds a consistency proof from the trusted checkpoint to the current one. The saved checkpoint stays the trust anchor, and the bundle's `inclusion.checkpoint` can replace it after verification.

The CLI produces the same evidence. Redirect its JSON output to files:

```sh
cargo run -p electoral-log --bin electoral-log-admin -- checkpoint --board BOARD > checkpoint.json
cargo run -p electoral-log --bin electoral-log-admin -- inclusion --board BOARD --record-id ID --checkpoint checkpoint.json > inclusion.json
cargo run -p electoral-log --bin electoral-log-admin -- consistency --board BOARD --checkpoint checkpoint.json > consistency.json
```

Without `--checkpoint`, an inclusion bundle verifies only against its own `inclusion.checkpoint`. The verification commands are offline and need no database environment variables:

```sh
cargo run -p electoral-log --bin electoral-log-admin -- verify-inclusion --checkpoint checkpoint.json --proof inclusion.json
cargo run -p electoral-log --bin electoral-log-admin -- verify-consistency --checkpoint checkpoint.json --proof consistency.json
```

## Published checkpoints

Windmill publishes a signed checkpoint of an event's log once per voting closure (`VOTING_CLOSED`), from a task queued after the closure is logged, and after a results tally session completes and is committed (`TALLY_COMPLETED`); initialization reports publish none. The event's protocol-manager key signs the compact JSON array `["sequent-electoral-log-checkpoint-v1", log_name, log_id, tree_size, hex(root), reason]`. The signed checkpoint is stored in the Hasura table `sequent_backend.electoral_log_checkpoint`, which lives in a different database with a different role than the log. An `ElectoralLogCheckpoint` statement also records it in the log itself, once per log size. Like the closure's log entries, a publication happens even if the closure's transaction later rolls back. Publishing is best effort: the closure only queues it, and the tally publishes after its commit, so a failed publication fails neither; it is logged, and for a tally also written to the tally logs. Users with `logs-read` can read the published checkpoints through GraphQL.

## Audits

Proofs show that a record is in the log; they do not show that every stored record is, or that the stored tree is the one previously published. An audit checks, in one consistent snapshot of the log database:

- every record has exactly one leaf and every leaf a record, leaf positions are dense and follow record order, and each recomputed record commitment matches its leaf;
- every stored node matches the node recomputed from the leaves, and the stored right edge and root match the recomputed tree;
- every published checkpoint names the event's board and log generation, is signed by the event's protocol-manager key with a valid signature, and has the root that the log's leaves have at that size.

Audits run as `AUDIT_ELECTORAL_LOG` task executions on the reports queue, one at a time per board: another audit of the same board waits up to 10 minutes for it to finish, without holding a database connection, and then fails. Every completed results tally publishes its checkpoint and then starts one; the publication, the queued audit and its result summary also appear in the tally session logs. Users with the `electoral-log-audit` permission can start one from the event's Logs tab, through the `audit_electoral_log` Hasura action, or with `cli step audit-electoral-log --election-event-id ID`. The task logs each finding (the first 100, with the total count) and records `outcome`, `board`, `tree_size`, `root`, `findings` and `published_checkpoints` in its annotations. A clean audit succeeds; findings or errors fail the task. Audits report problems and never repair them.

The CLI runs the same database checks against a board, optionally including a saved checkpoint, prints a JSON report and exits with an error on any finding:

```sh
cargo run -p electoral-log --bin electoral-log-admin -- audit --board BOARD --checkpoint checkpoint.json
```

An audit reads the whole board, so run it on demand or at milestones, not per request. A role that can rewrite records, leaves, nodes and the committed size together can produce a consistent new history; checking it against checkpoints published outside the log database detects that.

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

`require` requires encryption; `verify-full` also verifies the server certificate and hostname. Mount the CA file when the issuer is absent from the image trust store. `disable` is intended for the internal local development PostgreSQL connection. Each adapter uses a pool of at most eight connections and a ten-second connection timeout. No connection string or password is logged.

The application role owns the dedicated database in the supplied provisioning configuration. It therefore has administrative capability over its own history; it is not a cryptographically enforced append-only principal. Do not grant this role access to other application databases.

## Initialization and rollout

For fresh Compose installations, the existing `postgres` service creates the role/database and applies `packages/electoral-log/schema.sql` through `.devcontainer/postgresql/init-electoral-log.sh`. It uses the same server as Hasura. The development, remote and airgap Compose definitions pass the connection settings to the API/workers; airgap packaging includes the initialization script and schema.

PostgreSQL entrypoint scripts run automatically only for an empty data directory. For an existing development volume, first refresh the service's configured environment/mounts, then initialize explicitly without deleting the volume:

```sh
docker exec postgres sh /docker-entrypoint-initdb.d/20-electoral-log.sh
```

The script creates missing roles/databases and does not rotate an existing role's password. A password mismatch must be reconciled explicitly. Alternatively, initialize the schema as the dedicated owner with `psql --set ON_ERROR_STOP=1 --file packages/electoral-log/schema.sql`, connected to the new database, or run `cargo run -p electoral-log --bin electoral-log-admin -- init` with the variables above. `create-board --board <name>` and `delete-board --board <name>` administer an individual event board. Normal event creation/deletion already performs these operations.

For cloud deployments, the companion GitOps modules create the password, role, database and backup privileges on AWS/GCP. Apply client-secrets before client-postgres-init, then initialize the schema as the owner. Beyond's new-environment templates provide the endpoint/database/role and the `electoral-log-db-credentials` ExternalSecret mapping. Existing environments require those overrides when they are explicitly upgraded; the shared release defaults remain untouched. Backup jobs discover databases by client prefix, and the new database is included in the explicit backup grant list and future table/sequence grants.

Stop old log producers and drain pending tasks before deploying all main/B4 producers and consumers together. For a database written before tree nodes were stored, apply the schema (`init`), run `backfill-nodes` without `--board`, and only then start the producers. Instances older than the checkpoint statements cannot decode them, so do not run them against a log that has one. Task payloads and storage configuration change. Create fresh election events in the new installation; old ImmuDB boards are not imported. Keep old storage and backups until retention requirements permit removal. Switching an old application image back does not transfer newly written PostgreSQL records to ImmuDB, so rollback after new writes needs an explicit operational decision.

The internal Harvest path is `/electoral-log`; the existing listElectoralLog Hasura action uses the logs-read permission. CLI `export-cast-votes` reads the `ELECTORAL_LOG_PG_*` settings instead of accepting ImmuDB server/username/password flags.

## Focused verification

Use a disposable PostgreSQL database, setting `ELECTORAL_LOG_TEST_DATABASE_URL` without printing credentials:

```sh
cd packages
CARGO_BUILD_JOBS=1 cargo test -p electoral-log --test postgres -- --ignored --test-threads=1
```

Run `CARGO_BUILD_JOBS=1 cargo test -p trellis --lib` for the tree tests; they compare every root, inclusion proof and consistency proof for trees of up to 130 leaves, and for several larger trees, byte for byte with the `ct-merkle` implementation. The PostgreSQL integration tests also cover atomic message/leaf/node writes, duplicate deliveries, saved-checkpoint verification, changed-record rejection, board recreation, dense positions under concurrent appends, historical proofs, forged and future checkpoints, trusted-checkpoint inclusion, tampered nodes and roots, rebuilding legacy logs, and audit findings. They exercise isolation, complete raw rows, atomic rollback, concurrent idempotency, distinct same-content events, visibility rules, counts and pagination. The existing PostgreSQL-backed Windmill CI job runs them explicitly. Ordinary message/signature unit tests continue to cover the unchanged signed format. Targeted mutation checks and independent review outcomes are recorded in `docs/design/electoral-log-postgres-implementation.md`; their results are scoped to the changed code, not a whole-repository mutation score.

The `generate-logs` Windmill binary also reads `ELECTORAL_LOG_PG_*` and `ENV_SLUG`; its TOML config now only needs the election-name map. It pages through the same storage port and propagates failures.
