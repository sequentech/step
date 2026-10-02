---
id: task-queues-pgmq
title: PostgreSQL task queues
sidebar_label: Task queues (PGMQ)
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Windmill, Harvest, Beat and Keycloak use **PGMQ 1.13.0** for durable task delivery.
This is a clean-environment change on `main`: RabbitMQ messages are not imported,
and there is no legacy transport switch. The existing Celery task API, 58 task
registrations, routes, retry policies, time limits and task-execution records remain.

## Storage and configuration

Queues live in the **Keycloak PostgreSQL database**, in the `pgmq` schema. Rust
reuses `KEYCLOAK_DB__*` and the existing TLS/CA configuration. Keycloak uses its
existing JPA connection and transaction, with no additional JDBC pool or password.
`KC_DB_*` and `KEYCLOAK_DB__*` must identify the same database.

Every producer and worker, including Beat, needs the same `ENV_SLUG`. Logical names
remain `<ENV_SLUG>_<queue>`. The physical PGMQ name is `step_` followed by the first
40 lowercase hexadecimal characters of SHA-256 of that logical name in UTF-8.
This preserves case and punctuation without collisions caused by normalization,
and stays within PGMQ's identifier-length limit. Rust and Java use the same mapping.

| Logical suffix | Consumer |
| --- | --- |
| `beat` | Scheduled election and board work |
| `short_queue` | Short background tasks |
| `communication_queue` | Communication and cast-vote tasks |
| `tally_queue` | Tally execution |
| `reports_queue` | Reports, rendering and post-tally work |
| `import_export_queue` | Imports, exports and voter bulk operations |
| `electoral_log_beat_queue` | Electoral-log batch dispatcher |
| `electoral_log_batch_queue` | Electoral-log batch processing |
| `electoral_log_event_queue` | Raw events; **batch dispatcher only** |

Normal Windmill consumers reject the raw-event queue: its Celery task is an enqueue
marker, not an event processor. Keep the normal eight queue subscriptions in the
Compose examples. Prefetch is bounded **per queue** and remains bounded across
broker reconnects. Existing task semaphores still apply.

## Fresh installation

