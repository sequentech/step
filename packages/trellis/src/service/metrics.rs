use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::tree::LEAF_HASH_SIZE;

/// Per-log batch processing metrics
#[derive(Debug, Clone)]
pub struct LogMetrics {
    /// Name of the log these metrics belong to
    pub log_name: String,
    /// Number of rows processed in the last batch
    pub last_batch_rows: u64,
    /// Number of leaves added to tree in the last batch
    pub last_batch_leaves: u64,
    /// Total time for entire processing cycle (ms)
    pub last_total_ms: u64,
    /// Time for `copy_source_rows` operation (ms)
    pub last_copy_ms: u64,
    /// Time for INSERT operation (ms) - includes CTE query, sort, and insert
    pub last_insert_ms: u64,
    /// Time to fetch new entries from `merkle_log` (ms)
    pub last_fetch_merkle_log_ms: u64,
    /// Time to update in-memory tree (includes write lock hold time) (ms)
    pub last_tree_update_ms: u64,
    /// Total number of batches processed since startup
    pub batches_processed: u64,
    /// Current tree size (number of leaves)
    pub tree_size: u64,
    /// Estimated tree memory usage in bytes (placeholder)
    pub tree_memory_bytes: u64,
    /// Timestamp of last update
    pub last_updated: DateTime<Utc>,
}

impl LogMetrics {
    /// Creates a new `LogMetrics` instance for the given log
    #[must_use]
    pub fn new(log_name: String) -> Self {
        Self {
            log_name,
            last_batch_rows: 0,
            last_batch_leaves: 0,
            last_total_ms: 0,
            last_copy_ms: 0,
            last_insert_ms: 0,
            last_fetch_merkle_log_ms: 0,
            last_tree_update_ms: 0,
            batches_processed: 0,
            tree_size: 0,
            tree_memory_bytes: 0,
            last_updated: Utc::now(),
        }
    }

    /// Update metrics after processing a batch
    /// `arithmetic_side_effects`: It would take 10^11 years to overflow u64 here at 1 batch/s
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::arithmetic_side_effects)]
    pub fn record_batch(
        &mut self,
        rows_copied: u64,
        leaves_added: u64,
        total_ms: u64,
        copy_ms: u64,
        insert_ms: u64,
        fetch_merkle_log_ms: u64,
        tree_update_ms: u64,
        tree_size: u64,
    ) {
        self.last_batch_rows = rows_copied;
        self.last_batch_leaves = leaves_added;
        self.last_total_ms = total_ms;
        self.last_copy_ms = copy_ms;
        self.last_insert_ms = insert_ms;
        self.last_fetch_merkle_log_ms = fetch_merkle_log_ms;
        self.last_tree_update_ms = tree_update_ms;
        self.batches_processed += 1;
        self.tree_size = tree_size;
        self.tree_memory_bytes = Self::tree_size_bytes(tree_size);
        self.last_updated = Utc::now();
    }

    /// Returns an approximate size of a tree with n leaves in bytes
    ///
    /// A `CtMerkleTree` has three fields
    /// - `MemoryBackedTree`
    ///     - stores a vector of `LeafHash` entries with size equal to `tree.len()`
    ///     - stores a vector of internal hashes, which "contains all the hashes of the leaves and parents"
    ///
    /// A tree with n leaves has (2n - 1) nodes, each with a hash of `LEAF_HASH_SIZE` bytes.
    /// Therefore, the internal hashes use (2n - 1) * `LEAF_HASH_SIZE` bytes.
    /// Thus the total size of the `MemoryBackedTree` is approximately:
    ///     
    /// n + (2n - 1) * `LEAF_HASH_SIZE`
    ///
    /// - `leaf_hash_to_index`: `HashMap` mapping leaf hashes to indices
    ///     - stores n entries, each with a key of `LEAF_HASH_SIZE` bytes and a value of usize (8 bytes on 64-bit systems)     
    ///  
    /// Thus, this map uses approximately
    ///
    /// n * (`LEAF_HASH_SIZE` + 8) bytes.
    ///
    /// - `root_hash_to_size`: `HashMap` mapping root hashes to tree sizes
    ///    - in the current implementation, we are storing a root every time a new leaf is added (a possible
    ///      task to modify this is in the TODO), so this map stores one entry per leaf.
    ///      Each entry has a key of `LEAF_HASH_SIZE` bytes and a value of usize (8 bytes on 64-bit systems).
    ///     
    /// Thus, this map uses approximately
    ///
    /// n * (`LEAF_HASH_SIZE` + 8) bytes.
    ///
    /// Combining these, the total size in bytes is approximately:
    ///     n + (2n - 1) * `LEAF_HASH_SIZE` + n * (`LEAF_HASH_SIZE` + 8) + n * (`LEAF_HASH_SIZE` + 8)
    /// =   (3n - 1) * `LEAF_HASH_SIZE` + 2n * (`LEAF_HASH_SIZE` + 8)
    ///
    /// Due to overhead from `HashMap` and Vector structures, actual memory usage may be higher, so we apply
    /// a multiplier of 1.2 to try to account for that.
    ///
    /// `arithmetic_side_effects`: tree sizes will never approach `u64::MAX` in practice, resource consumption will limit much earlier.
    #[allow(clippy::arithmetic_side_effects)]
    #[must_use]
    pub const fn tree_size_bytes(n: u64) -> u64 {
        // we do not use 3n - 1 and just use 3n to avoid underflow when n = 0
        let tree = (3 * n) * LEAF_HASH_SIZE;
        let maps = (2 * n) * (LEAF_HASH_SIZE + 8);

        ((tree + maps) * 12) / 10
    }
}

