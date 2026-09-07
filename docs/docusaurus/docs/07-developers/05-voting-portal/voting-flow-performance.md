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
encoded ciphertext. Every event has at most **200 elections** and at most
**10 total schedules per election**, including its opening and closing tasks.
The matrix varies votes, concurrent voters and populated areas independently,
then combines the maximum values. Two 200-election cases separate election
cardinality from fetching 400 versus 2,000 schedules:

| Votes table | Peak concurrent voters | Elections per event | Areas | Total schedules per event |
|---|---:|---:|---:|---:|
| 10k | 8 | 10 | 100 | 100 |
| 100k | 8 | 10 | 100 | 100 |
| 1M | 8 | 10 | 100 | 100 |
| 100k | 32 | 10 | 100 | 100 |
| 100k | 64 | 10 | 100 | 100 |
| 1M | 64 | 10 | 100 | 100 |
| 100k | 8 | 200 | 100 | 2,000 |
| 100k | 8 | 200 | 100 | 400 |
| 100k | 8 | 10 | 1,000 | 100 |
| 100k | 8 | 10 | 10,000 | 100 |
| 1M | 64 | 200 | 10,000 | 2,000 |

Each fixture creates actual election rows, canonical opening/closing schedules,
and area rows. Every seeded area and election has prior ballots. Returning
voters are distributed deterministically across the elections and areas and
retain their assigned election/area in the measured submission. Both variants
restore exactly the same mapping; the harness verifies all table cardinalities.
The SQL path reads one area by primary key and checks that voter's history; it
does not enumerate all event areas. More areas can still enlarge the lookup and
ballot-index working sets. Area cardinality is therefore tested independently
rather than assuming that ten times as many areas means ten times more work.

The coverage table records the number of areas and elections actually touched
by measured requests. With 2,560 distinct submissions, the 10k-area fixture
cannot touch every area during measurement, even though all 10,000 areas have
seeded votes. This is a spread-out returning-voter workload, not an exhaustive
area traversal or a test of the portal's full area/election listing queries.

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

The tables and SVG graphs below are committed snapshots. Raw JSON is generated
locally under the ignored `.cache/voting-flow/` directory, outside Docusaurus
static assets. It contains query counts, phase measurements, relation sizes and
PostgreSQL settings; it is neither committed nor shipped with the documentation.
A normal documentation build needs no raw JSON or database benchmark.

To measure a new full matrix and regenerate the tables and graphs together:

```sh
devenv shell python3 scripts/test_cast_vote_scalability.py --benchmark
devenv shell python3 scripts/voting_flow/schedules.py
devenv shell python3 scripts/voting_flow/report.py
```

Use `--output` on either benchmark to keep separate runs, and pass their directory
to `report.py --input-dir PATH`. The renderer expects `voting-flow.json` from a
full matrix and `schedule-indexes.json`; it fails with generation instructions
if either is missing. Running the scripts again produces new measurements,
not byte-for-byte copies of historical timings. Retain local JSON when auditing
a particular run; commit only the resulting documentation and SVGs.

To add the optional release verification section after generating the full
matrix and schedule diagnostic on that checkout:

```sh
devenv shell python3 scripts/test_cast_vote_scalability.py --benchmark \
  --scenario 100k-votes --scenario 100k-votes-1k-areas \
  --scenario 100k-votes-10k-areas \
  --scenario 1m-votes-64-voters-200-elections-10k-areas \
  --output .cache/voting-flow/voting-flow-release-10.json
devenv shell python3 scripts/voting_flow/report.py
```

<!-- voting-flow-benchmark:start -->

SQL-only measurements at implementation `2a17342cc3` against baseline `e93ca05104`.

**Accepted votes/second = accepted submissions / elapsed measurement seconds.** Elapsed time is the sum of the opening, lull and closing phase wall times; seeding and warmups are excluded. Each measured submission uses a distinct voter.

For the 100k votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules workload, before: 2,560 / 4.9338 s = **518.9 votes/s**. After: 2,560 / 1.7021 s = **1504.0 votes/s**. Calculations use unrounded durations from the JSON; displayed durations are rounded.

### Visual comparisons

