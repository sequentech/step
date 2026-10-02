# Trellis

Verifiable logs for PostgreSQL tables

## What is this?

Trellis provides **tamper-evident append-only logs** for PostgreSQL tables using the same cryptographic primitives as [Certificate Transparency](https://certificate.transparency.dev/howctworks/).

**How it works**: Configure source tables as "verifiable logs" - Trellis continuously copies entries to per-log tables (`merkle_log_{log_name}`), building Merkle trees in memory. Clients can request:
- **Inclusion proofs** - verify an entry exists in the log
- **Consistency proofs** - verify the log only appended (no deletions/modifications)

**Security properties** (see [RFC 6962](https://datatracker.ietf.org/doc/html/rfc6962)):
- ✅ Tamper detection - any modification invalidates cryptographic proofs
- ✅ Append-only guarantee - consistency proofs catch deletions or reordering
- ✅ Independent verifiability - clients verify proofs without trusting the operator
- ✅ Efficient auditing - verify without downloading entire log

**Out of scope**:
- ❌ Tamper prevention (database access control is your responsibility)
- ❌ Gossip protocol or split-view protection (no distributed monitors yet)

## Quickstart Tutorial

### Prerequisites

- Rust (tested on 1.90)
- PostgreSQL (tested on 18)
- Environment variables in `.env`:
  ```bash
  DATABASE_URL=postgres://user:password@localhost/dbname
  # The following are optional, with given default values
  TRELLIS_SERVER_ADDR=127.0.0.1:3000
  TRELLIS_BATCH_INTERVAL_SECS=1
  TRELLIS_BATCH_SIZE=30000
  TRELLIS_SERVER_URL=http://localhost:3000
  TRELLIS_DASHBOARD_REFRESH_INTERVAL=1
  ```

### Schema setup

```bash
# Initialize database schema
cargo run --bin setup --release
```

The following tables are created:

* `verification_logs`

    A collection of one or more verification_sources. Proofs of inclusion and consistency are computed over a verifiable log as a unit, combining its sources.
    
    **Log name restrictions**: Log names must match `[a-z0-9_]+` (lowercase letters, digits, underscores) and cannot start with a digit. This ensures safe use as PostgreSQL table name suffixes.

* `verification_sources`

    A source of entries for a verifiable log. These point to the pre-existing tables you wish to make verifiable.

* `merkle_log_{log_name}` (created dynamically)

    Per-log tables created automatically when a log is first processed. Each table stores the ground truth for that log - entries copied from sources in committed order, corresponding to merkle tree leaves.

### Running the server

```bash
# Run the server
cargo run --bin main --release
```
This binary runs
- **Batch Processor** - Continuously monitors source tables and updates per-log merkle tables and in-memory merkle trees.
- **HTTP API** - Serves proofs and tree state queries using in-memory data (the HTTP threads do not perform database access).

### Example: Inclusion Proof

```bash
# Simulate a post and compute an inclusion proof
cargo run --example post
```

This [example](examples/post.rs)
1. Simulates a post to a verification source
2. Waits for the new entry to be aggregated into the log
3. Requests a proof of inclusion for the entry
4. Verifies the returned proof
   
In order to work as a self-contained example, it also creates a sample `verification_source` and `verification_log`. To detect when the new entry is aggregated, `post` uses the `has_leaf` HTTP [endpoint](#api-reference).

### Example: Consistency Proof

```bash
# Run a log monitor
cargo run --example monitor -- example_post_log
```

This [example](examples/monitor.rs)
1. Polls a verification log for changes in the log root
2. When a change is detected, requests a proof of consistency with respect to the previous root.
3. Verifies the proof of consistency

This example runs continuously. To trigger a change in the log root, you can
run the `post` example from a new window and observe the output of the `monitor`:

```bash
💤 [Check #77] No changes (size: 3)
💤 [Check #78] No changes (size: 3)
📊 [Check #79] Size changed: 3 → 4
   ✅ Consistency proof VERIFIED
   → Log correctly appended 1 new entries
   Size: 4
   Root: 9NPDdyL+kAEQ6ej4...

💤 [Check #80] No changes (size: 4)
```
To detect when the the log root has changed, `monitor` uses the `get_log_size` and `get_root` HTTP [endpoints](#api-reference).

### Metrics and the dashboard

The service provides an [endpoint](#api-reference) that returns performance metrics. You can consume
this service with the `dashboard` binary, an n-curses style application that polls the service and
updates the display.

```bash
$ cargo run --bin dashboard
Connecting to http://localhost:3000/metrics...
Mode: AVERAGE | Press 'a' for average, 'l' for last

Cycle: 26ms (0.03x) | Active Logs: 3

LOG                      ROWS    TOTAL     COPY   INSERT   (µs/row)    FETCH     TREE  TREE/WORK         SIZE     MEMORY
------------------------------------------------------------------------------------------------------------------------
load_test_0              1032        9        8        4        3.9        0        1       0.05       458.5k     92.4MB
load_test_1              1032        7        6        4        3.9        0        1       0.05       458.5k     92.4MB
load_test_2              1032        7        5        4        3.9        0        1       0.05       458.5k     92.4MB
```

### Pausing/Resuming the Batch Processor

```bash
# Pause the Batch Processor
curl -X POST localhost:3000/admin/pause
{"status":"ok","message":"Batch processor paused","state":"paused"}
```

```bash
# Resume the Batch Processor
curl -X POST localhost:3000/admin/resume
{"status":"ok","message":"Batch processor resumed","state":"running"}
```

### Stopping the server

```bash
# Stop the server
curl -X POST localhost:3000/admin/stop
{"status":"ok","message":"Batch processor stopping (will shut down entire application)","state":"stopping"}
```
## How to..

### Configure a Log

To configure a log you
1. Identify one or more source tables and their required columns
2. Insert an entry into `verification_logs`
3. Insert one entry `verification_sources` for each source table from 1.

#### Step 1: Identify source tables and their required columns

Your source table(s) must have:
- An **id column** - [Unique](#source-table-id-column), total order identifier (typically `BIGSERIAL PRIMARY KEY`)
- A **hash column** - Pre-computed SHA256 hash (`BYTEA NOT NULL`)
It can optionally also have:
- A **timestamp_column** - timestamp for chronological ordering (`TIMESTAMPTZ`)

Examples of steps 2 and 3 follow, taken from [examples/post.rs](examples/post.rs).

#### Step 2: Creating a new verification log

```rust
// Create log if it doesn't exist
client
    .execute(
        "INSERT INTO verification_logs (log_name, description) 
          VALUES ($1, $2) 
          ON CONFLICT (log_name) DO NOTHING",
        &[&LOG_NAME, &"Example log for post demonstration"],
    )
    .await?;
```

#### Step 3: Attaching a verification source to a verification log

```rust
// Register source table with the log
client
    .execute(
        "INSERT INTO verification_sources (source_table, log_name, hash_column, id_column, timestamp_column)
          VALUES ($1, $2, $3, $4, $5)
          ON CONFLICT (source_table, log_name) DO NOTHING",
        &[&SOURCE_TABLE, &LOG_NAME, &"leaf_hash", &"id", &Some("created_at")],
    )
    .await?;
```

in this example, the columns for the source table would be

* id column: `id`
* hash column: `leaf_hash`
* timestamp column: `created_at`

### Verify Entry Inclusion

In the following example, a hash value is proved to be included in log `my_log`

```rust
use trellis::service::Client;

let client = Client::new("http://localhost:3000")?;

// We assume this has been entered into a source table and merged into the log
let hash = b"my hash";

// Get inclusion proof
let proof = client.get_inclusion_proof("my_log", hash).await?;

proof.verify(&hash)?;
```
A complete example can be seen in [examples/post.rs](examples/post.rs).

### Verify Log Consistency

In the following example, the root of the log `my_log` is proved to be consistent 
with a previous root.

```rust
// Get current root
let old_root = client.get_root("my_log").await?;

// Later, verify the log grew consistently
let proof = client.get_consistency_proof("my_log", &old_root).await?;
proof.verify(&old_root)?;
```
A full consistency verification workflow can be seen in [examples/monitor.rs](examples/monitor.rs).

### Validate log configurations

You can check for correct verification_log and verification_source configurations with

```bash
# Checks all log configurations
cargo run --bin main -- --verify-db
```

This will check that logs are configured correctly, pointing to existing sources with the
right column types.

```bash
$ cargo run --bin main -- --verify-db

✅ Log: 'test_log_no_timestamp' [ENABLED]
   Sources: 2 total, 2 valid, 0 invalid
   ✅ Source: 'source_no_timestamp'
   ✅ Source: 'source_no_timestamp_b'

✅ Log: 'test_log_single_source' [ENABLED]
   Sources: 1 total, 1 valid, 0 invalid
   ✅ Source: 'source_log'
```

### Disable and unload a log from memory

```bash
# Disable log 'example_post_log'
psql "postgres://user:user@localhost:5432/trellis_db" -c "update verification_logs set enabled = 'f' where log_name = 'example_post_log'";
```

To re-enable replace `set enabled = 'f'` with `set enabled = 't'` above.

## ⚠️Important notes

### The per-log `merkle_log_{log_name}` tables

**⚠️ CRITICAL**: The per-log `merkle_log_{log_name}` tables are ground truth and **cannot be reconstructed** deterministically from source tables. If these tables are lost, the merkle tree roots cannot be recomputed nor extended in a consistent way.

**Why?** Batch boundaries and late arrivals create path-dependent ordering:

```
Scenario 1 (Two batches):
  Batch 1: [A(t=1), B(t=2), C(no timestamp)] → C at position 3
  Batch 2: [D(t=3), E(t=4)]                  → positions 4-5
  
Scenario 2 (One batch):
  Batch 1: [A(t=1), B(t=2), D(t=3), E(t=4), C(no timestamp)]
  Result: C at position 5 (after all timestamped entries)
```

Same source data → different Merkle roots. This is **correct behavior** for append-only transparency logs where ordering is a point-in-time commitment.

**Implications**:
- Startup rebuild from `merkle_log_{log_name}` is deterministic
- Rebuild from source tables is NOT deterministic
- Per-log tables must be backed up for disaster recovery

### Source table id column

The **id column** _must_ be unique across all entries in a source table. It must have a unique
constraint on it - either a `PRIMARY KEY` or `UNIQUE` constraint. It is _not_ sufficient that 
this id is _part_ of a composite unique constraint.

**Why uniqueness matters**: The batch processor queries source tables by id ranges (e.g., `WHERE id > last_processed_id`). 
If ids are not unique, entries with duplicate ids could be skipped - if a row with id=5 is processed, 
any other rows with id=5 inserted later will never be picked up by subsequent batches.

The validation tool (`cargo run --bin main -- --verify-db`) will check for the required 
uniqueness constraint.

### Tiered ordering

Entries are ordered by: `(Option<Timestamp>, source_id, source_table)`

- **With timestamp_column**: Chronological order across all sources
- **Without timestamp_column**: ID-based order, sorts after timestamped entries
- Stable sort tie-breaking via `(source_id, source_table)` tuple

Verifiable logs are implemented following this ordering, but this is not dependable guarantee: the service only guarantees that _an_ order is followed and that the resulting trees will be consistent.
The use of a timestamp column helps in providing a more "natural" order when a verification
log has more than one source; in this case an ordering based only on id columns has no meaning.

### Batch processing atomicity

The batch processor ensures per-log all-or-nothing semantics:

**Transaction scope**: Each log's batch processes all its configured source tables within a single database transaction. This means:
- If any error occurs while processing a log, that log's batch rolls back
- No partial state is ever committed to `merkle_log_{log_name}` for that log
- Either all source tables for a log are processed together, or none are

**Independent log processing**: Logs are processed independently - an error in one log does not affect other logs being processed in the same cycle.

## API Reference

All endpoints return JSON with `"status": "ok"` or `"status": "error"`.

### Tree State

#### `GET /logs/{log_name}/root`
Current Merkle root.

**Response:**
```json
{
  "status": "ok",
  "log_name": "my_log",
  "root": "base64-encoded-root",
  "tree_size": 12345
}
```

**Errors**: `LogNotFound`, `EmptyTree`

#### `GET /logs/{log_name}/size`
Current tree size.

**Response:**
```json
{
  "status": "ok",
  "log_name": "my_log",
  "tree_size": 12345
}
```

### Proofs

#### `GET /logs/{log_name}/proof?hash=<base64>`
Inclusion proof for a leaf hash.

**Response:**
```json
{
  "status": "ok",
  "log_name": "my_log",
  "index": 42,
  "tree_size": 100,
  "root": "base64-root",
  "proof": ["base64-hash1", "base64-hash2"]
}
```

**Errors**: `LogNotFound`, `EmptyTree`, `ProofGenerationFailed`

#### `GET /logs/{log_name}/consistency?old_root=<base64>`
Consistency proof between old root and current root.

**Response:**
```json
{
  "status": "ok",
  "log_name": "my_log",
  "old_root": "base64-old-root",
  "new_root": "base64-new-root",
  "old_size": 50,
  "new_size": 100,
  "proof": ["base64-hash1", "base64-hash2"]
}
```

**Errors**: `LogNotFound`, `EmptyTree`, `ProofGenerationFailed`

### Queries

#### `GET /logs/{log_name}/has_leaf?hash=<base64>`
Check if a leaf exists.

**Response:**
```json
{
  "status": "ok",
  "log_name": "my_log",
  "exists": true
}
```

#### `GET /logs/{log_name}/has_root?root=<base64>`
Check if a root exists in history.

**Response:**
```json
{
  "status": "ok",
  "log_name": "my_log",
  "exists": true
}
```

### Admin

#### `POST /admin/pause`
Pause batch processing (HTTP server continues).

#### `POST /admin/resume`
Resume batch processing.

#### `POST /admin/stop`
Gracefully stop batch processor.

#### `GET /admin/status`
Check processor state: `Running`, `Paused`, or `Stopped`.

### Monitoring

#### `GET /metrics`
Detailed performance metrics (JSON).

**Response includes:**
- Per-log metrics: rows copied, timing breakdown (query/insert/fetch/tree)
- Global metrics: cycle duration, active logs, idle percentage

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

### Testing

#### Unit tests

```bash
# Run unit tests
cargo test
```
#### Integration tests

Running integration tests requires the server to be running

```bash
# Run the server
cargo run --bin main --release

# Run integration tests from a new process
cargo test --test basic -- --include-ignored --nocapture
```

Because integration tests require the server to be running, they are marked `#[ignore]`. They also use `serial_test` to prevent concurrent database access, which causes test failures.

### Linting

```bash
# Check all lints
cargo clippy --all-targets -- -D warnings
```
The lint configuration is relatively strict, it can be found in `Cargo.toml`.

### Performance

#### Batch processor optimization

The batch processor's primary bottleneck is PostgreSQL inserts and queries, 
not in-memory merkle tree operations (which usually complete in milliseconds). In a high insert rate scenario, the number of copied rows per batch can be large, which can benefit from:
* Multi-row INSERTs: The processor uses batch INSERTs instead of individual row inserts, 
  achieving a 10x performance improvement
    * TODO: consider postgresql COPY for even greater (though probably marginal) improvement

#### Per-log table architecture

Each log has its own dedicated table (`merkle_log_{log_name}`) rather than a shared table with a `log_name` column. This provides:

* **Smaller indexes**: Each table maintains independent indexes, improving INSERT performance as indexes stay smaller and more cache-friendly
* **Isolation**: Operations on one log don't affect others
* **Future parallelization**: Enables concurrent processing of different logs

#### `merkle_log_{log_name}` indexes

Each per-log table has these indexes:

* **Primary key on `id`**: Sequential ID for ordering within the log (each table has its own BIGSERIAL sequence). The PRIMARY KEY constraint automatically creates a B-tree index on `id`.

* **UNIQUE constraint on `(source_table, source_id)`**: Defense-in-depth against duplicate entries. Given that source IDs are unique per table, we copy with `WHERE source_id > last_processed`, and operations are transactional, duplicates are logically impossible. This constraint catches programming bugs and could be removed as a future optimization if INSERT performance becomes critical.

These indexes are created automatically when a log is first processed.

#### Source table contention

Contention for source tables can be minimized with
* Appropriate transaction isolation levels: use Read Committed (the default) in the batch processor
* Appropriate indices on source tables: the id column [should](#source-table-id-column) have a unique or primary key constraint on it, which automatically creates an index.

#### Merkle tree lock contention

Contention for merkle trees can be minimized with
* A concurrent hashmap: use Dashmap to store logs indexed by name
* Minimize locking scope: release the RwLock as soon as possible
    * TODO: could use read, clone, update and then write, to perform the tree updates outside of the lock, but this is not necessary at the moment.

#### HTTP endpoints

Whereas the batch processor is a serial process that runs over one second (by default)
intervals, HTTP endpoints can be accessed concurrently by large numbers of users. HTTP
endpoint performance benefits from:
* No database access: do not interact with the database from any HTTP endpoints
* O(1) historical root and leaf access: use separate hashmaps for this data
* No rewinding on merkle trees: do not offer any endpoint that requires rewinding trees, since that is a potentially expensive operation.

#### Load Testing

The `load` binary simulates heavy workloads for both the batch processor and HTTP API.

**Batch processor load**: Direct database inserts create entries that the batch processor must copy and incorporate into merkle trees.

```bash
# Insert 1000 entries per cycle, spread across 3 sources per log
cargo run --bin load --release -- --rows-per-interval 1000 --num-sources 3
```

**HTTP API load** (`--http-load`): Samples inserted entries and verifies them via HTTP endpoints:
- **Inclusion proofs**: Polls `has_leaf`, then requests and verifies inclusion proofs for sampled entries
- **Consistency proofs**: Maintains a ring buffer of historical roots and verifies consistency proofs against ~10% of stored roots each cycle

```bash
# Combined batch + HTTP load testing
cargo run --bin load --release -- --rows-per-interval 1000 --http-load --sample-rate 0.01
```

Key HTTP load parameters:
- `--http-load`: Enable HTTP endpoint testing
- `--sample-rate`: Fraction of inserted leaves to verify (default: 0.01 = 1%)
- `--max-roots`: Historical roots to retain per log for consistency proofs (default: 100)
- `--max-concurrent`: Concurrent HTTP request limit (default: 50)
- `--http-interval-ms`: Interval between HTTP test cycles (default: 500ms)

Together with the `dashboard` binary, this can be used to measure performance under load.

#### Typical performance numbers

##### Hardware 1 (drb)

`cargo run --bin load --release -- --num-sources 3 --rows-per-interval 1000`, ~450k entries

      Mode: AVERAGE | Press 'a' for average, 'l' for last

      Cycle: 26ms (0.03x) | Active Logs: 3

      LOG                      ROWS    TOTAL     COPY   INSERT   (µs/row)    FETCH     TREE  TREE/WORK         SIZE     MEMORY
      ------------------------------------------------------------------------------------------------------------------------
      load_test_0              1032        9        8        4        3.9        0        1       0.05       458.5k     92.4MB
      load_test_1              1032        7        6        4        3.9        0        1       0.05       458.5k     92.4MB
      load_test_2              1032        7        5        4        3.9        0        1       0.05       458.5k     92.4MB


`cargo run --bin load --release -- --num-sources 3 --rows-per-interval 5000`, ~1M entries

      Mode: AVERAGE | Press 'a' for average, 'l' for last

      Cycle: 102ms (0.09x) | Active Logs: 3

      LOG                      ROWS    TOTAL     COPY   INSERT   (µs/row)    FETCH     TREE  TREE/WORK         SIZE     MEMORY
      ------------------------------------------------------------------------------------------------------------------------
      load_test_0              5497       35       26       23        4.2        0        7       0.06         1.1M    219.5MB
      load_test_1              5497       33       24       21        3.8        0        7       0.06         1.1M    219.5MB
      load_test_2              5497       33       24       21        3.8        0        7       0.06         1.1M    219.5MB


`cargo run --bin load --release -- --num-sources 3 --rows-per-interval 10000`, ~1M entries

      Mode: AVERAGE | Press 'a' for average, 'l' for last

      Cycle: 220ms (0.18x) | Active Logs: 3

      LOG                      ROWS    TOTAL     COPY   INSERT   (µs/row)    FETCH     TREE  TREE/WORK         SIZE     MEMORY
      ------------------------------------------------------------------------------------------------------------------------
      load_test_0             11998       75       56       52        4.3        1       15       0.06         1.2M    251.7MB
      load_test_1             11998       72       54       50        4.2        1       15       0.06         1.2M    251.7MB
      load_test_2             11998       72       54       51        4.3        1       15       0.06         1.2M    251.7MB

`cargo run --bin load --release -- --num-sources 3 --rows-per-interval 10000`, ~5M entries

      Mode: AVERAGE | Press 'a' for average, 'l' for last

      Cycle: 245ms (0.20x) | Active Logs: 3

      LOG                      ROWS    TOTAL     COPY   INSERT   (µs/row)    FETCH     TREE  TREE/WORK         SIZE     MEMORY
      ------------------------------------------------------------------------------------------------------------------------
      load_test_0             11998       81       57       53        4.4        4       18       0.07         5.1M      1.0GB
      load_test_1             11998       82       59       56        4.7        2       18       0.07         5.1M      1.0GB
      load_test_2             11998       79       56       53        4.4        4       18       0.07         5.1M      1.0GB

`cargo run --bin load --release -- --num-sources 3 --rows-per-interval 20000`, ~10M entries

      Mode: AVERAGE | Press 'a' for average, 'l' for last

      Cycle: 811ms (0.77x) | Active Logs: 3

      LOG                      ROWS    TOTAL     COPY   INSERT   (µs/row)    FETCH     TREE  TREE/WORK         SIZE     MEMORY       UPDATE
      ------------------------------------------------------------------------------------------------------------------------------------------------
      load_test_0             35963      272      192      189        5.3       13       65       0.08        10.2M      2.0GB     00:17:08
      load_test_1             35329      261      182      178        5.0       14       63       0.08        10.2M      2.0GB     00:17:08
      load_test_2             35196      262      182      178        5.1       13       65       0.08        10.2M      2.0GB     00:17:08

##### Hardware 2

## Acknowledgments

Built with:
- [ct-merkle](https://crates.io/crates/ct-merkle) - RFC 6962 Merkle tree implementation
- [Axum](https://crates.io/crates/axum) - Web framework
- [deadpool-postgres](https://crates.io/crates/deadpool-postgres) - Connection pooling
- [tokio](https://crates.io/crates/tokio) - Async runtime