/// Global processing metrics
#[derive(Debug, Clone)]
pub struct GlobalMetrics {
    /// Duration of last complete processing cycle (ms)
    pub last_cycle_duration_ms: u64,
    /// Number of active logs being processed
    pub last_active_log_count: usize,
    /// Ratio of cycle duration to processing interval
    pub last_cycle_fraction: f64,
    /// Timestamp when last cycle completed
    pub last_cycle_timestamp: DateTime<Utc>,
}

impl GlobalMetrics {
    /// Creates a new `GlobalMetrics` instance with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_cycle_duration_ms: 0,
            last_active_log_count: 0,
            last_cycle_fraction: 0.0,
            last_cycle_timestamp: Utc::now(),
        }
    }

    /// Update metrics after a processing cycle
    #[allow(clippy::cast_precision_loss)]
    pub fn record_cycle(
        &mut self,
        cycle_duration_ms: u64,
        active_logs: usize,
        interval: &core::time::Duration,
    ) {
        self.last_cycle_duration_ms = cycle_duration_ms;
        self.last_active_log_count = active_logs;
        // Calculate fraction assuming a 1-second interval (configurable in processor)
        self.last_cycle_fraction = cycle_duration_ms as f64 / interval.as_millis() as f64;
        self.last_cycle_timestamp = Utc::now();
    }
}

impl Default for GlobalMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Container for all metrics
pub struct Metrics {
    /// Per-log metrics, keyed by `log_name`
    pub log_metrics: Arc<RwLock<HashMap<String, LogMetrics>>>,
    /// Global processing metrics
    pub global_metrics: Arc<RwLock<GlobalMetrics>>,
}

impl Metrics {
    /// Creates a new Metrics container with empty log metrics and default global metrics
    #[must_use]
    pub fn new() -> Self {
        Self {
            log_metrics: Arc::new(RwLock::new(HashMap::new())),
            global_metrics: Arc::new(RwLock::new(GlobalMetrics::new())),
        }
    }

    /// Get or create metrics for a log
    #[must_use]
    pub fn get_or_create_log_metrics(&self, log_name: &str) -> LogMetrics {
        let mut log_metrics = self.log_metrics.write();
        log_metrics
            .entry(log_name.to_string())
            .or_insert_with(|| LogMetrics::new(log_name.to_string()))
            .clone()
    }

    /// Update metrics for a specific log
    pub fn update_log_metrics<F>(&self, log_name: &str, update_fn: F)
    where
        F: FnOnce(&mut LogMetrics),
    {
        let mut log_metrics = self.log_metrics.write();
        let metrics = log_metrics
            .entry(log_name.to_string())
            .or_insert_with(|| LogMetrics::new(log_name.to_string()));
        update_fn(metrics);
    }

