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


## Electoral-log entries waiting for their signer

When an election event's log is signed by the key service and the service is
down, the log's entries wait instead of being lost: events on
`electoral_log_event_queue`, due later with a capped backoff
(`ELECTORAL_LOG_DEFER_FIRST_SECS`, `ELECTORAL_LOG_DEFER_MAX_SECS`), and a row each
in `step_queue.electoral_log_waiting`, which only the worker role writes. Run the
queue setup again after upgrading: it creates that table and the deferral-header
indexes.

While an event's log has waiting entries, actions that must record a direct entry
in that log fail visibly (election open, close and publish, secret-attribute
actions, readiness, ceremonies, configuration imports, among others) with "the
electoral log of `<tenant>/<event>` has N entries waiting for its signer; this
entry is refused until they are stored". They work again once the waiting entries
are stored after the key service is back. Watch the "Electoral-log events wait for
their signer" log line: it reports the count and the oldest wait, and becomes an
error after `ELECTORAL_LOG_DEFER_ALERT_SECS`. Do not discard waiting or
dead-lettered entries to unblock a log unless losing them is intended.

Unsigned electoral-log statements that best-effort producers (Windmill and Harvest:
refused-ballot audits, ballot-publication failures, enrollment problems) record while
the signer is unavailable go to `sequent_backend.electoral_log_outbox` with an
HMAC-SHA256 under the tenant's `electoral-log-pending-mac:<tenant id>` vault secret. The relay and
the batch task refuse a statement without a valid MAC, so it is never signed.

Trust boundary: the MAC key lives in the vault, encrypted under `MASTER_SECRET`; the
encrypted vault is in the database, the master key is not. A database-only compromise
(including the shared `postgres` superuser Keycloak connects as in the EMS) can't read
the MAC key or forge pending rows. Whoever administers the hosts (who can read the
environment or memory of Windmill or Harvest, or the secrets file) is outside the
boundary and trusted; nothing here protects against them. The per-install generated
`MASTER_SECRET` of the EMS is a temporary demo boundary until meta#15035's key-inventory
requirements are implemented: master-key custody in the HSM is later work within
meta#15035.

Certificate-authority imports and deletions sign their electoral-log entry with the
administrator's key and write it to `sequent_backend.electoral_log_outbox` in their
own transaction (a Hasura migration): if it can't be written, the action fails and
nothing is saved; if the action rolls back, there is no entry. The audit-log outbox's
beat relays committed entries to the electoral log, announcing the administrator's
key in the tenant's log first. An entry whose key the tenant's log doesn't announce
waits (alert class `electoral-log-admin-key-unannounced`) and holds its log's direct
posts; an operator may cancel it with `cancel_waiting_entry` (naming who and why;
logged under `electoral-log-entry-cancelled`). An outbox row that keeps failing to be
relayed is reported under `electoral-log-outbox-stuck`.

Every outbox row, administrator-signed too, carries a MAC under the tenant's
`electoral-log-pending-mac:<tenant id>` key, written in the action's transaction over the
row's delivery ID and statement: a copy of a row under another delivery ID, or a changed
payload, never verifies. When a row was written is the database's (a trigger sets and
keeps `created_at`; the batch task checks the statement's time against it). An
administrator's signature may be in one outbox row only: a second row with the same
signed statement is refused at insert.

A row the relay can never move as it is (unreadable, a MAC or announcement that doesn't
verify, a statement of another log than its row) is dead-lettered at once: kept, logged
under `electoral-log-outbox-dead-letter`, and no longer holding its log, whose later
entries go on. Once its cause is fixed (a restored vault key, say), replay it with
`windmill replay-electoral-log-outbox-entry --delivery-id <id> --by <who> --reason
<why>`, or give it up with `windmill cancel-electoral-log-entry`. A replayed entry is
stored after those relayed meanwhile. Other failures (the database, the vault, the board)
hold the log and are retried, with `electoral-log-outbox-stuck` after five attempts.

After a certificate import or deletion commits, the action relays the outbox itself,
waiting at most 5 seconds, and asks for a batch dispatch; beat does both every 5 seconds
anyway. With the signer up, the entry is stored within seconds (the relay, then one
batch). Until then, a direct post to that event's log (opening voting, say) is refused
with: "the electoral log of `<tenant id>/<election event id>` has `N` entries waiting
for its signer; this entry is refused until they are stored (the log keeps its order)".
Try again after a few seconds; if it persists, the signer is unavailable (its alerts
say why) or the entry is held (`electoral-log-outbox-stuck`,
`electoral-log-outbox-dead-letter`, `electoral-log-admin-key-unannounced`).

Harvest connects as the producer role and posts some entries itself, so the setup
grants that role `SELECT (scope)` on `step_queue.electoral_log_waiting`: it can tell
whether a log has waiting entries, but not read or write them. Deploy in this order:
run the queue setup, then the workers, then Harvest. A worker of an earlier release
dead-letters a pending statement's reference (it reads it as a malformed record);
its scope stays held until that dead letter is replayed by an updated worker.

Not gated, by design: the ballot-box sequencer, the signing-log and audit-log
outboxes, sealed ballot boxes, CSV imports of a log, and logs made from a signer
alone (the batch task, tenant logs).

A waiting row past due whose copy is no longer on the event queue holds its log:
the dispatcher logs it with the alert class `electoral-log-orphan-waiting`. Replay
its dead letter if it has one; to give the entry up, remove its row
(`DELETE FROM step_queue.electoral_log_waiting WHERE delivery_id = '<id>'`) as the
task-queue database owner, and record why.

The integration tests of the queues, the outbox relay and the pending statements
(`cargo test -p windmill --lib -- --ignored`) need disposable databases (they can
be one database):

- `PGMQ_TEST_DATABASE_URL`: with PGMQ (the task-queue tests set it up).
- `HASURA_TEST_DATABASE_URL`: with every Hasura migration applied
  (`hasura/migrations/backend-db`).
- `ELECTORAL_LOG_TEST_DATABASE_URL`: for the electoral-log boards.
- The vault's settings: `SECRETS_BACKEND=EnvVarMasterSecret` and a `MASTER_SECRET`.
- `HASURA_TEST_TENANT_ID` (optional; by default the bundled tenant). The tests
  insert the tenant row the vault's secrets reference when it is missing.

The checkpoint-copy tests also need the S3 settings (`AWS_REGION` and the bucket's).

The worker-only `step_queue.electoral_log_waiting` table is worker-only only where
the queue roles are separate (`queue_producer`, `queue_worker`, `queue_reader`, as
the EMS and the offline server configure them); a deployment that connects every
service as the queue owner has no such boundary.

## Remaining meta#15035 work

- Master-key (`MASTER_SECRET`) custody in the HSM, replacing the per-install generated
  value, a temporary demo boundary.
- Any further items of meta#15035's key inventory not listed here are pending too; this
  page claims no completion of meta#15035.
