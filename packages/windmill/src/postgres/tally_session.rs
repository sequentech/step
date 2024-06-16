// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::TallySession;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct TallySessionWrapper(pub TallySession);

impl TryFrom<Row> for TallySessionWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(TallySessionWrapper(TallySession {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            election_ids: item
                .try_get::<_, Option<Vec<Uuid>>>("election_ids")?
                .map(|uuids| {
                    uuids
                        .clone()
                        .into_iter()
                        .map(|uuid| uuid.to_string())
                        .collect()
                }),
            area_ids: item
                .try_get::<_, Option<Vec<Uuid>>>("area_ids")?
                .map(|uuids| {
                    uuids
                        .clone()
                        .into_iter()
                        .map(|uuid| uuid.to_string())
                        .collect()
                }),
            is_execution_completed: item.try_get("is_execution_completed")?,
            keys_ceremony_id: item.try_get("keys_ceremony_id")?,
            execution_status: item.try_get("execution_status")?,
            threshold: item.try_get("threshold")?,
        }))
    }
}