    /// Update global metrics
    pub fn update_global_metrics<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut GlobalMetrics),
    {
        let mut global_metrics = self.global_metrics.write();
        update_fn(&mut global_metrics);
    }

    /// Get all log metrics
    #[must_use]
    pub fn get_all_log_metrics(&self) -> Vec<LogMetrics> {
        self.log_metrics.read().values().cloned().collect()
    }

    /// Get global metrics
    #[must_use]
    pub fn get_global_metrics(&self) -> GlobalMetrics {
        self.global_metrics.read().clone()
    }

    /// Get a snapshot of all metrics (logs and global)
    /// Returns (`HashMap`<`log_name`, `LogMetrics`>, `GlobalMetrics`)
    #[must_use]
    pub fn get_snapshot(&self) -> (HashMap<String, LogMetrics>, GlobalMetrics) {
        let log_metrics = self.log_metrics.read().clone();
        let global_metrics = self.global_metrics.read().clone();
        (log_metrics, global_metrics)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HTTP Metrics (lock-free)
// ============================================================================

/// Latency bucket thresholds in milliseconds
pub const LATENCY_BUCKETS: [u64; 5] = [1, 5, 10, 50, 100];

/// Number of latency buckets (5 thresholds + 1 overflow bucket)
pub const NUM_LATENCY_BUCKETS: usize = 6;

/// Lock-free metrics for a single HTTP endpoint
pub struct EndpointMetrics {
    /// Total number of requests
    pub requests: AtomicU64,
    /// Number of requests that resulted in errors
    pub errors: AtomicU64,
    /// Latency buckets: <1ms, <5ms, <10ms, <50ms, <100ms, ≥100ms
    pub latency_buckets: [AtomicU64; NUM_LATENCY_BUCKETS],
}

impl EndpointMetrics {
    /// Creates a new `EndpointMetrics` instance with all counters at zero
    #[must_use]
    pub const fn new() -> Self {
        Self {
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            latency_buckets: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    /// Record a successful request with the given latency
    ///
    /// # Panics
    /// - Infallible: 0 <= bucket < `NUM_LATENCY_BUCKETS`
    pub fn record_success(&self, latency: Duration) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        let bucket = Self::latency_bucket(latency);
        let value = self
            .latency_buckets
            .get(bucket)
            .expect("0 <= bucket < NUM_LATENCY_BUCKETS");
        value.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed request with the given latency
    ///
    /// # Panics
    /// - Infallible: 0 <= bucket < `NUM_LATENCY_BUCKETS`
    pub fn record_error(&self, latency: Duration) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.errors.fetch_add(1, Ordering::Relaxed);
        let bucket = Self::latency_bucket(latency);
        let value = self
            .latency_buckets
            .get(bucket)
            .expect("0 <= bucket < NUM_LATENCY_BUCKETS");
        value.fetch_add(1, Ordering::Relaxed);
    }

    /// Determine which latency bucket a duration falls into
    /// Returns index 0-5 for buckets: <1ms, <5ms, <10ms, <50ms, <100ms, ≥100ms
    fn latency_bucket(latency: Duration) -> usize {
        let ms = u64_millis(latency.as_millis());
        for (i, &threshold) in LATENCY_BUCKETS.iter().enumerate() {
            if ms < threshold {
                return i;
            }
        }
        NUM_LATENCY_BUCKETS - 1 // ≥100ms bucket
    }

    /// Get a snapshot of current metrics
    #[must_use]
    pub fn snapshot(&self) -> EndpointMetricsSnapshot {
        EndpointMetricsSnapshot {
            requests: self.requests.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            latency_buckets: [
                self.latency_buckets[0].load(Ordering::Relaxed),
                self.latency_buckets[1].load(Ordering::Relaxed),
                self.latency_buckets[2].load(Ordering::Relaxed),
                self.latency_buckets[3].load(Ordering::Relaxed),
                self.latency_buckets[4].load(Ordering::Relaxed),
                self.latency_buckets[5].load(Ordering::Relaxed),
            ],
        }
    }
}

impl Default for EndpointMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of endpoint metrics (non-atomic, for serialization)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EndpointMetricsSnapshot {
    /// Total number of requests
    pub requests: u64,
    /// Number of requests that resulted in errors
    pub errors: u64,
    /// Latency bucket counts: <1ms, <5ms, <10ms, <50ms, <100ms, ≥100ms
    pub latency_buckets: [u64; NUM_LATENCY_BUCKETS],
}

/// Lock-free HTTP metrics container for proof endpoints
pub struct HttpMetrics {
    /// Metrics for inclusion proof endpoint
    pub inclusion: EndpointMetrics,
    /// Metrics for consistency proof endpoint
    pub consistency: EndpointMetrics,
}

impl HttpMetrics {
    /// Creates a new `HttpMetrics` instance
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inclusion: EndpointMetrics::new(),
            consistency: EndpointMetrics::new(),
        }
    }

    /// Get a snapshot of all HTTP metrics
    #[must_use]
    pub fn snapshot(&self) -> HttpMetricsSnapshot {
        HttpMetricsSnapshot {
            inclusion: self.inclusion.snapshot(),
            consistency: self.consistency.snapshot(),
        }
    }
}

impl Default for HttpMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of all HTTP metrics (for serialization)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpMetricsSnapshot {
    /// Metrics for inclusion proof endpoint
    pub inclusion: EndpointMetricsSnapshot,
    /// Metrics for consistency proof endpoint
    pub consistency: EndpointMetricsSnapshot,
}

/// Safely converts u128 milliseconds to u64, capping at `u64::MAX`
///
/// This is used for timing metrics where u64 is expected.
pub(crate) fn u64_millis(millis: u128) -> u64 {
    if millis > u128::from(u64::MAX) {
        u64::MAX
    } else {
        u64::try_from(millis).expect("millis <= u64::MAX")
    }
}
