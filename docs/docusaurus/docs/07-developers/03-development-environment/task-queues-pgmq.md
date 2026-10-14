---
id: task-queues-pgmq
title: PostgreSQL task queues
sidebar_label: Task queues (PGMQ)
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Windmill, Harvest, Beat and Keycloak use **PGMQ 1.13.0** for durable task delivery,
in a PostgreSQL database of the environment's own: its **task-queue database**. No
separate broker service is needed. The Celery task API, task registrations, routes,
retry policies, time limits and task-execution records are the same as with RabbitMQ.

## The task-queue database

Each environment has one task-queue database, separate from Keycloak's and Hasura's
databases. In a deployment it is a database of the cluster's PostgreSQL server, like
the environment's other databases; in development it is `dev_queues` on the
`postgres` service. It holds the `pgmq` schema, with two tables per queue
(`pgmq.q_<queue>` for waiting messages and `pgmq.a_<queue>` for processed ones), and
the `step_queue.installation` table, which records the environment the database
belongs to.

Because the database belongs to one environment, queues have the same plain names in
every environment:

| Queue | Consumer |
| --- | --- |
| `beat` | Scheduled election and board work, and the archive purge |
| `short_queue` | Short background tasks |
| `communication_queue` | Communication and cast-vote tasks |
| `tally_queue` | Tally execution |
| `reports_queue` | Reports, rendering and post-tally work |
| `import_export_queue` | Imports, exports and voter bulk operations |
| `monitoring_queue` | Monitoring snapshot refreshes, one task per election event |
| `electoral_log_beat_queue` | Electoral-log batch dispatcher, and the signing and audit log outboxes (`post_signing_log_outbox`, `post_audit_log_outbox`) |
| `electoral_log_batch_queue` | Electoral-log batch processing |
| `electoral_log_event_queue` | Raw events; **batch dispatcher only** |
| `electoral_log_dead_letter_queue` | None; electoral-log events set aside for inspection and replay |

Windmill refuses to consume the raw-event and dead-letter queues: a worker consuming
them would discard their events. Queue names given to `windmill consume -q` may still
carry the `<ENV_SLUG>_` prefix they had with RabbitMQ; it is removed.

Each component connects with a role of its own:

| Role | Used by | Privileges |
| --- | --- | --- |
| Owner | The setup job | Owns the database; installs PGMQ and creates the queues |
| Worker | Windmill and Beat | Reads, sends, archives and deletes messages; purges archives |
| Producer | Harvest and Keycloak | Only sends messages |
| Reader | Harvest's Task Queues page | Reads queues, archives and their metrics |

The connection is configured with `QUEUE_DB__HOST`, `QUEUE_DB__PORT`,
`QUEUE_DB__DBNAME`, `QUEUE_DB__USER`, `QUEUE_DB__PASSWORD` and, optionally,
`QUEUE_DB__SSL_MODE` (`Disable`, `Prefer` or `Require`) and `QUEUE_DB_CA_PATH`.
Windmill, Beat and Harvest need the CA with `Prefer` or `Require`; with `Require` and
a CA, Keycloak verifies the server's certificate and host name. The user is the
component's role. Keycloak also reads `QUEUE_DB__POOL__MAX_SIZE` (10 by default) for
its connection pool. Every service needs `ENV_SLUG`: a service refuses a
task-queue database that is not set up, or that belongs to another environment, and
does not create queues.

## Setting up a task-queue database

1. **Provision** the database and the four roles, with the owner role owning the
   database. In a deployment the infrastructure code does this, per environment; in
   development `.devcontainer/postgresql/init-task-queues.sh` does it when the
   `postgres` volume is created. A volume created before the task queues, also on a
   remote or airgap host, needs the script run once by hand, and then the setup; both
   are idempotent. On a remote host, first add the `QUEUE_DB_*` settings of
   `.env.remote-deployment.example` to its `.env`, and add
   `-f docker-compose-remote.yml` to the `docker compose` commands.

   ```sh
   docker compose up -d postgres
   docker exec postgres sh /docker-entrypoint-initdb.d/30-task-queues.sh
   docker compose up task-queues-setup
   ```
2. **Set up** the database as its owner:

   ```sh
   ENV_SLUG=<environment> \
   QUEUE_DB__HOST=... QUEUE_DB__DBNAME=... \
   QUEUE_DB__USER=<owner> QUEUE_DB__PASSWORD=... \
   QUEUE_DB_WORKER_ROLE=<worker> QUEUE_DB_PRODUCER_ROLE=<producer> \
   QUEUE_DB_READER_ROLE=<reader> \
   main setup-queue-database
   ```

   `setup-queue-database` is a command of Windmill's `main` binary, so the setup job
   uses the Windmill image of the release being deployed. It installs PGMQ if
   the database does not have it, records the environment, creates every queue,
   grants each role its privileges, and revokes access to the database from other
   roles of the server. It runs in one transaction and can run again: run it on every
   upgrade, so that queues added by a release exist before its services start. In
   development the `task-queues-setup` Compose service runs it when the stack starts,
   and Windmill and Beat retry until it has run; run
   `docker compose up task-queues-setup` after adding a queue.

