// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Drawn charts, kept so that the number of viewers does not set the
//! number of draws.
//!
//! A chart is keyed by everything that decides what it shows and how: the
//! revisions of its documents, the snapshot run, the elections the viewer
//! may see, the scope, the selector values, the engine, the locale, the
//! width (in buckets) and the palette. Viewers asking for the same chart at
//! once wait on one draw (single flight); later ones are served from a
//! least-recently-used cache. A failed draw is shared with whoever waited on
//! it, and not kept.

use indexmap::IndexMap;
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

/// The SHA-256 of a chart's [`RenderKeyParts`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderKey([u8; 32]);

impl std::fmt::Debug for RenderKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "RenderKey({})", hex::encode(&self.0[..8]))
    }
}

/// Everything that decides what a drawn chart shows and how.
#[derive(Debug, Clone)]
pub struct RenderKeyParts<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    /// Each document as its id and revision; a draft's revision is the
    /// digest of its text.
    pub dashboard: (&'a str, String),
    pub widget: (&'a str, String),
    pub theme: (&'a str, String),
    pub settings_revision: String,
    pub snapshot_revision: i64,
    pub election_set_key: &'a str,
    pub scope_key: String,
    pub selector_values: IndexMap<String, String>,
    pub renderer_version: String,
    pub locale: &'a str,
    pub width_bucket: u32,
    pub color_scheme: &'a str,
}

impl RenderKeyParts<'_> {
    pub fn key(&self) -> RenderKey {
        let mut selector_values: Vec<(&String, &String)> =
            self.selector_values.iter().collect();
        selector_values.sort();
        // A JSON array: every part is delimited, so no two keys run together.
        let canonical = serde_json::json!([
            self.tenant_id,
            self.election_event_id,
            [self.dashboard.0, self.dashboard.1],
            [self.widget.0, self.widget.1],
            [self.theme.0, self.theme.1],
            self.settings_revision,
            self.snapshot_revision,
            self.election_set_key,
            self.scope_key,
            selector_values,
            self.renderer_version,
            self.locale,
            self.width_bucket,
            self.color_scheme,
        ]);
        RenderKey(Sha256::digest(canonical.to_string().as_bytes()).into())
    }
}

type Flight<V, E> = Arc<OnceCell<Result<Arc<V>, E>>>;

/// Drawn charts by key, and the draws under way.
pub struct RenderCache<V, E> {
    entries: Mutex<LruCache<RenderKey, Arc<V>>>,
    flights: Mutex<HashMap<RenderKey, Flight<V, E>>>,
}

/// How many charts are kept unless `HARVEST_MONITORING_CACHE_ENTRIES` says.
pub const DEFAULT_ENTRIES: usize = 1024;

impl<V, E: Clone> RenderCache<V, E> {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN),
            )),
            flights: Mutex::new(HashMap::new()),
        }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var("HARVEST_MONITORING_CACHE_ENTRIES")
                .ok()
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(DEFAULT_ENTRIES),
        )
    }

    /// The chart under `key`: kept, being drawn for someone else, or drawn
    /// now by `draw`.
    pub async fn get_or_render<F, Fut>(
        &self,
        key: RenderKey,
        draw: F,
    ) -> Result<Arc<V>, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, E>>,
    {
        if let Some(kept) = self.entries.lock().unwrap().get(&key) {
            return Ok(kept.clone());
        }
        let flight = self
            .flights
            .lock()
            .unwrap()
            .entry(key)
            .or_insert_with(|| Arc::new(OnceCell::new()))
            .clone();
        let result = flight
            .get_or_init(|| async {
                let drawn = draw().await.map(Arc::new);
                if let Ok(drawn) = &drawn {
                    self.entries.lock().unwrap().put(key, drawn.clone());
                }
                drawn
            })
            .await
            .clone();
        let mut flights = self.flights.lock().unwrap();
        if flights
            .get(&key)
            .is_some_and(|current| Arc::ptr_eq(current, &flight))
        {
            flights.remove(&key);
        }
        result
    }
}

#[cfg(test)]
#[path = "../../tests/support/monitoring_cache.rs"]
mod monitoring_cache_tests;
