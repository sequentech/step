// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::database::DatabasePools;
use crate::ports::monitoring_snapshots::{
    KeptRun, MonitoringSnapshots, ScopeCatalogue, ScopeRead, SnapshotHead,
};
use anyhow::Context;
use sequent_core::monitoring::sources::DataSourceId;
use std::sync::Arc;
use uuid::Uuid;
use windmill::services::monitoring::config_store::EventRef;
use windmill::services::monitoring::snapshot;

/// The Windmill job's snapshot tables, each read in a transaction of its
/// own on the Hasura database.
pub struct WindmillMonitoringSnapshots {
    pub databases: Arc<dyn DatabasePools>,
}

impl WindmillMonitoringSnapshots {
    async fn client(&self) -> anyhow::Result<deadpool_postgres::Client> {
        self.databases
            .hasura()
            .await
            .get()
            .await
            .context("Failed to get a Hasura database connection")
    }
}

#[rocket::async_trait]
impl MonitoringSnapshots for WindmillMonitoringSnapshots {
    async fn live(
        &self,
        event: EventRef,
    ) -> anyhow::Result<Option<SnapshotHead>> {
        let mut client = self.client().await?;
        let transaction = client.transaction().await?;
        let head = snapshot::live_snapshot(&transaction, event).await?;
        transaction.commit().await?;
        Ok(head)
    }

    async fn run(
        &self,
        event: EventRef,
        revision: i64,
    ) -> anyhow::Result<KeptRun> {
        let mut client = self.client().await?;
        let transaction = client.transaction().await?;
        let run = snapshot::kept_run(&transaction, event, revision).await?;
        transaction.commit().await?;
        Ok(run)
    }

    async fn read_scope(
        &self,
        event: EventRef,
        revision: i64,
        source: DataSourceId,
        election_set_key: &str,
        scope_key: &str,
    ) -> anyhow::Result<ScopeRead> {
        let mut client = self.client().await?;
        let transaction = client.transaction().await?;
        let read = snapshot::read_scope(
            &transaction,
            event,
            revision,
            source,
            election_set_key,
            scope_key,
        )
        .await?;
        transaction.commit().await?;
        Ok(read)
    }

    async fn catalogue(
        &self,
        event: EventRef,
        revision: i64,
        election_set_key: &str,
    ) -> anyhow::Result<ScopeCatalogue> {
        let mut client = self.client().await?;
        let transaction = client.transaction().await?;
        let catalogue = snapshot::scope_catalogue(
            &transaction,
            event,
            revision,
            election_set_key,
        )
        .await?;
        transaction.commit().await?;
        Ok(catalogue)
    }

    async fn request_election_set(
        &self,
        event: EventRef,
        election_ids: &[Uuid],
    ) -> anyhow::Result<String> {
        let mut client = self.client().await?;
        let transaction = client.transaction().await?;
        let key =
            snapshot::request_election_set(&transaction, event, election_ids)
                .await?;
        transaction.commit().await?;
        Ok(key)
    }
}
