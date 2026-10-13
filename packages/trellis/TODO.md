# Merkle Tree PostgreSQL Integration - TODO List

### Phase 4: Production Readiness

**Critical Issues**:

**Infrastructure Tasks**:
- [x] Error handling: What happens when source table doesn't exist or has wrong schema?
  - Implemented comprehensive validation module (validation.rs)
  - `--verify-db` flag for pre-deployment/operational checks with detailed reporting (cargo run --bin main -- --verify-db)
  - Startup validation warns about invalid sources without blocking service
  - Runtime resilience: skips invalid sources, continues processing valid ones
  - Validates table existence, column existence, and column types (id: bigint/integer, hash: bytea, timestamp: timestamp)
- [x] Check if leaf/root exists
  - Implemented `GET /logs/{log_name}/has_leaf?hash=<base64>` endpoint
  - Implemented `GET /logs/{log_name}/has_root?root=<base64>` endpoint
  - Both endpoints return `{"status": "ok", "log_name": "...", "exists": true/false}`
  - O(1) lookups via index map (has_leaf) and root history map (has_root)
  - Client methods: `has_leaf(log_name, data)` and `has_root(log_name, root)`
  - Use cases: Check before requesting proofs, polling for entry processing, validating historical roots
- [X] Empty tree handling. These are the responses for our endpoints on an empty tree
    1️⃣ Testing GET /root with empty tree...
      ✗ Error: Error getting root: Merkle tree for log 'test_log_single_source' is empty
    2️⃣ Testing GET /size with empty tree...
      ✓ Got size: 0
    3️⃣ Testing GET /proof with empty tree...
      ✗ Error: Error getting inclusion proof: Failed to generate inclusion proof: LeafNotFound
    4️⃣ Testing GET /consistency with empty tree...
      ✗ Error: Error getting consistency proof: Failed to generate consistency proof: ProofError: RootNotFound
- [X] Uniform error handling
  - Created responses.rs module with structured response types
  - Implemented `ApiResponse<T>` enum with `Success(T)` and `Error` variants using `#[serde(tag = "status")]`
  - Implemented `ApiError` enum with typed error cases: `LogNotFound`, `EmptyTree`, `InvalidRequest`, `ProofGenerationFailed`, `ProofVerificationFailed`
  - Created endpoint-specific response structs: `RootResponse`, `SizeResponse`, `InclusionProofResponse`, `ConsistencyProofResponse`, `HasLeafResponse`, `HasRootResponse`
  - All responses follow consistent JSON structure with `"status": "ok"` or `"status": "error"`
  - Replaced all ad-hoc `json!()` macros in route handlers
  - Proper HTTP status codes: 200 OK for success, 400 Bad Request for errors
  - Backward compatible with existing client code
- [x] Performance monitoring
    - [x] Implemented comprehensive metrics system (metrics.rs) with per-log and global statistics
    - [x] Created HTTP /metrics endpoint returning JSON with detailed timing breakdown
    - [x] Built TUI monitor utility (dashboard.rs) for real-time observation
    - [x] Created load generator (load.rs) with configurable parameters for testing
- [x] Graceful shutdown and pause control
    - Implemented processor_state: Arc<AtomicU8> in AppState (0=Running, 1=Paused, 2=Stopping)
    - Added admin endpoints (memory-only, no DB access):
      - POST /admin/pause - pause batch processing
      - POST /admin/resume - resume from paused state
      - POST /admin/stop - gracefully stop processor (HTTP server continues)
      - GET /admin/status - check current processor state
    - Batch processor checks state flag at start of each cycle
    - Maintains architectural separation: HTTP endpoints never touch database
- [X] Graceful shutdown: Stop processor cleanly, wait for in-flight batches (PARTIAL: stop 
      implementein-flight tracking not yet added) ==> Not necessary.