The PGMQ SQL distribution and its license are unchanged from
[PGMQ v1.13.0](https://github.com/pgmq/pgmq/tree/v1.13.0), in
`packages/pgmq-broker/sql/`, and are compiled into Windmill. The setup refuses a
database whose recorded PGMQ version differs from the release's: upgrading PGMQ needs
upstream's versioned upgrade scripts. Keep `synchronous_commit` enabled, and back up
the task-queue database like the environment's other databases. Backups contain task
payloads and need the same protection as the source data.

## Delivery and scheduling guarantees

* Claims have a 60-second visibility lease, renewed every 10 seconds throughout
  local waits and execution. Long tally and report tasks keep their time limits.
* Workers acknowledge **after execution**. Acknowledgement, renewal and retry are
  fenced by the message ID, read count and unexpired lease, so an old worker cannot
  acknowledge a newer claim. Enqueueing and database terminal operations have a
  five-second timeout. A worker exits if it loses a lease or cannot renew it for 40
  seconds; its supervisor must restart it. Unfinished messages become visible when their leases
  expire. A message whose deliveries ended five times without an outcome, because
  its worker crashed or lost the lease, is archived as `failed` instead of being
  delivered again.
* A Celery retry updates the same queue row and keeps the task ID. Retry delays and
  ETAs are stored in PostgreSQL; delayed tasks do not hold worker capacity.
* Acknowledged messages move to the queue's archive with their outcome in the
  `x-step-outcome` header: `succeeded`, `failed` (failed and not retried), `expired`
  or `rejected` (the message could not be decoded or names an unknown task). The
  outcome is the task's, as Celery saw it; consult task-execution records and worker
  logs for business results.
* Raw audit events are promoted into one batch task and removed from the event queue
  in **one database transaction**. Failure or cancellation rolls back the whole
  handoff. Raw events that cannot be parsed move to `electoral_log_dead_letter_queue`
  in the same transaction, without blocking later events. A batch closes at
  `ELECTORAL_LOG_BATCH_SIZE` events or `ELECTORAL_LOG_BATCH_MAX_BYTES` bytes; messages
  read beyond the byte limit become visible again for the next batch.
* Keycloak's events are enqueued when Keycloak commits the request that produced
  them, as `ELECTORAL_LOG_PUBLISH_FAILURE_POLICY` says:
  * `fail-request` (the default) enqueues the event just before Keycloak commits. If
    that fails, the request fails and Keycloak rolls it back, so no committed request
    lacks its event. If Keycloak's own commit fails after the event was enqueued, the
    log holds the event of a request that did not complete.
  * `log-and-continue` enqueues the event after Keycloak commits. If that fails, the
    request still succeeds and the event is only in Keycloak's log, with the message
    to send to `electoral_log_event_queue`.

  A request that Keycloak rolls back enqueues nothing. Keycloak 26.6.1 normally emits
  error events in a separate transaction, so ordinary `LOGIN_ERROR` events survive
  the rollback of the original request.
* Beat keeps its schedules. A PostgreSQL advisory lock in the task-queue database
  admits one Beat per environment, and Beat publishes on that same connection, so
  losing it stops the scheduler. Another Beat, such as a rolling update's, waits as a
  ready standby until the lock is free. The server ends the leader's session 30
  seconds after it stops answering (`idle_session_timeout`), so a Beat whose node
  died does not keep the lock. Enqueued tasks and their ETAs survive scheduler
  restarts.

Delivery is **at least once**. A crash after an external effect but before the
acknowledgement can repeat the effect, so task handlers keep their idempotency
protections; a queue cannot make email, SMS or S3 effects exactly once. Hasura,
Keycloak and the task queues are different databases, so a write to one and an
enqueue are not one transaction.

Workers poll each consumed queue every 200 ms when it is empty.

## Archive retention

Beat schedules `purge_queue_archives` every `QUEUE_ARCHIVE_PURGE_INTERVAL_SECS`
seconds (3600 by default). It deletes archived messages older than
`QUEUE_ARCHIVE_RETENTION_HOURS` hours (168, seven days, by default), a few thousand
rows per statement. It never deletes waiting messages. From the archive of the
electoral-log raw-event queue it deletes only messages archived as `succeeded`, and from
the batch queue's also those archived as `expired`, which are sequencer runs that never
ran and carry no events. Any other message, one that failed, was rejected or has no
outcome, such as a batch that could be neither stored nor dead-lettered, stays in its
archive until an operator sends it again or deletes it. The dead-letter queue's archive
is never purged. Beat
refuses to start with an invalid interval, and a worker consuming `beat` refuses to
start with an invalid retention.

## Migrating an environment from RabbitMQ

Environments move to the task queues one at a time, with the release that contains
them:

1. Provision the environment's task-queue database and roles, and their secrets.
2. Deploy the release with the task-queue settings, running the setup job before the
   services start.
3. Messages still in the environment's RabbitMQ queues are **not** migrated: they are
   discarded. To keep work in progress, let the queues drain before upgrading.
4. Remove RabbitMQ from a cluster once none of its environments uses it.

## The Task Queues page

The super-admin tenant's **Task Queues** page, in the Admin Portal, shows every queue's
depth and recent outcomes, the throughput and latency of a queue over time, and its
messages. Harvest serves it through the `task_queues_overview`,
`task_queues_throughput`, `task_queues_messages`, `task_queues_dead_letters` and
`task_queues_query` Hasura actions, to users of the super-admin tenant with
`task-queues-read`.

Harvest reads the task-queue database as the reader role, from
`QUEUE_DB_READER_USER` and `QUEUE_DB_READER_PASSWORD` with the other `QUEUE_DB__*`
settings; without them the page reports that inspection is not configured. Throughput
comes from the archives, so it covers the archive retention. Messages are summarized
from their Celery envelope: the task, task ID, retries, ETA, expiry and size, the
`x-step-outcome`, `x-electoral-log-stage` and `x-electoral-log-error` headers, and, for
electoral-log events, the tenant, election event and message type. These lists never
return task arguments.

With `task-queues-write`, operators replay or discard selected messages of
`electoral_log_dead_letter_queue`. Harvest only enqueues
`manage_electoral_log_dead_letters`, on `short_queue`, with the user who asked; a
worker applies it in one transaction. A replay sends each event back to
`electoral_log_event_queue` and deletes it from the dead letters; a discard archives it
with the `discarded` outcome. The dead-letter archive is never purged.

With `task-queues-query`, users run read-only SQL queries on the task-queue database
from the page's **Query** tab, through `task_queues_query`. Harvest runs each query as
the reader role in a read-only transaction, with a 30-second timeout and at most 1,000
rows, and logs it with the user who ran it. The reader role reads every queue's
`pgmq.q_<queue>` and `pgmq.a_<queue>` tables, so queries read messages with their
tasks' arguments.

## Operations and verification

```sql
-- Depth and age of every queue.
SELECT * FROM pgmq.metrics_all();
-- Outcomes of the last hour's processed messages of a queue.
SELECT headers->>'x-step-outcome' AS outcome, count(*)
FROM pgmq.a_reports_queue
WHERE archived_at > now() - interval '1 hour'
GROUP BY 1;
```

Message headers carry the Celery task ID, to correlate with task-execution records.
Replay only deliberately selected failed work, accounting for its original expiry,
retry count and possible completed side effects; successful tasks are archived too.

Production workers and Beat need restart supervision. Development Compose wraps
Windmill and Beat in a five-second restart loop under cargo-watch, so an exhausted
reconnect budget or lost lease starts a fresh process even without a source change.
Broker errors expose SQLSTATE or timeout and connection categories, without database
payloads. For SQLSTATE 53100, restore database disk headroom before expecting
recovery.

The focused suites run against a disposable database, which they set up themselves:

```sh
PGMQ_TEST_DATABASE_URL=postgres://... cargo test -p pgmq-broker -- --include-ignored
PGMQ_TEST_DATABASE_URL=postgres://... cargo test -p windmill --lib \
  pgmq_tests -- --ignored --test-threads=1
```

Set `PGMQ_TEST_DATABASE_URL` through the test environment, not committed
configuration; the role test needs a user that may create roles. CI creates a separate
`pgmq_test` database and runs these checks; `step-dev test` runs them as the
`cargo-test-db:pgmq-broker` check, against a `pgmq_test` database on `127.0.0.1:5432`
with a `test` role. They cover setup, the environment guard,
role privileges, worker and Beat delivery, retry, expiry, outcomes, archive purge,
transaction rollback, lease renewal, claim fencing, abandoned-claim recovery,
malformed messages, atomic batch handoff, dead letters and batch limits. Keycloak's
publisher tests cover both failure policies, and a shared fixture,
`electoral-log-event-envelope.json`, checks that the envelope Keycloak writes is the
one Windmill reads. From the repository root,
`python3 .devcontainer/test-restart-worker.py` checks the development restart loop.
