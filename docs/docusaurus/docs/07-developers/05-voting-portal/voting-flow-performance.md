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
current database path using the same fixture schema and approximately 21 KB
encoded ciphertext. It varies three factors independently, then combines table
growth and a voter burst:

| Scenario | Seeded ballots | Peak concurrent voters | Unrelated schedules |
|---|---:|---:|---:|
| 10k votes table | 10,000 | 8 | 100 |
| 100k votes table | 100,000 | 8 | 100 |
| 1M votes table | 1,000,000 | 8 | 100 |
| 100k votes table, 32 concurrent voters | 100,000 | 32 | 100 |
| 100k votes table, 64 concurrent voters | 100,000 | 64 | 100 |
| 1M votes table, 64 concurrent voters | 1,000,000 | 64 | 100 |
| 100k votes table, 2k schedules | 100,000 | 8 | 2,000 |

Every variant starts with the same population, two prior ballots per voter and
64 warmup requests on reusable connections. Opening/lull/closing phases submit
1,024/512/1,024 requests. Lull concurrency is one quarter of peak, with a minimum
of two. All 2,560 measured requests use different voters, also distinct from the
warmups. A deterministic permutation spreads those voters across the entire
seeded population, so the 1M votes table case exercises more than its first few
participation-index pages. The report records actual relation size including
indexes and TOAST, accepted requests, errors and per-phase throughput/latency.

Seeding, vacuuming, index construction and verification run outside the measured
interval. The harness checks the initial and final ballot counts and fails if a
submission fails. Each scenario restores the seed before comparing variants.
PostgreSQL allows 160 connections locally to accommodate the largest baseline's
64 writer and 64 identity connections; durability settings remain enabled.

This is a bounded-concurrency workload, not an open-loop arrival-rate test. Peak
concurrency means different voters submit simultaneously, not 64 submissions
contending for one voter's lock. Same-voter races are covered by regressions.
The 1M votes fixture needs roughly 25 GB of temporary database storage; allow at
least 50 GB free for its data, indexes and WAL. Runs can take several minutes.
Use `--benchmark --scenario 100k-votes` to rerun one case, or repeat `--scenario`
to select several; omitting it runs the full matrix.

PostgreSQL `pg_stat_statements` counts reads, writes and transaction starts,
including nested trigger SQL. The driver records logical connection checkouts.
The baseline uses a separate local Keycloak fixture database; the current path
does not. The complete report includes query text, counts, per-phase throughput,
p50/p99 latency, PostgreSQL version and machine information.

The benchmark deliberately excludes HTTP, Rust cryptography, broker delivery,
Datafix and application rendering. Its fixtures model the database operations and
use the production policy query and trigger definitions; it is not a full platform
capacity test. The Python driver shares one interpreter across worker threads,
so throughput can be limited by client processing as well as PostgreSQL.
Do not present its latency as an end-to-end voting latency or a
production service-level guarantee. Use it to compare database work reproducibly.

## Reference measurements

The [reference report](/benchmarks/voting-flow.json) contains the complete query
counts, per-phase measurements, relation sizes and PostgreSQL settings. The tables
below are generated from that same report. To publish a new reference run:

```sh
devenv shell python3 scripts/test_cast_vote_scalability.py --benchmark \
  --output docs/docusaurus/static/benchmarks/voting-flow.json
devenv shell python3 scripts/voting_flow/report.py
```

<!-- voting-flow-benchmark:start -->

SQL-only measurements at implementation `f562c7deae` against baseline `e93ca05104`.

**Accepted votes/second = accepted submissions / elapsed measurement seconds.** Elapsed time is the sum of the opening, lull and closing phase wall times; seeding and warmups are excluded. Each measured submission uses a distinct voter.

For the 100k votes table, 8 concurrent voters, 100 schedules workload, before: 2,560 / 4.7475 s = **539.2 votes/s**. After: 2,560 / 1.6752 s = **1528.2 votes/s**. Calculations use unrounded durations from the JSON; displayed durations are rounded.