The unchanged upstream SQL distribution and license are packaged at
`.devcontainer/postgresql/pgmq-1.13.0.sql` and `PGMQ-LICENSE`, from
[PGMQ v1.13.0](https://github.com/pgmq/pgmq/tree/v1.13.0).
No PostgreSQL host extension or separate broker container is required.

Development, remote and airgap Compose mount this SQL as a fresh-database init
script in `postgres-keycloak`. The airgap builder includes the SQL and license in
the delivered archive. PostgreSQL init scripts run only for an empty data volume.

For a clean managed PostgreSQL database, install as its application owner before
starting Keycloak, Harvest, Windmill or Beat. Use a configured libpq service (or
normal secure connection configuration):

```sh
psql service=keycloak -v ON_ERROR_STOP=1 --single-transaction \
  -f .devcontainer/postgresql/pgmq-1.13.0.sql
```

The installer creates the schema/functions; startup declares the application's
logged queues. The application role needs access to the schema, its functions,
queue tables and sequences, including permission to declare queues. Installing as
the application owner satisfies these requirements. Keep `synchronous_commit`
enabled and include this schema in Keycloak database backups and HA configuration.
Do not rerun the base SQL over an existing PGMQ installation; use upstream versioned
upgrade scripts when upgrading PGMQ in future releases.

## Delivery and scheduling guarantees

* Claims have a 60-second visibility lease, renewed every 10 seconds throughout
  local waits and execution. Long tally/report tasks retain their existing time limits.
* Workers acknowledge **after execution**. Acknowledgement, renewal and retry are
  fenced by the PGMQ message ID, read count and unexpired lease, so an old worker
  cannot acknowledge a newer claim. Database terminal operations have a five-second
  timeout. A worker exits if ownership is lost or renewal remains unavailable for
  40 seconds; its supervisor must restart it. Unfinished messages become visible
  when their leases expire.
* A Celery retry atomically updates the same queue row and preserves the task ID.
  Its subsequent acknowledgement is a no-op. Retry delays and ETA timestamps are
  persisted in PostgreSQL; delayed jobs do not hold worker capacity before their ETA.
* Completion, expiry, malformed envelopes and terminal task failures are archived.
  Existing task-specific retry limits remain authoritative, including tasks with
  zero retries. Archival is not evidence of business success; consult task-execution
  records and worker logs for the outcome.
* Raw audit events are promoted into one batch job and removed from the source queue
  in **one database transaction**. Failure or cancellation rolls back the whole
  handoff. Malformed raw events are archived without blocking later valid events.
* Keycloak success events enqueue in their request transaction. Publication failure
  marks that transaction for rollback. Keycloak 26.6.1 normally emits error events
  in a separate transaction, so ordinary `LOGIN_ERROR` records survive rollback of
  the original request. Other caller-controlled rollback behavior follows Keycloak's
  event transaction; there is no independent cross-database publish.
* Beat keeps its five existing schedules. A PostgreSQL advisory lock admits one Beat
  per environment. Publication uses that same pinned connection, so loss of its
  database session prevents further publication and stops the scheduler. The report
  poller receives the configured report interval. Periodic timer state remains in
  Celery; already-enqueued jobs and their ETAs are durable across scheduler restarts.

Delivery is **at least once**. A crash after an external effect but before the
acknowledgement can repeat the effect. Task handlers must retain their existing
idempotency protections; a queue cannot make email, SMS or S3 effects exactly once.
Hasura writes and queue writes are in different databases, so they are not one
atomic transaction. The adapter exposes transactional enqueue for callers operating
inside the queue database; a future cross-database atomic workflow needs an outbox.

Polling every 200 ms supplies wakeups without depending on `LISTEN/NOTIFY` delivery.
The workload is intentionally small; no partitioning or additional scheduler service
is introduced. Archive retention is an operational responsibility: inspect failures
before purging old archive rows under the environment's retention policy. Backups
contain task payloads and must receive the same protection as the source data.

## Operations and verification

`pgmq.metrics(queue_name)` reports queue depth and message age. To derive a physical
queue name in SQL, for example:

```sql
SELECT 'step_' || left(encode(sha256(convert_to(
  'dev_reports_queue', 'UTF8')), 'hex'), 40) AS queue_name;
SELECT * FROM pgmq.list_queues();
```

Archive tables are `pgmq.a_<physical_queue_name>`. Inspect message headers for the
Celery task ID and correlate with task-execution records. Replay only deliberately
selected failed work, accounting for its original expiry, retry count and possible
completed side effects. Do not replay all archived rows: successful jobs are also
archived.

Production workers and Beat require restart supervision. Development Compose wraps
Windmill and Beat in a five-second restart loop under cargo-watch, so an exhausted
reconnect budget or lost lease starts a fresh process even without a source change.
The watcher retains process-group termination so old leased tasks stop before replacement.
Broker errors expose SQLSTATE or timeout/connection categories without database payloads.
For SQLSTATE 53100, restore database disk headroom before expecting recovery.
Keep compiler caches from exhausting the development database filesystem. Readiness checks verify PostgreSQL and the active subscribed consumers,
not merely the container state.

The focused suite runs against a disposable database initialized from the same SQL:

```sh
cargo test -p pgmq-broker -- --include-ignored --test-threads=1
cargo test -p windmill --lib \
  tasks::electoral_log::pgmq_tests::batch_handoff_is_atomic_and_quarantines_invalid_events \
  -- --ignored --exact
```

Set `PGMQ_TEST_DATABASE_URL` through the test environment, not committed configuration.
CI creates a separate `pgmq_test` database and runs these checks explicitly. They
cover real worker/Beat delivery, retry, expiry, transaction rollback, lease renewal,
claim fencing, abandoned-claim recovery, malformed messages and atomic batch handoff.
The Java publisher has two focused transaction/envelope tests.
From the repository root, run the development supervision regression with
`python3 .devcontainer/test-restart-worker.py`. It checks restart after failure
and termination of the complete worker process group. These contracts do
not execute every election business operation or certify external-effect idempotency.
