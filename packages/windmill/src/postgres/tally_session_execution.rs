// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::TallySessionExecution;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct TallySessionExecutionWrapper(pub TallySessionExecution);

impl TryFrom<Row> for TallySessionExecutionWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(TallySessionExecutionWrapper(TallySessionExecution {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            current_message_id: item.try_get("current_message_id")?,
            tally_session_id: item.try_get::<_, Uuid>("tally_session_id")?.to_string(),
            session_ids: item.try_get("session_ids")?,
            status: item.try_get("status")?,
            results_event_id: item
                .try_get::<_, Option<Uuid>>("results_event_id")?
                .map(|val| val.to_string()),
        }))
    }
}
