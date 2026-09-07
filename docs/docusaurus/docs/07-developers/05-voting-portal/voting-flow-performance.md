---
id: voting_flow_performance
title: Voting flow performance
sidebar_position: 4
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Voting flow performance

The authenticated voting path keeps database work bounded by the selected
election, rather than by the number of scheduled tasks in its event. It reads
current election policy and a transactionally maintained voting-window projection,
then inserts the ballot under a per-voter eligibility lock.

## Database work per ordinary cast

The following comparison counts successful, retry-free casts. Reads include the
SQL SELECTs inside the eligibility trigger, not physical page reads or internal
constraint checks. Asynchronous audit delivery, cryptography, and HTTP transport
are outside these counts and the SQL benchmark.

| Work | Previous path | Current path |
|---|---:|---:|
| Read statements | 12 | 6 |
| Synchronous ballot writes | 1 | 1 |
| Application transactions | 3 | 1 |
| Logical connection checkouts | 3 | 1 |
| Databases used synchronously | 2 | 1 |

The previous path loaded signing secrets three times, fetched the election twice,
loaded all non-archived scheduled tasks, and fetched previous ballot contents
before repeating eligibility checks in the INSERT trigger. It then looked up the
username in Keycloak and reloaded signing context in a separate transaction.

The current path reads area, election event, signing secret and election policy
once each. The INSERT trigger adds two reads: one aggregate for prior-vote count
and cross-area exclusivity, and one election revote-limit lookup. The verified
JWT supplies the optional username. Loaded signing context is reused for the
existing post-commit asynchronous audit message.

:::note Datafix and audit semantics
Datafix keeps its separate voter lease and connection lifecycle; the one-transaction
count applies to ordinary casts. Audit delivery remains asynchronous and
best-effort after commit. An accepted ballot does not imply an atomic audit write.
:::

## Materialized voting windows

`sequent_backend.election_voting_window` stores opening and closing date strings,
keyed by `(tenant_id, election_event_id, election_id)`. This is a small projection
table maintained by database triggers, rather than a view refreshed on a timer.

Every relevant schedule INSERT, UPDATE or DELETE refreshes the affected scope in
the **same transaction** as the source change. Moving a task refreshes both its
old and new scopes. Archiving removes its contribution; restoring it restores the
date. Executed/stopped tasks continue defining the deadline. A source TRUNCATE
clears the projection. Bookkeeping updates that do not affect dates skip refresh.

Configuration writers serialize per election with a transaction advisory lock.
The refresh query runs after acquiring that lock, so concurrent changes to
opening and closing tasks cannot overwrite each other's committed state. A
rollback restores source and projection together. Votes do not acquire this
configuration lock; they read committed data from the writer.

The projection does not depend on election insertion order: importing schedules
before their election is supported. It matches the canonical task names and
exact election payload used by schedule generation. Duplicate active endpoints
and malformed date configuration are rejected instead of choosing an arbitrary
closing time. Migration backfill uses the same validation under a source-table
write lock and fails atomically if existing configuration needs repair.

The cast-vote query joins the election and projection on the full tenant/event/
election key. This is one result row joining two rows by their full keys, with no
scheduled-event scan or per-vote window derivation. Election status, channel
settings and presentation are still read from the election row, so administrative
pauses and policy changes remain authoritative.

Current time, authentication time and grace-period rules are evaluated for every
submission. Without grace, the existing comparison accepts a vote at the exact
closing instant and rejects one after it. Materialization never stores an
"accepted" decision across requests. Malformed election presentation returns an
internal configuration error; only missing presentation uses defaults.

## Eligibility, signing and response handling

The revote trigger holds the existing per-voter advisory lock while counting
`valid` and `in-progress` ballots and checking cross-area exclusivity in a single
aggregate. Discarded ballots do not consume eligibility. Unlimited revotes still
cannot cross areas. INSERT remains the final SQL statement before COMMIT so that
the per-voter lock does not cover audit delivery or unrelated database work.