Each figure holds the dimensions in its subtitle constant. Gray dashed lines show the baseline; teal lines show the revised path. Latency panels use milliseconds (lower is better); throughput uses accepted votes/second (higher is better). All y-axes start at zero. Lines connect tested cases, not predictions between them. The p99 outliers are retained; one run does not establish statistical significance.

![Vote-table size: before and after p50, p99 and accepted votes per second. 8 concurrent voters · 10 elections · 100 areas · 100 total schedules.](/benchmarks/voting-flow-votes.svg)

![Concurrent voters: before and after p50, p99 and accepted votes per second. 100k votes table · 10 elections · 100 areas · 100 total schedules.](/benchmarks/voting-flow-concurrency.svg)

![Populated areas: before and after p50, p99 and accepted votes per second. 100k votes table · 8 concurrent voters · 10 elections · 100 total schedules.](/benchmarks/voting-flow-areas.svg)

![Schedules within a 200-election event: before and after p50, p99 and accepted votes per second. 100k votes table · 8 concurrent voters · 200 elections · 100 areas.](/benchmarks/voting-flow-schedules.svg)

![Combined workload: before and after p50, p99 and accepted votes per second. 1M votes table · 64 concurrent voters · 200 elections · 10k areas · 2k total schedules.](/benchmarks/voting-flow-combined.svg)

### Detailed measurements

| Scenario | Before p50 / p99 (ms) | After p50 / p99 (ms) |
|---|---:|---:|
| 10k votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 14.86 / 20.49 | 4.67 / 9.57 |
| 100k votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 14.87 / 19.00 | 4.77 / 8.96 |
| 1M votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 14.51 / 20.38 | 4.62 / 29.14 |
| 100k votes table, 32 concurrent voters, 10 elections, 100 areas, 100 total schedules | 59.90 / 77.09 | 19.07 / 28.29 |
| 100k votes table, 64 concurrent voters, 10 elections, 100 areas, 100 total schedules | 120.84 / 155.91 | 38.04 / 57.29 |
| 1M votes table, 64 concurrent voters, 10 elections, 100 areas, 100 total schedules | 121.19 / 156.33 | 38.17 / 56.48 |
| 100k votes table, 8 concurrent voters, 200 elections, 100 areas, 2k total schedules | 96.07 / 128.70 | 4.79 / 7.81 |
| 100k votes table, 8 concurrent voters, 200 elections, 100 areas, 400 total schedules | 27.64 / 35.99 | 4.69 / 8.28 |
| 100k votes table, 8 concurrent voters, 10 elections, 1k areas, 100 total schedules | 14.80 / 102.08 | 4.65 / 7.97 |
| 100k votes table, 8 concurrent voters, 10 elections, 10k areas, 100 total schedules | 14.74 / 18.88 | 4.57 / 8.25 |
| 1M votes table, 64 concurrent voters, 200 elections, 10k areas, 2k total schedules | 805.02 / 1032.38 | 38.55 / 58.64 |

| Scenario | Before seconds | After seconds | Before votes/s | After votes/s | Accepted per variant | Errors before / after |
|---|---:|---:|---:|---:|---:|---:|
| 10k votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 4.8921 | 1.6741 | 523.3 | 1529.2 | 2,560 | 0 / 0 |
| 100k votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 4.9338 | 1.7021 | 518.9 | 1504.0 | 2,560 | 0 / 0 |
| 1M votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 5.0648 | 2.0691 | 505.4 | 1237.2 | 2,560 | 0 / 0 |
| 100k votes table, 32 concurrent voters, 10 elections, 100 areas, 100 total schedules | 4.9684 | 1.6681 | 515.3 | 1534.7 | 2,560 | 0 / 0 |
| 100k votes table, 64 concurrent voters, 10 elections, 100 areas, 100 total schedules | 5.0674 | 1.6793 | 505.2 | 1524.5 | 2,560 | 0 / 0 |
| 1M votes table, 64 concurrent voters, 10 elections, 100 areas, 100 total schedules | 5.0790 | 1.6870 | 504.0 | 1517.5 | 2,560 | 0 / 0 |
| 100k votes table, 8 concurrent voters, 200 elections, 100 areas, 2k total schedules | 31.5647 | 1.6987 | 81.1 | 1507.0 | 2,560 | 0 / 0 |
| 100k votes table, 8 concurrent voters, 200 elections, 100 areas, 400 total schedules | 8.9507 | 1.6443 | 286.0 | 1556.9 | 2,560 | 0 / 0 |
| 100k votes table, 8 concurrent voters, 10 elections, 1k areas, 100 total schedules | 5.1667 | 1.6713 | 495.5 | 1531.7 | 2,560 | 0 / 0 |
| 100k votes table, 8 concurrent voters, 10 elections, 10k areas, 100 total schedules | 4.7322 | 1.6366 | 541.0 | 1564.3 | 2,560 | 0 / 0 |
| 1M votes table, 64 concurrent voters, 200 elections, 10k areas, 2k total schedules | 33.3332 | 1.7255 | 76.8 | 1483.6 | 2,560 | 0 / 0 |

