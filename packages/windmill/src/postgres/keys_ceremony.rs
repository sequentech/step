// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::KeysCeremony;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct KeysCeremonyWrapper(pub KeysCeremony);

impl TryFrom<Row> for KeysCeremonyWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(KeysCeremonyWrapper(KeysCeremony {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            trustee_ids: item
                .try_get::<_, Vec<Uuid>>("trustee_ids")?
                .iter()
                .map(|uuid| uuid.to_string())
                .collect(),
            status: item.try_get("status")?,
            execution_status: item.try_get("description")?,
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            threshold: item.try_get("threshold")?,
        }))
    }
}