| Scenario | Ballots | Peak concurrent voters | Schedules | Before p50 / p99 (ms) | After p50 / p99 (ms) |
|---|---:|---:|---:|---:|---:|
| 10k votes table, 8 concurrent voters, 100 schedules | 10,000 | 8 | 100 | 15.06 / 113.77 | 4.78 / 10.87 |
| 100k votes table, 8 concurrent voters, 100 schedules | 100,000 | 8 | 100 | 14.91 / 19.62 | 4.75 / 7.99 |
| 1M votes table, 8 concurrent voters, 100 schedules | 1,000,000 | 8 | 100 | 14.86 / 19.30 | 4.77 / 9.27 |
| 100k votes table, 32 concurrent voters, 100 schedules | 100,000 | 32 | 100 | 61.53 / 78.88 | 19.69 / 29.48 |
| 100k votes table, 64 concurrent voters, 100 schedules | 100,000 | 64 | 100 | 123.36 / 158.02 | 39.49 / 58.76 |
| 1M votes table, 64 concurrent voters, 100 schedules | 1,000,000 | 64 | 100 | 122.66 / 160.61 | 38.83 / 58.60 |
| 100k votes table, 8 concurrent voters, 2k schedules | 100,000 | 8 | 2,000 | 91.36 / 123.02 | 4.77 / 55.97 |

| Scenario | Before seconds | After seconds | Before votes/s | After votes/s | Accepted per variant | Errors before / after |
|---|---:|---:|---:|---:|---:|---:|
| 10k votes table, 8 concurrent voters, 100 schedules | 5.6889 | 1.7461 | 450.0 | 1466.1 | 2,560 | 0 / 0 |
| 100k votes table, 8 concurrent voters, 100 schedules | 4.7475 | 1.6752 | 539.2 | 1528.2 | 2,560 | 0 / 0 |
| 1M votes table, 8 concurrent voters, 100 schedules | 5.0937 | 2.1244 | 502.6 | 1205.0 | 2,560 | 0 / 0 |
| 100k votes table, 32 concurrent voters, 100 schedules | 5.0857 | 1.6821 | 503.4 | 1521.9 | 2,560 | 0 / 0 |
| 100k votes table, 64 concurrent voters, 100 schedules | 5.1814 | 1.7435 | 494.1 | 1468.3 | 2,560 | 0 / 0 |
| 1M votes table, 64 concurrent voters, 100 schedules | 5.1628 | 1.7198 | 495.9 | 1488.5 | 2,560 | 0 / 0 |
| 100k votes table, 8 concurrent voters, 2k schedules | 30.1351 | 2.1263 | 85.0 | 1204.0 | 2,560 | 0 / 0 |

Latencies cover the complete SQL path per request. Throughput is total completed requests divided by the combined phase wall time, including driver scheduling overhead. These measurements come from one local run, not production capacity estimates or statistical confidence intervals.

<!-- voting-flow-benchmark:end -->

The table-size comparison holds each voter's history at two ballots. Both paths
already have an index keyed by voter, so increasing the global ballot count does
not imply scanning the whole table for every cast. The revised path removes
redundant reads and ciphertext transfers, and covers the combined eligibility
aggregate. Growing one voter's revote history is a different workload.

The concurrency comparison holds the seed at 100,000 ballots and increases
simultaneous distinct voters. Rising p99 at higher concurrency remains possible
even when SQL work per cast falls; inspect throughput alongside latency. The
combined million-ballot/64-voter case checks these two pressures together.

## Schedule placement, filtering and indexing

In the cast benchmark, **unrelated schedules belong to the same tenant and
same election event** as the vote. They are unrelated to the selected voting
window. This is different from schedules belonging to another event or tenant.

The original query requests every non-archived schedule for its tenant/event.
It transfers and decodes all matching rows. An index can locate that scope, but
cannot remove rows the query requests. To fetch fewer rows within a busy event,
the query must also select the two relevant voting-window task IDs.

