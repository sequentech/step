// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::monitoring::sources::DataSourceId;
use uuid::Uuid;
use windmill::services::monitoring::config_store::EventRef;

pub use windmill::services::monitoring::snapshot::{
    LiveSnapshot as SnapshotHead, ScopeCatalogue, ScopeRead,
};

/// The snapshot rows the Windmill job writes, read only; and the one row a
/// viewer adds, the set of elections they may see, so the next pass counts
/// it. Harvest never counts anything itself.
#[rocket::async_trait]
pub trait MonitoringSnapshots: Send + Sync {
    /// The run the event's dashboards show.
    async fn live(
        &self,
        event: EventRef,
    ) -> anyhow::Result<Option<SnapshotHead>>;

    /// The run at `revision` if it is complete and kept.
    async fn complete(
        &self,
        event: EventRef,
        revision: i64,
    ) -> anyhow::Result<Option<SnapshotHead>>;

    async fn read_scope(
        &self,
        event: EventRef,
        revision: i64,
        source: DataSourceId,
        election_set_key: &str,
        scope_key: &str,
    ) -> anyhow::Result<ScopeRead>;

    async fn catalogue(
        &self,
        event: EventRef,
        revision: i64,
        election_set_key: &str,
    ) -> anyhow::Result<ScopeCatalogue>;

    /// Records that a viewer asks for these elections; the set's key.
    async fn request_election_set(
        &self,
        event: EventRef,
        election_ids: &[Uuid],
    ) -> anyhow::Result<String>;
}