The covering participation index includes both `status` and `area_id`, which are
needed by this combined aggregate. Index-only scans still perform heap visibility
checks on recently changed pages; vacuum and the visibility map determine whether
heap reads can actually be avoided.

The audit API distinguishes system-authored messages
(`for_system_with_signing_key`) from voter-attributed messages
(`for_voter_with_signing_key`). Both reuse the already loaded election system key,
preserving the previous signature and actor conventions without another lookup.

INSERT does not return ciphertext from PostgreSQL. The response uses the submitted
content, preserving its API shape without reading TOAST data back immediately.
`STORAGE EXTERNAL` skips compression for future ciphertext writes. The portal also
omits unused election EML; signed ballot-style data remains available as before.

## Local regression and benchmark workflow

Inside the dev container, from `/workspaces/step`, run:

```sh
devenv shell python3 scripts/test_cast_vote_scalability.py \
  --rust-tests --benchmark --output /tmp/voting-flow-results.json
```

Everything runs inside devenv. The harness creates a disposable PostgreSQL cluster
with a private Unix socket, applies the actual migrations, and tears it down on
completion. It needs no deployed environment, voter credentials or external service.
The PostgreSQL driver and formatter are supplied by `devenv.nix`.

The regression suite tests bounded/unlimited concurrent revotes, cross-area
exclusivity, discarded ballots, concurrent schedule creation and rescheduling,
archive/move/delete behavior, import order, rollback, malformed configuration,
tenant scope, storage migration and interrupted concurrent index builds. Rust
integration tests exercise the production INSERT/error mapping and materialized
configuration query, including equivalence with the schedule-generation contract.

The SQL benchmark compares the previous implementation at `e93ca05104` with the
current database path. Each variant starts with 100,000 seeded ballots, approximately
21 KB encoded ciphertext, two prior ballots per voter, warm reusable connections,
and the same fixture schema. It runs opening/lull/closing concurrency phases of
8/2/8 clients and 128/64/128 requests, with 100 and 2,000 unrelated scheduled tasks.
This is a bounded-concurrency workload, not an open-loop arrival-rate test.

PostgreSQL `pg_stat_statements` counts reads, writes and transaction starts,
including nested trigger SQL. The driver records logical connection checkouts.
The baseline uses a separate local Keycloak fixture database; the current path
does not. The complete report includes query text, counts, per-phase throughput,
p50/p99 latency, PostgreSQL version and machine information.

The benchmark deliberately excludes HTTP, Rust cryptography, broker delivery,
Datafix and application rendering. Its fixtures model the database operations and
use the production policy query and trigger definitions; it is not a full platform
capacity test. Do not present its latency as an end-to-end voting latency or a
production service-level guarantee. Use it to compare database work reproducibly.

A checked-in [reference report](/benchmarks/voting-flow.json) records one local run.

## Migration and recovery

Deploy the cross-area trigger, EXTERNAL storage and voting-window migrations
before the application that uses them. The projection migration is transactional:
backfill and trigger installation become visible together. It requires a brief
configuration-write lock while backfilling, so apply it before voting peaks.
Do not modify the internal projection directly or disable its maintenance triggers.

Run the covering-index replacement separately against the intended writer:

```sh
psql -X -v ON_ERROR_STOP=1 -f scripts/postgres/cast_vote_covering_index.sql
```

Do not wrap it in a transaction. The script sets stop-on-error internally, builds
the replacement concurrently while the old index remains usable, then retires the
old index and restores its name. On interruption, inspect `pg_index.indisvalid`:
remove an invalid partial build concurrently and retry; if the new index is valid,
resume the remaining steps instead of blindly rerunning CREATE. The regression
suite verifies that an invalid leftover cannot cause the valid old index to be
retired. To undo the index change, use the same build/retire/rename sequence with
the original four-key index definition.

Restore the old application before removing the projection it queries. Rolling
back the cross-area trigger removes database cross-area enforcement entirely;
the current application has no duplicate precheck. The previous application had
a cross-area concurrency race, so retaining the strengthened trigger is preferable.
Storage rollback changes future writes and does not rewrite existing ballots.
