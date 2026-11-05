// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An event's monitoring configuration, read and checked once per
//! configuration generation, so that the number of viewers does not set how
//! often every document is parsed.
//!
//! Every change of an event's configuration (a save, a reset, a mode
//! switch) raises its generation in the same transaction, and the database
//! refuses a revision written without that (see the monitoring migration's
//! triggers), so what an event had at a generation never changes: a request
//! that reads the current generation and finds it kept uses what is kept.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use windmill::services::monitoring::config_store::{
    ConfigAtGeneration, EventRef, LiveConfig,
};

type Key = (Uuid, Uuid, i64);

/// Values by event and configuration generation, least recently used out.
pub struct ConfigCache<V> {
    entries: Mutex<LruCache<Key, Arc<V>>>,
    loads: AtomicUsize,
}

impl<V> ConfigCache<V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN),
            )),
            loads: AtomicUsize::new(0),
        }
    }

    fn key(event: EventRef, generation: i64) -> Key {
        (event.tenant_id, event.election_event_id, generation)
    }

    pub fn get(&self, event: EventRef, generation: i64) -> Option<Arc<V>> {
        self.entries
            .lock()
            .unwrap()
            .get(&Self::key(event, generation))
            .cloned()
    }

    /// Keeps what was read from the database for `generation`.
    pub fn put(&self, event: EventRef, generation: i64, value: V) -> Arc<V> {
        self.loads.fetch_add(1, Ordering::Relaxed);
        let value = Arc::new(value);
        self.entries
            .lock()
            .unwrap()
            .put(Self::key(event, generation), value.clone());
        value
    }

    /// How many times a configuration was read from the database.
    pub fn loads(&self) -> usize {
        self.loads.load(Ordering::Relaxed)
    }
}

/// How many generations are kept unless
/// `HARVEST_MONITORING_CONFIG_CACHE_ENTRIES` says.
pub const DEFAULT_CONFIG_ENTRIES: usize = 64;

/// The live configurations, and those runs were counted under.
pub struct MonitoringConfigs {
    pub live: ConfigCache<LiveConfig>,
    pub at: ConfigCache<ConfigAtGeneration>,
}

impl MonitoringConfigs {
    pub fn new(capacity: usize) -> Self {
        Self {
            live: ConfigCache::new(capacity),
            at: ConfigCache::new(capacity),
        }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var("HARVEST_MONITORING_CONFIG_CACHE_ENTRIES")
                .ok()
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(DEFAULT_CONFIG_ENTRIES),
        )
    }
}

#[cfg(test)]
#[path = "../../tests/support/monitoring_config_cache.rs"]
mod monitoring_config_cache_tests;
