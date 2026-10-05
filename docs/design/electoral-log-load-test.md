<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# PostgreSQL electoral log load test

Parent issue: https://github.com/sequentech/meta/issues/13698

This test measures whether appends, queries, proofs and audits of the PostgreSQL electoral log scale to 50 million records on one board. It also checks whether the indexes are used and sufficient. One board was filled to 20 million records and measured at 1, 5, 10 and 20 million. The 50 million figures are extrapolated from those measurements, the measured storage per record and a memory-pressure run. A 50 million record board needs about 100 GB, which the test machine did not have.

## Verdict

- **Proofs, checkpoints and index-backed reads scale.** This covers the default admin page, sorting by ID, creation time or user ID, filtering by user ID, record lookups, the ballot locator's page and own-ballot lookup, and the cursor-based CSV and cast-vote exports. They stayed between 0.2 and 30 ms from 1 to 20 million records, with record proofs at 17 ms when cold. Their cost depends on the depth of the B-trees and on cache misses, not on the number of records.
- **Appends work at 50 million, but per-board throughput falls as the indexes outgrow memory.** On a 12 GB database server, batch appends fell from 8,900 records/s at 1 million to 1,500 records/s at 20 million. With the bulk insert added here and memory scaled down to match 50 million records on 12 GB, they ran at 1,000 to 1,800 records/s. Single-record appends ran at about 400/s locally and at about 90/s with 0.5 ms of network latency. Appends to one board are serialized, so more workers do not help.
- **Several admin portal features do not scale.** These are sorting by statement timestamp or kind, filtering by username, statement timestamp or ballot ID, filtering by creation time while sorted by ID, counts over most of the board, and deep pages. At 20 million records they took 20 seconds to 8 minutes per request, and they grow linearly. They are listed under [Remaining limits](#remaining-limits) with the measured fixes.
- **The indexes are used as intended and are enough for the paths above, but not for the admin portal's username and statement-timestamp filters and sorts.** Indexes for those two fix them, measured below, at about 5 GB each at 50 million records and about a third slower batch appends.
- **Three problems were fixed.** The audit was quadratic in the board size, the admin page counted the whole board on every load, and batch appends made one round trip per record.

## Setup

- **Machine and database:** an 8 vCPU, 31 GiB development VM with a GCP persistent disk. PostgreSQL 18.6 ran in its own container limited to 4 CPUs and 12 GB of memory, with `shared_buffers=3GB`, `effective_cache_size=8GB`, `work_mem=32MB`, `maintenance_work_mem=1GB`, `max_wal_size=4GB`, `random_page_cost=1.1`, `effective_io_concurrency=200`, default autovacuum and `synchronous_commit=on`. The database collation is `en_US.utf8`, as in development. The database was about three times larger than its memory at 20 million records, as a 100 GB database would be on a 32 GB server.
- **Client:** the client was `packages/electoral-log/examples/load_test.rs`, a release build that appends through `PostgresStore`, the path Windmill uses. It ran on the same host (round trip about 0.1 ms) unless stated.
- **Data:** one board shaped like an election. Each voter produced four Keycloak events and one cast vote, with 20,000 voters interleaved so a voter's records are far apart. Message sizes came from signed development records: 702 bytes for cast votes and 330 to 450 bytes for Keycloak events. The data used random UUID user and delivery IDs, 5 elections, 1,000 areas and 1,000 records per second of timestamps. Filling used appends of 5,000 records, and appends were measured in the forms Windmill uses: single records, and batches from the queue dispatcher.
- **Measurements:**
  - **Steps:** at each size the test timed appends with a warm cache, ran every query (`probe`), captured `EXPLAIN (ANALYZE, BUFFERS)` plans, repeated the appends, ran an audit, and ran an audit with appends running concurrently.
  - **Repetitions:** queries ran up to five times, and a query that took more than 30 seconds ran once.
  - **Cold timings:** the first run is "cold" because earlier scans had evicted the cache, as concurrent admin use does in production.

## Storage

Measured at 10 million records: 1,968 bytes per record, about 98 GB at 50 million before WAL and free space.

| Relation | Bytes/record |
| --- | ---: |
| `electoral_log_messages` rows | 855 |
| `electoral_log_voter` (board, user, ballot, ID) | 214 |
| unique (board, delivery ID) | 188 |
| `electoral_log_cast_vote` (board, kind, election, ID) | 164 |
| `electoral_log_created` (board, created, ID) | 105 |
| `electoral_log_board_cursor` (board, ID) | 97 |
| `trellis_nodes` rows / primary key | 93 / 73 |
| `trellis_leaves` rows / primary key / (log, source) | 93 / 32 / 32 |
| `electoral_log_messages` primary key | 22 |

Every message index starts with the 64-character board name. That is about 320 bytes per record, about 16 GB at 50 million.

## Appends

Batch appends, in records per second. Appends to a board are serialized, so 4 concurrent writers changed the warm rates by −7 to +39 %; they only overlap client work with database work. At 1 million records the database fit in memory, so the run after the admin scans was warm too.

| Records | Filling (5,000) | 1,000, warm | 1,000, after admin scans |
| ---: | ---: | ---: | ---: |
| 1 M | 7,000–7,600 | 8,864 | — |
| 5 M | 6,000 | — | 1,259 |
| 10 M | 5,000 | 3,159 | 714 |
| 20 M | 2,200–3,300 | 1,527 | 204–800 |

- **Why throughput falls:** `pg_stat_statements` shows that the record insert dominates. Over the whole run it took 2,987 s, of which 1,787 s was spent reading pages, against 302 s for the leaf and subtree inserts. The delivery-ID key and the voter index are the only indexes inserted at random positions; once they exceed memory, most inserted rows read one of their pages from disk. Admin queries that scan the board evict those pages, which is why appends ran 2 to 7 times slower right after the query probe.
- **Bulk insert (added here):** batch appends now insert each chunk of up to 5,000 records with one `INSERT … SELECT … FROM UNNEST(…) ORDER BY position` instead of one statement per record. The semantics are unchanged: records get IDs in order, the first copy of a repeated delivery ID is kept, and each chunk is journaled in ID order. At 20 million records, A/B against the previous binary. Each row alternated the two binaries back to back, and the cache warmed between rows, so compare within a row. The table above was measured row by row, before this change.

  | Batch | Row by row | Bulk |
  | --- | ---: | ---: |
  | 1,000, local | 714–812/s | 957–1,283/s |
  | 5,000, local | 1,470/s | 1,859/s |
  | 1,000, +0.5 ms round trip | 840–868/s | 1,814–1,844/s |

  Row by row, a database 0.5 ms away caps a board at about 2,000 records/s regardless of its size.
- **Memory pressure equivalent to 50 million records:** with the database container limited to 5.5 GB at 20 million records, the ratio of memory to random-key indexes roughly matches 50 million records on 12 GB. Bulk appends then ran at 988 to 1,067 records/s in batches of 1,000 and 1,759 records/s in batches of 5,000.
- **Single-record appends:** these are used for administrative events, checkpoints and password changes. They took 1.8 ms at 1 million records and 2.5 ms at 20 million, about 400/s per board, and concurrency only queues them (8 writers: 376/s, 20 ms median). Each makes about 20 round trips, so with 0.5 ms of added round-trip latency they took 11 ms, about 90/s per board. Votes and Keycloak events do not use this path; they go through the batching queue.
- **Lock hold time:** a batch holds the board's lock for its whole transaction, 0.6 s per 1,000 records at 20 million and up to 9.8 s per 5,000. The development and remote deployment configurations let the dispatcher batch up to 100,000 events (`DEFAULT_SQL_BATCH_SIZE`), which could hold a board for a minute or more on a large board.

To sustain 50 million records over a 12-hour election, a board must absorb an average of about 1,200 records/s, plus peaks. The queue absorbs the peaks, and the measurements above show this is marginal on a 12 GB server. A server whose memory holds the message indexes (about 40 GB at 50 million records) should keep appends near the 10-million-record rates.

## Queries

Median of the warm runs, with the cold first run in parentheses where it was much slower. Queries above 30 seconds ran once, cold.

| Operation (caller) | 1 M | 5 M | 10 M | 20 M |
| --- | --- | --- | --- | --- |
| Newest page, ID order (admin default) | 0.3 ms | 0.4 ms | 0.3 ms | 0.4 ms |
| Count of the whole board (every admin page load)¹ | 119 ms | 442 ms (2.6 s) | 781 ms (10.1 s) | 1.3 s (18.2 s) |
| Sort by created or user ID | <1 ms | <1 ms | <1 ms | <1 ms |
| Sort by statement timestamp | 354 ms | 2.3 s (10.4 s) | 30 s | 36 s |
| Sort by statement kind | 216 ms | 47 s (80 s) | 84 s | 33 s |
| Page at offset size/2 | 130 ms | 619 ms (43 s) | 54 s | 476 s |
| Filter by user ID, page and count | ≤0.3 ms | ≤0.3 ms | ≤0.3 ms | ≤0.4 ms |
| Filter by username, page + count | 0.3 s | 2.1 s | 4.1 s | 41 s |
| Filter by kind, page / count | 0.5 / 61 ms | 0.5 / 189 ms (10 s) | 6 / 350 ms (22 s) | 9 ms / 47 s |
| Filter by created minute, page / count | 126 / 5 ms | 1.2 s (45 s) / 5 ms | 114 s / 6 ms | 343 s / 5 ms |
| Filter by statement-timestamp minute, page / count | — | — | 2.7 / 1.5 s | 391 / 20 s |
| Filter by ballot ID, page / count | 68 / 68 ms | 0.3 (44) / 0.3 s | 89 / 0.5 s | 178 / 2.8 s |
| Election-scoped admin, page / count | 0.4 / 202 ms | 0.5 ms / 1.1 s | 1.7 ms / 2.1 s | 2 ms / 21 s |
| Count of records with a user | 206 ms | 0.7 (18) s | 111 s | 182 s |
| Ballot locator: count of the election's votes | 7 ms | 32 ms (2.9 s) | 48 ms (6 s) | 124 ms (11 s) |
| Ballot locator: page, own ballot | ≤0.3 ms | ≤0.3 ms | ≤0.3 ms | ≤0.3 ms |
| CSV export batch after an ID (2,500) | 7 ms | 6 ms | 5 ms | 6 ms |
| PDF report batch at offset size/2 (2,500) | 203 ms | 931 ms | 3.0 s | 32 s |
| Cast-vote export batch after an ID (1,000) | 29 ms | 29 ms | 24 ms | 27 ms |
| Current checkpoint | 0.7 ms | 0.5 ms | 0.5 ms | 0.9 ms |
| Record proof, random record | 1.2 ms | 11 ms | 15 ms | 17 ms (p95 23 ms) |
| Consistency proof | 0.7 ms | 0.9 ms | 0.8 ms | 0.7 ms |
| Record proof from a trusted checkpoint, cached | 1.5 ms | 1.8 ms | 1.5 ms | 1.8 ms |

¹ Before the fix below. An unfiltered count now reads the journal's size and is constant time.

The plans confirm the slow rows:

- **Sorting by statement timestamp or kind:** there is no usable index, so these read the whole board. At 5 million records the kind sort walked `electoral_log_cast_vote` in kind order and fetched every row to sort by ID.
- **Username, ballot ID and statement-timestamp filters:** these columns are not indexed after the board name, so the filters scan the board.
- **Creation-time filter sorted by ID (planner trap):** the planner walks the primary key backwards and filters by time, expecting matches to be spread out. Matches from the middle of the log are reached only after half the board (2.44 million rows skipped at 5 million). The `electoral_log_created` index is not used for this query.
- **Deep offsets:** `OFFSET n` fetches and discards `n` rows from the heap.
- **Broad counts:** counts over the board, a kind, a scope or records with a user read every matching index entry or row.

`LIKE` filters with no wildcard use the B-tree indexes despite the `en_US.utf8` collation. A value containing `_` or `%` turns them into patterns that cannot use these indexes.

## Audits

- **Duration:** a full audit took 85 s at 5 million records, 166 s at 10 million and 376 s at 20 million. That is linear, about 16 to 20 minutes at 50 million on this server. The 1-million-record audit took 67 s before the fix below.
- **Effect on appends:** an audit reads one snapshot for its whole run. While it runs, superseded versions of the board's `trellis_logs` row cannot be removed. With single-record appends running throughout, append latency rose from 4.4 to 5.4 ms (+23 %) over the 6.5-minute audit at 20 million records, and 15,500 row versions (2.2 MB) accumulated. Autovacuum removed them afterwards. The growth is proportional to audit duration times append rate, so audits are best run when the board is quiet, as the post-tally audit is.

## Fixed in this change

1. **Quadratic audit:** each audit page joined records after a cursor to the leaves, and PostgreSQL does not carry `m.id > $3` over to `trellis_leaves`. Every page therefore re-read the leaf index from the start of the log: a page in the middle of the 20-million-record board read 10 million leaf entries and took 4.8 s, so a full audit would have taken about a day there and about a week at 50 million. The page now bounds both sides and takes 0.7 ms. A PostgreSQL test reads the plan and fails if one page reads, or filters out, more than two pages' worth of leaves; it read 20,000 leaves for a 1,000-record page before the fix. CI now runs it with the other PostgreSQL tests.
2. **Full count on every admin page load:** every list request also ran `COUNT(*)` over the board. A count with no filter now reads the board's committed size from `trellis_logs`, because each record commits with exactly one leaf. A test shows that a row written around the journal is still listed and fails the audit, but is not counted.
3. **One round trip per appended record:** batch appends now use the bulk insert described above. A test covers repeated deliveries within one append.

The load-test harness is committed as an example, so these measurements can be repeated.

## Remaining limits

These need product or sizing decisions. Each fix was measured at 20 million records where noted.

- **Admin sorting:** sorting by statement timestamp or kind reads the whole board.
  - An index on (board, statement timestamp, ID) reduced the sort from 36 s to 0.14 ms. It costs 106 bytes/record, built in 1 min 42 s.
  - Kind sorting would need (board, kind, ID), not measured.
  - Alternatively, the portal could offer only indexed sorts.
- **Username filter:** an index on (board, username, ID) reduced a page and its count from 41 s to 4 ms. It costs 115 bytes/record, built in 2 min 24 s. With both candidate indexes, batch appends at 20 million records were 31 to 37 % slower (1,213 vs 1,766 records/s for batches of 5,000) and single appends 13 % slower. The test's usernames grow with the voter number; random usernames would cost more.
- **Timestamp filters with the default ID sort:**
  - Sorting by the filtered timestamp uses its index directly. The created-minute page took 6.5 ms cold instead of 343 s.
  - Keeping the ID sort, adding the minute's ID range, computed in 34 ms through the created index, made it 1.5 ms.
  - The statement-timestamp filter also needs the index above.
- **Counts on filtered views:**
  - Counts of a kind, an election scope or records with a user grow linearly: 21 to 182 s at 20 million records.
  - The ballot locator counts the election's cast votes on every request: 124 ms warm, 11 s cold at 20 million records, about 2.5 times that at 50 million.
  - Possible fixes are capped counts ("10,000+"), estimates or cached totals.
- **Deep pages:** a page at offset 10 million took 8 minutes. The admin list needs an offset limit or cursor pagination. The PDF activity report fetches its batches by offset, so its total cost is quadratic in the board size; the CSV export already uses a cursor.
- **Ballot ID filter:** used by the API only. It took 178 s at 20 million records; the ballot locator also filters by user and is fast.
- **Sizing:** plan about 2 KB per record (100 GB at 50 million) and memory for the message indexes (about 40 GB at 50 million) to keep appends fast.
  - Replacing the board name with the log's numeric ID in `electoral_log_messages` would save about 300 bytes per record (15 GB at 50 million) and shrink every index.
  - Making `electoral_log_cast_vote` partial to cast votes would save about 130 bytes per record, but the admin kind filter uses it for other kinds too.
- **Dispatcher batch size:** a batch holds the board lock for its whole transaction, so batches of 100,000 events make other appends to the board wait; a smaller `DEFAULT_SQL_BATCH_SIZE` bounds that wait.

## Reproduce

Use a disposable database initialized with `packages/electoral-log/schema.sql`. Set the `ELECTORAL_LOG_PG_*` variables to point at it, then run from `packages/`:

```bash
cargo run --release -p electoral-log --example load_test -- fill --target 20000000
cargo run --release -p electoral-log --example load_test -- probe
cargo run --release -p electoral-log --example load_test -- appends --count 10 --batch 1000
cargo run --release -p electoral-log --example load_test -- audit
cargo run --release -p electoral-log --example load_test -- params   # psql variables for EXPLAIN scripts
```

`fill` resumes from the board's current size and saves a checkpoint every 250,000 records; `probe` and `audit` use these checkpoints.