- [ ] Performance optimization (remaining items - not currently bottlenecks):
    - [X] REMOVED, not necessary: "Storing only published roots is only a marginal reduction in memory. No need to store entire tree,
          rewind never called by http endpoints", see [1]
    - [X] REMOVED: Superseded by multiple table implementation: Investigate PostgreSQL partitioning for merkle_log (partitions on log_name)
    - [ ] ct-merkle double-hashing issue (expects raw data, we're passing pre-computed hashes that get hashed again)
    - [ ] Configurable batch sizes and intervals per log
    - [ ] Minimize RwLock contention (VALIDATED: tree updates are 0-1ms, not needed currently)
          - Could use read, clone, update and then write, to perform the tree updates outsiude of the lock
    - [ ] Minimize PostgreSQL lock contention on source tables (documentation task for external applications)
    - [ ] Parallelize batch processor per-log
    - [ ]  Occasionally the merkle tree update time spikes to 2-4x its previous values, for approximately 10-20 cycles, before returning to the baseline. UPDATE: this has been confirmed to be caused by reallocation [2]
      - [ ] Preallocate CtMerkleTree hashmaps (there's two) with a given capacity
      - [ ] Use jemalloc as the allocator. This could also help in measuring memory usage which
      is currently only approximated for trees (with LogMetrics::tree_size_bytes)
      - [ ] If forking ct-merkle, we could also preallocate its two internal vectors; an additional positive side effect of this would be that we could potentially eliminate the "double hashing" problem item above.
    Note also, regarding initializing with capacities, staggered capacities is a valid technique to smooth out the worst-case latency, as reallocations would occur at different times.

**Documentation**:
- [X] REMOVED, sufficiently covered by Tutorial: Deployment guide (environment variables, database setup)
- [X] REMOVED, rustdoc + REAMDE.md is enough: API documentation (OpenAPI/Swagger spec)
- [X] REMOVED, examples + README.md is enough: Multi-log configuration examples
- [ ] Performance characteristics and tuning guide

**Testing**:
- [ ] Concurrent multi-log stress tests
- [ ] Error recovery scenarios (database disconnect, invalid source tables)
- [ ] Large dataset tests (millions of entries per log)

**Future Enhancements** (Post-Phase 4):
- [X] REMOVED: can be easily achieved with independent log tables: Pruning old entries (if needed)
- [ ] Read replicas for HTTP layer
- [ ] Async proof generation for large trees
- [ ] Proof caching for frequently requested proofs

## 📝 Design Notes

**Current Architecture**:
- Multiple independent logs, each with their own merkle tree and dedicated table
- verification_logs table defines available logs (log_name, enabled flag)
- verification_sources maps source tables to logs (composite PK: source_table, log_name)
  - id_column (required): Must be unique per table for ordering guarantees
  - timestamp_column (optional): Enables chronological ordering
- Per-log tables (`merkle_log_{log_name}`) store entries with independent BIGSERIAL sequences
- DashMap<String, Arc<RwLock<MerkleState>>> provides concurrent per-log state
- Startup rebuild ensures in-memory trees match database before serving requests
- Batch processor: sole owner of all database operations (clean separation)
- HTTP handlers: memory-only operations, read from DashMap (fast, no DB contention)
- Universal ordering: (Option<Timestamp>, id, table_name) provides chronological ordering with graceful degradation

**Key Invariants**:
1. Per-log ID monotonicity: Each log's `merkle_log_{log_name}.id` is sequential with no gaps
2. Tree only updated from committed per-log table data
3. **Per-log tables are ground truth and CANNOT be reconstructed deterministically from source tables**
   - Batch boundaries create different orderings depending on when batches run
   - Late-arriving entries with earlier timestamps cannot be retroactively inserted
   - Same source data produces different merkle roots depending on batching pattern
   - Therefore: per-log tables must be preserved and backed up for disaster recovery
4. Batch processor holds advisory lock per log during processing
5. All database writes happen in batch processor only
6. id_column must be unique per source table (typically primary key or unique constraint)

**Why per-log tables Cannot Be Rebuilt from Sources**:

Example demonstrating non-determinism:
- Scenario 1 (Two batches): 9 timestamped entries + X (no timestamp) → X at position 10
  Then 10 more timestamped entries in batch 2 → positions 11-20
  Result: X is at position 10

- Scenario 2 (One batch): Same 20 entries processed together
  All 19 timestamped entries come first → positions 1-19, X at position 20
  Result: X is at position 20

Same data, different merkle roots. This is correct behavior for append-only logs where ordering
is a point-in-time commitment based on what entries are known when the batch runs.

**Ordering Strategy**:
- Universal ordering key: (Option<DateTime<Utc>>, source_id, source_table)
- Timestamped sources: Chronological ordering by timestamp column (e.g., created_at)
- Non-timestamped sources: Sort after all timestamped entries, ordered by (id, table_name)
- Custom Ord implementation provides type-safe, compile-time verified ordering
- BTreeSet ensures O(log n) sorted insertion with automatic deduplication
- Conditional SQL queries: includes timestamp column only when configured
- Backward compatible: Optional timestamp column allows mixed timestamped/non-timestamped scenarios
- Deterministic and repeatable: Same database state always produces same tree

## Archived

## ✅ Completed

### Phase 0: Code Cleanup & Foundation
- [x] Renamed tables for clarity (processed_log → merkle_log, append_only_log → source_log)
- [x] Removed unused SQL objects (analyze_next_batch, processing_status, validate_processed_sequence)
- [x] Added source_table column to merkle_log for multi-source tracking
- [x] Changed source_log.id to BIGSERIAL for type consistency (all i64)

### Phase 1: Single Log, Multiple Sources
- [x] Implemented SourceConfig struct for configurable source tables
- [x] Replaced PL/pgSQL process_next_batch() with Rust implementation
- [x] Implemented batch processor that queries multiple source tables
- [x] Merges rows by order column and inserts with sequential merkle_log IDs
- [x] Implemented connection pooling (deadpool-postgres)
- [x] Fixed all type conversions (consistent i64 throughout)
- [x] **Created verification_sources configuration table**
- [x] **Dynamic source loading on every batch iteration**
- [x] **All-or-nothing validation of configured tables**
- [x] **Automated multi-source testing infrastructure**
- [x] All tests passing

**Phase 1 Complete!** ✨

### Phase 2: Multiple Independent Logs
**Goal**: Support multiple independent merkle trees, each tracking different sets of source tables

**Implementation Complete**:
- [x] Created `verification_logs` table (log_name PRIMARY KEY, enabled BOOLEAN)
- [x] Extended `verification_sources` with log_name (composite PK: source_table, log_name)
- [x] Added log_name FK to merkle_log with CASCADE delete
- [x] Implemented DashMap<String, Arc<RwLock<MerkleState>>> for per-log state
- [x] Path-based HTTP endpoints: /logs/{log_name}/root, /size, /proof, /consistency
- [x] Batch processor loops over all enabled logs, processes each independently
- [x] Per-log advisory locks using hash_string_to_i64(log_name)
- [x] Composite key sorting: (order_value, source_table, source_id) for deterministic ordering
- [x] Startup rebuild: all enabled logs rebuilt from database on server start
- [x] Clean architectural separation: batch processor owns all DB access, HTTP handlers read from DashMap only
- [x] Removed manual /rebuild endpoint (no longer needed with startup rebuild + continuous processing)
- [x] Idempotent test setup with two separate test logs (no destructive operations)
- [x] All tests passing, repeatable from any database state

**Phase 2 Complete!** ✨

**Architecture Notes**:
- Each log has its own table: `merkle_log_{log_name}` with independent BIGSERIAL sequence
- IDs are monotonically increasing and sequential within each log (no gaps)
- Log names must match `[a-z0-9_]+` pattern (validated at runtime)
- Each log maintains independent merkle tree state in memory
- Processor and HTTP server share only DashMap (designed for concurrent access)
- No database contention from HTTP requests (memory-only operations)

### Phase 3: Universal Ordering System
**Goal**: Implement robust chronological ordering across multiple sources with different schemas

**Implementation Complete**:
- [x] Schema updates: Renamed order_column → id_column (must be unique per table)
- [x] Added optional timestamp_column to verification_sources
- [x] Universal ordering key: (Option<DateTime<Utc>>, i64, String)
- [x] Custom Ord implementation for SourceRow with explicit ordering logic:
  1. Timestamped entries sort before non-timestamped (Some < None)
  2. Within timestamped: chronological order by timestamp
  3. Secondary sort by source_id (unique per table)
  4. Tertiary sort by source_table (deterministic tie-breaker)
- [x] BTreeSet for O(log n) automatic sorted insertion
- [x] Conditional SQL queries based on timestamp_column presence
- [x] Test infrastructure for mixed timestamped/non-timestamped sources
- [x] Comprehensive ordering tests: test_universal_ordering and test_no_timestamp_ordering
- [x] Backward compatible: sources without timestamps gracefully degrade to ID-based ordering
- [x] All tests passing with verification of chronological and ID-based ordering

**Phase 3 Complete!** ✨

### References

[1] Hypothesis CONFIRMED. ✅ The rewind function cannot be called by any HTTP endpoints.

Detailed trace:

get_inclusion_proof (lines 100-159)
Calls tree.prove_inclusion(&query.hash)
Which delegates to prove_inclusion_at_root(hash, &self.root()) ← passes CURRENT root
In prove_inclusion_at_root (line 163-165): checks if root == self.root() → takes early return, skips rewind
Then calls tree.verify_inclusion(&query.hash, &proof)
Which delegates to verify_inclusion_at_root(leaf_hash, &self.root(), proof) ← passes CURRENT root
In verify_inclusion_at_root (line 185-187): checks if root == self.root() → takes early return, skips rewind
get_consistency_proof (lines 161-228)
Calls tree.prove_consistency(&query.old_root)
Which delegates to prove_consistency_between(old_root, &self.root()) ← passes CURRENT root as new_root
In prove_consistency_between (line 226-228): checks if new_root == self.root() → takes first branch &self.tree, skips rewind
Then calls tree.verify_consistency(&query.old_root, &proof)
Which delegates to verify_consistency_between(old_root, &self.root(), proof) ← passes CURRENT root as new_root
In verify_consistency_between (line 257-259): checks if new_root == self.root() → takes first branch &self.tree, skips rewind
Key insight: The HTTP handlers only use the public API methods (prove_inclusion, verify_inclusion, prove_consistency, verify_consistency) which always use the current root for the "new" or "target" tree. The rewind function is only called when checking against historical roots, but the HTTP endpoints never pass historical roots as the verification target.

[2]

2026-01-04T12:59:04.032391Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_1" tree_size=459954 leaves_added=19998 tree_ms="116.93" normalized="0.31"
2026-01-04T12:59:04.249591Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_2" tree_size=459954 leaves_added=19998 tree_ms="118.91" normalized="0.32"
2026-01-04T12:59:05.476890Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_0" tree_size=469953 leaves_added=19998 tree_ms="117.48" normalized="0.31"
2026-01-04T12:59:50.831264Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_0" tree_size=919908 leaves_added=9999 tree_ms="216.70" normalized="1.09"
2026-01-04T12:59:51.089273Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_1" tree_size=919908 leaves_added=9999 tree_ms="214.89" normalized="1.08"
2026-01-04T12:59:51.352406Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_2" tree_size=919908 leaves_added=9999 tree_ms="219.23" normalized="1.11"
2026-01-04T13:01:23.877836Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_0" tree_size=1839816 leaves_added=9999 tree_ms="431.69" normalized="2.07"
2026-01-04T13:01:24.373380Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_1" tree_size=1839816 leaves_added=9999 tree_ms="437.35" normalized="2.10"
2026-01-04T13:01:24.908695Z  WARN trellis::service::processor: Tree update spike detected (threshold: 0.2) log_name="load_test_2" tree_size=1849815 leaves_added=19998 tree_ms="440.38" normalized="1.06"

Doubling pattern: 460K → 920K → 1.84M — each spike is at roughly 2× the previous tree size

Increasing cost: The reallocation time scales with tree size (116ms → 217ms → 435ms), which matches O(n) copy behavior

Consistent across logs: All three logs spike at the same tree sizes, confirming it's the data structure, not random variance

Not exact powers of 2: The sizes (460K, 920K, 1.84M) suggest the underlying Vec started with non-power-of-2 capacity or there's some offset from the HashMap thresholds overlapping

This code was used to produce the traces:

// Update metrics for this log <---- existing code marker  
let final_tree_size = merkle_state.tree.len();

// Trace code
// Detect tree update spikes (likely caused by reallocation)
// Formula: tree_ms / (rows * log2(tree_size)) = µs per (row × tree level)
if leaves_added > 0 && final_tree_size > 1 {
    let tree_ms = tree_duration.as_secs_f64() * 1000.0;
    let log2_size = (final_tree_size as f64).log2();
    let normalized = (tree_ms * 1000.0) / (leaves_added as f64 * log2_size);
    const SPIKE_THRESHOLD: f64 = 0.2; // µs per (row × level), adjust as needed
    if normalized > SPIKE_THRESHOLD {
        tracing::warn!(
            log_name,
            tree_size = final_tree_size,
            leaves_added,
            tree_ms = format!("{:.2}", tree_ms),
            normalized = format!("{:.2}", normalized),
            "Tree update spike detected (threshold: {SPIKE_THRESHOLD})"
        );
    }
}