| Scenario | Distinct measured voters | Distinct measured elections | Distinct measured areas |
|---|---:|---:|---:|
| 10k votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 2,560 | 10 | 100 |
| 100k votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 2,560 | 10 | 100 |
| 1M votes table, 8 concurrent voters, 10 elections, 100 areas, 100 total schedules | 2,560 | 10 | 100 |
| 100k votes table, 32 concurrent voters, 10 elections, 100 areas, 100 total schedules | 2,560 | 10 | 100 |
| 100k votes table, 64 concurrent voters, 10 elections, 100 areas, 100 total schedules | 2,560 | 10 | 100 |
| 1M votes table, 64 concurrent voters, 10 elections, 100 areas, 100 total schedules | 2,560 | 10 | 100 |
| 100k votes table, 8 concurrent voters, 200 elections, 100 areas, 2k total schedules | 2,560 | 200 | 100 |
| 100k votes table, 8 concurrent voters, 200 elections, 100 areas, 400 total schedules | 2,560 | 200 | 100 |
| 100k votes table, 8 concurrent voters, 10 elections, 1k areas, 100 total schedules | 2,560 | 10 | 1,000 |
| 100k votes table, 8 concurrent voters, 10 elections, 10k areas, 100 total schedules | 2,560 | 10 | 2,560 |
| 1M votes table, 64 concurrent voters, 200 elections, 10k areas, 2k total schedules | 2,560 | 200 | 2,560 |

### Comparing the factors

Holding the 100k votes table, 8 concurrent voters, 10 elections and 100 schedules constant, revised-path p50 was **100 areas: 4.77 ms, 1k areas: 4.65 ms, 10k areas: 4.57 ms**. This tests populated area cardinality for point lookups and per-voter history; it does not measure an area-list response or larger area payloads.

Holding 200 elections, 100 areas, 100k votes and 8 concurrent voters constant, 400 versus 2,000 schedules produced baseline p50 **27.64 versus 96.07 ms**, and revised p50 **4.69 versus 4.79 ms**. The broad baseline query transfers every schedule in the event, while the revised cast reads its election's keyed window. These single-run comparisons show observed sensitivity, not statistical significance or a capacity guarantee.


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

The cast benchmark's schedule counts are **total active schedules in the
same tenant and election event**, including two endpoints for every election.
The largest event contains exactly 200 elections and 2,000 schedules.

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

This separate devenv diagnostic uses a target event with **200 elections and
2,000 total active schedules**. It compares that event alone with 14 additional
events in the same tenant, then with 14 events in different tenants. Every event
has 200 elections and 2,000 schedules: the shared-table cases contain 30,000
schedules in total, and the broad query always returns the target event's 2,000.
These are events in one database, not 15 environment databases on an RDS cluster.
Separate databases cannot add rows scanned by this query.

Each comparison holds query/data constant and changes only the schedule index.
It measures the broad original query, a two-endpoint query, the production
projection query, and a real reschedule UPDATE invoking the production refresh
trigger. UPDATE samples alternate dates so each exercises maintenance, not the
unchanged-value fast path.

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
  --output .cache/voting-flow/schedule-indexes.json