Schedules in another event or tenant are excluded from the result. Without a
matching index, PostgreSQL can still scan them to find and reject nonmatches.
With a selective indexed lookup, it can go directly to the requested scope.
Schedules in another environment's **separate database** are never scanned by
this query, though those databases can compete for shared CPU, memory and I/O.

The projection migration creates this partial index before backfill:

```sql
CREATE INDEX scheduled_event_active_scope_task_idx
ON sequent_backend.scheduled_event (tenant_id, election_event_id, task_id)
WHERE archived_at IS NULL;
```

Its leading keys serve active tenant/event queries. The task ID key serves
endpoint lookups within that scope. Projection refresh now explicitly filters
both task IDs, so PostgreSQL can use all three keys instead of deriving an
election ID from every candidate task. The existing helper still checks the
exact canonical task name and payload; index filtering does not replace those
validation rules. The index does not include wide JSON payloads.

A direct query joining election policy with two indexed schedule endpoints is
also a viable read strategy. The diagnostic below compares the same policy and
date fields with the production projection query. It tests valid configuration;
it is not a complete replacement for configuration-write validation. The
projection continues to validate duplicate/invalid endpoints at configuration
write time and maintain dates transactionally. The measurements do not establish
that materialization is necessary for fast indexed window reads.

### Schedule query diagnostic

This separate devenv diagnostic places 100,000 extra active schedules in the
same event, another event, or another tenant. Each comparison holds the query
and data constant and changes only the schedule index. It measures the broad
original query, a two-endpoint query, the production projection query, and a
real reschedule UPDATE invoking the production refresh trigger. UPDATE samples
alternate dates so that each exercises maintenance, not the unchanged-value
fast path.

There are three warmups and 21 measured samples per query/placement/index state.
The tables show client p50, including fetching and decoding returned rows.
Raw results also contain every sample and `EXPLAIN (ANALYZE, BUFFERS)` plans.
Automatic prepared-statement caching is disabled to avoid reusing a generic
plan from a different placement. These are single-client query/UPDATE costs,
**not complete cast-vote latency or votes per second**. Index creation and data
seeding are excluded. Both UPDATE variants use the current explicit task-ID
predicates; this isolates the index's benefit rather than the entire change
from the earlier opaque refresh predicate.

```sh
devenv shell python3 scripts/voting_flow/schedules.py \
  --output docs/docusaurus/static/benchmarks/schedule-indexes.json
devenv shell python3 scripts/voting_flow/report.py
```

<!-- schedule-index-benchmark:start -->

Measurements at `5ce21b3e96`. [Raw schedule-query evidence](/benchmarks/schedule-indexes.json).

| Extra schedules in | Rows returned | Broad query without index p50 (ms) | With index p50 (ms) |
|---|---:|---:|---:|
| same event | 100,002 | 635.431 | 642.050 |
| other event | 2 | 5.752 | 5.772 |
| other tenant | 2 | 5.056 | 0.051 |

| Extra schedules in | Two-endpoint query without index p50 (ms) | With index p50 (ms) | Projection p50 (ms) |
|---|---:|---:|---:|
| same event | 13.000 | 0.055 | 0.045 |
| other event | 12.999 | 0.051 | 0.043 |
| other tenant | 13.011 | 0.050 | 0.043 |

| Extra schedules in | Reschedule without index p50 (ms) | With index p50 (ms) |
|---|---:|---:|
| same event | 8.210 | 0.556 |
| other event | 8.076 | 0.598 |
| other tenant | 8.124 | 0.523 |

<!-- schedule-index-benchmark:end -->

## Migration and recovery

Deploy the cross-area trigger, EXTERNAL storage and voting-window migrations
before the application that uses them. The projection migration is transactional:
the schedule index, backfill and trigger installation become visible together.
Index creation and backfill hold a configuration-write lock; apply the migration
before voting peaks and allow time proportional to the schedule table size.
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
The projection down migration also removes the active schedule index. Storage
rollback changes future writes and does not rewrite existing ballots.
