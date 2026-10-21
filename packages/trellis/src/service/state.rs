use crate::{LeafHash, tree::CtMerkleTree};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

use crate::service::metrics::{HttpMetrics, Metrics};

/// Processor state for graceful shutdown and pause control
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorState {
    /// Normal operation - processor is actively processing batches
    Running = 0,
    /// Paused state - processor sleeps but server continues running
    Paused = 1,
    /// Stopping state - processor exits, causing the entire application to shut down
    Stopping = 2,
}

impl ProcessorState {
    /// Converts a u8 value to `ProcessorState`
    ///
    /// Used to read the atomic state value. Unknown values default to Running.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => ProcessorState::Paused,
            2 => ProcessorState::Stopping,
            _ => ProcessorState::Running,
        }
    }
}

/// Shared state between HTTP server and periodic processor
#[derive(Clone)]
pub struct AppState {
    /// Map of log name to merkle state (tree + last processed ID)
    /// `DashMap` allows concurrent access without a single global lock
    pub merkle_states: Arc<DashMap<String, Arc<parking_lot::RwLock<MerkleState>>>>,
    /// Database connection pool for operations that need it
    pub db_pool: deadpool_postgres::Pool,
    /// Processing metrics for monitoring and diagnostics
    pub metrics: Arc<Metrics>,
    /// HTTP endpoint metrics (lock-free atomics)
    pub http_metrics: Arc<HttpMetrics>,
    /// Processor state for graceful shutdown and pause control
    /// 0=Running, 1=Paused, 2=Stopping
    pub processor_state: Arc<AtomicU8>,
}

/// A combined state holding the merkle tree and its corresponding last processed ID.
/// This ensures that readers always see a consistent view of both values.
/// This is a service-layer wrapper around the core `CtMerkleTree` to coordinate with HTTP endpoints.
pub struct MerkleState {
    /// The history-tracking merkle tree storing leaf hashes and their indices
    pub tree: CtMerkleTree,
    /// The ID of the last `merkle_log` entry incorporated into the tree
    pub last_processed_id: i64,
}

impl MerkleState {
    /// Creates a new empty merkle state
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: CtMerkleTree::new(),
            last_processed_id: 0,
        }
    }

    /// Updates both the tree and `last_processed_id` atomically
    pub fn update_with_entry(&mut self, hash: LeafHash, id: i64) {
        self.tree.push(hash);
        self.last_processed_id = id;
    }
}

impl Default for MerkleState {
    fn default() -> Self {
        Self::new()
    }
}