devenv shell python3 scripts/voting_flow/report.py
```

<!-- schedule-index-benchmark:start -->

Measurements at `2a17342cc3`. Raw samples and EXPLAIN plans are generated locally as `schedule-indexes.json`.

![Schedule index comparison: broad event query, two-endpoint query and rescheduling p50, with and without the index.](/benchmarks/schedule-indexes.svg)

The broad query returns 2,000 rows in every case. These are single-query and configuration-update timings, not complete cast latency or votes per second.

| Schedule population | Rows returned | Broad query without index p50 (ms) | With index p50 (ms) |
|---|---:|---:|---:|
| one event | 2,000 | 11.813 | 11.833 |
| 15 events, same tenant | 2,000 | 13.558 | 11.825 |
| 15 events, different tenants | 2,000 | 13.483 | 11.689 |

| Schedule population | Two-endpoint query without index p50 (ms) | With index p50 (ms) | Projection p50 (ms) |
|---|---:|---:|---:|
| one event | 0.308 | 0.141 | 0.107 |
| 15 events, same tenant | 2.562 | 0.135 | 0.097 |
| 15 events, different tenants | 2.585 | 0.139 | 0.096 |

| Schedule population | Reschedule without index p50 (ms) | With index p50 (ms) |
|---|---:|---:|
| one event | 0.828 | 0.647 |
| 15 events, same tenant | 3.282 | 0.754 |
| 15 events, different tenants | 3.253 | 0.684 |

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


## Publication transaction lifecycle

Ballot generation and final publication now acquire a tenant-scoped row lock on
an election event for the lifetime of their database transaction. This supplements
the short task lease: a worker that outlives its lease cannot race another worker
or final publication for the same event. Independent events remain concurrent.
The lock serializes full-event and partial-election publications together, because
both can change which styles remain visible.

Successful generation is immutable: repeated delivery of an already generated
publication does not append a new set of ballot styles. Deleted publications and
published records missing generated content are rejected. Generation propagates
commit failures to task execution instead of reporting success after a failed
commit. Transaction rollback releases the event lock and leaves generation
eligible for retry; final publication rejects deleted drafts.

Validate the production locking SQL with
`devenv shell python3 scripts/test_ballot_publication_lifecycle.py`. The isolated
PostgreSQL check covers contention, tenant isolation, independent events, and
lock release after commit and rollback. The Windmill ballot-style unit tests cover
completed delivery retries and invalid publication states. These checks do not
run the SQL benchmarks.

This is a prerequisite for immutable private-object activation. It does not yet
upload publication objects, change voter authorization, or implement rollback to
an earlier publication. No S3 or bootstrap request-count improvement is measured
by these lifecycle tests.


## Consolidated voter read (database-backed rollout)

The election chooser now issues `GetVoterContext`, which returns ballot styles,
the corresponding election records, event presentation/status, and the current
voter's cast metadata in one GraphQL operation. Elections are reached through a
ballot-style relationship scoped by tenant, event and election, so the client does
not first need to load election IDs and then issue `GetElections`. Empty eligible
styles remain a definitive empty result. Cast ciphertext is not requested.

This increment still returns full ballot EML from PostgreSQL. It has no private
S3 objects or publication references yet, and it still eagerly transfers every
eligible style. It establishes the consolidated application read and preserves
the existing publication format while the immutable-object contract is developed.
The 200-election regression checks one application operation; it does not establish
bounded ballot bytes or publication costs at 100/1k/10k areas.

The hook seeds the scoped election and event cache entries used by review and
confirmation. Existing direct-entry and gold reauthentication queries still work
when those entries are absent. An unresolved cast starts a separate, narrow,
network-backed `GetCastVotes` query and polling; a newly submitted unresolved cast
in Redux also starts that refresh on return to the chooser. Support-material and
receipt reads retain their own policy-driven behavior.

Voter select permissions now require `X-Hasura-Election-Event-Id` on event,
election, ballot-style and cast rows. Election reads also require membership in
`X-Hasura-Authorized-Election-Ids`; style and cast reads retain area restrictions,
and cast metadata retains the voter-ID restriction. Deploy the metadata before
the portal and confirm that the configured voter token mapper emits the event
claim. Missing claims fail closed. Existing publications need no regeneration or
data migration for this increment. To roll back the portal, retain the stricter
permissions: the previous standalone operations remain supported for tokens with
the required claims.

Apollo data is replaced on access-token changes. Redux voter state is also cleared
when identity, client, session, authentication level, tenant, event, area or
permissions change, and on logout. An expiry-only token refresh preserves Redux
ballot selections. The client-side scope comparison only partitions caches;
Hasura validates authentication and enforces authorization. Writer-side cast
acceptance remains authoritative for pauses, channels, schedules and revotes.

Validation includes 86 portal tests, including delayed/empty/200-election bootstrap
responses, scoped cache reuse, event changes, fresh authenticated clients and
session invalidation. The GraphQL operation was generated and validated against
the local Hasura schema. Read-only authorization checks run with:

```sh
devenv shell python3 scripts/test_voter_context_authorization.py
```

This requires a populated local ballot-style fixture and the updated metadata.
It uses administrative role impersonation to exercise Hasura's voter row
permissions, including nested election isolation, wrong tenant/event/area,
empty authorized elections, and an unrelated voter's empty cast metadata. It
does not test JWT signature verification. It logs no ballot content or voter IDs.

The prior five-request browser cohort remains release-10 evidence. The new
transport regression observes one `GetVoterContext` operation for initial data,
with no subsequent election request on review/confirmation cache consumption.
The browser verification below adds a main-portal cohort. No new SQL benchmark,
throughput or S3 measurement has been run. GraphQL operation counts are not SQL statement, physical-read,
transaction or database-checkout counts.

## TypeScript validation

From the existing devcontainer, enter `devenv shell`, then run:

```sh
cd /workspaces/step/packages
yarn typecheck:voting-portal
```

This generates declarations for `ui-core` and `ui-essentials` before checking the
portal with `tsc --noEmit`. A fresh workspace therefore does not need production
JavaScript bundles merely to resolve the UI package types. The portal explicitly
loads the Node and Jest ambient types it uses, avoiding unrelated transitive
`@types` packages such as the empty `minimatch` compatibility stub. Strict source
checking remains enabled.

The previously blocked check now passes. Fixes cover React 19's scoped element
types, nullable presentation data, Apollo 4's direct transport errors and typed
receipt mutation variables, validated cast-status strings, complete test fixtures,
and the invalid-login-link messages in all portal locales. Receipt generation
waits until its required tracker URL is available. Portal unit tests include the
transport-error distinction and rejection of unknown cast statuses.


## Browser verification after TypeScript fixes

The main portal was rebuilt and tested with Chromium 144.0.7559.132 against the
existing release-10/B3 local backend. This verifies the main portal's browser
journey and consolidated read operation; it does not verify a complete main/B4
backend deployment or private S3 delivery.

Ten fresh synthetic voters ran sequentially with fresh browser contexts, HTTP
cache disabled, warm services, and no think time. The fixture contained one event,
election, contest and area.

| Check | Result |
| --- | --- |
| Completed journeys with matching API, UI and persisted ballot IDs | 10/10 |
| Cast submissions | 10, with 10 distinct persisted cast IDs |
| Initial voter-context operations | One `GetVoterContext` per voter |
| Legacy initial reads | No `GetBallotStyles`, `GetElectionEvent`, `GetElections` or `GetCastVotes` |
| HTTP requests | 500 total; 50 per voter |
| Journey latency | p50 2,474 ms; p99 2,517 ms; range 2,306–2,518 ms |
| Capture failures or unexpected origins in the final cohort | 0 |

An earlier pilot cast was accepted and persisted, but its browser assertion failed
because the confirmation screen lacked the harness's ballot-ID selector. The
selector was added before rebuilding and running the final cohort with different
voters. The pilot is excluded from the ten successful journeys and was not retried.

These serial observations establish functional behavior and operation counts,
not capacity or a paired latency improvement. Raw browser captures, database
logs and synthetic credentials remain in the ignored local
`.cache/voter-types-e2e/` directory and must not be committed. The tested portal
bundle SHA-256 is
`aaf6efe33e93965cfc9bb37176a59b184e77d12b7d7f86e700ff0873efc13fcd`.

The accompanying validation passed all 92 portal unit tests, the TypeScript check
with both UI output directories initially absent, and production builds for both
UI packages and the portal.
