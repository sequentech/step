// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::monitoring_snapshots::{
    MonitoringSnapshots, ScopeCatalogue, ScopeRead, SnapshotHead,
};
use chrono::{TimeZone, Utc};
use sequent_core::monitoring::scope::election_set_key;
use sequent_core::monitoring::sources::DataSourceId;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use windmill::services::monitoring::config_store::EventRef;

/// Snapshot rows held in memory, and every call made on them.
#[derive(Default)]
pub struct MemorySnapshots {
    pub head: Mutex<Option<SnapshotHead>>,
    /// By source, set key and scope key; anything else reads as `fallback`.
    pub scopes: Mutex<HashMap<(DataSourceId, String, String), ScopeRead>>,
    pub fallback: Mutex<Option<ScopeRead>>,
    pub catalogue: Mutex<ScopeCatalogue>,
    calls: Mutex<Vec<String>>,
    requested: Mutex<Vec<Vec<Uuid>>>,
}

impl MemorySnapshots {
    /// A live run at `revision`, whose scopes read as `fallback`.
    pub fn at(revision: i64, fallback: ScopeRead) -> Self {
        let snapshots = Self::default();
        *snapshots.head.lock().unwrap() = Some(SnapshotHead {
            revision,
            as_of: Utc.with_ymd_and_hms(2026, 5, 4, 10, 0, 0).unwrap(),
            checked_at: None,
            settings_revision: 1,
            config_generation: 1,
        });
        *snapshots.fallback.lock().unwrap() = Some(fallback);
        snapshots
    }

    pub fn with_scope(
        self,
        source: DataSourceId,
        set_key: &str,
        scope_key: &str,
        read: ScopeRead,
    ) -> Self {
        self.scopes
            .lock()
            .unwrap()
            .insert((source, set_key.to_string(), scope_key.to_string()), read);
        self
    }

    pub fn with_catalogue(self, catalogue: ScopeCatalogue) -> Self {
        *self.catalogue.lock().unwrap() = catalogue;
        self
    }

    /// Every call, as `name` or `name:argument`, in order.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    pub fn requested(&self) -> Vec<Vec<Uuid>> {
        self.requested.lock().unwrap().clone()
    }

    fn call(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
}

#[rocket::async_trait]
impl MonitoringSnapshots for MemorySnapshots {
    async fn live(
        &self,
        _event: EventRef,
    ) -> anyhow::Result<Option<SnapshotHead>> {
        self.call("live".into());
        Ok(self.head.lock().unwrap().clone())
    }

    async fn complete(
        &self,
        _event: EventRef,
        revision: i64,
    ) -> anyhow::Result<Option<SnapshotHead>> {
        self.call(format!("complete:{revision}"));
        Ok(self
            .head
            .lock()
            .unwrap()
            .clone()
            .filter(|head| head.revision == revision))
    }

    async fn read_scope(
        &self,
        _event: EventRef,
        revision: i64,
        source: DataSourceId,
        election_set_key: &str,
        scope_key: &str,
    ) -> anyhow::Result<ScopeRead> {
        self.call(format!(
            "read_scope:{revision}:{source}:{election_set_key}:{scope_key}"
        ));
        let key = (source, election_set_key.to_string(), scope_key.to_string());
        if let Some(read) = self.scopes.lock().unwrap().get(&key) {
            return Ok(read.clone());
        }
        Ok(self
            .fallback
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(ScopeRead::NotCounted))
    }

    async fn catalogue(
        &self,
        _event: EventRef,
        revision: i64,
        election_set_key: &str,
    ) -> anyhow::Result<ScopeCatalogue> {
        self.call(format!("catalogue:{revision}:{election_set_key}"));
        Ok(self.catalogue.lock().unwrap().clone())
    }

    async fn request_election_set(
        &self,
        _event: EventRef,
        election_ids: &[Uuid],
    ) -> anyhow::Result<String> {
        self.call("request_election_set".into());
        self.requested.lock().unwrap().push(election_ids.to_vec());
        election_set_key(election_ids.iter().map(Uuid::to_string))
            .map_err(|error| anyhow::anyhow!("{error}"))
    }
}
