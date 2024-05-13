// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::scheduled_event::{CronConfig, EventProcessors};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct PostgresScheduledEvent {
    pub id: String,
    pub tenant_id: Option<String>,
    pub election_event_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub event_processor: Option<EventProcessors>,
    pub cron_config: Option<CronConfig>,
    pub event_payload: Option<Value>,
    pub task_id: Option<String>,
}

impl TryFrom<Row> for PostgresScheduledEvent {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        let event_processors_js: Option<Value> = item.try_get("event_processor")?;
        let event_processors: Option<EventProcessors> =
            event_processors_js.map(|val| serde_json::from_value(val).unwrap());

        let cron_config_js: Option<Value> = item.try_get("cron_config")?;
        let cron_config: Option<CronConfig> =
            cron_config_js.map(|val| serde_json::from_value(val).unwrap());

        Ok(PostgresScheduledEvent {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item
                .try_get::<_, Option<Uuid>>("tenant_id")?
                .map(|val| val.to_string()),
            election_event_id: item
                .try_get::<_, Option<Uuid>>("election_event_id")?
                .map(|val| val.to_string()),
            created_at: item.get("created_at"),
            stopped_at: item.get("created_at"),
            labels: item.get("labels"),
            annotations: item.get("annotations"),
            event_processor: event_processors,
            cron_config: cron_config,
            event_payload: item.get("event_payload"),
            task_id: item.get("task_id"),
        })
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn find_scheduled_event_by_task_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    task_id: &str,
) -> Result<Option<PostgresScheduledEvent>> {
    let tenant_uuid: uuid::Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM "sequent_backend".scheduled_event
            WHERE
                tenant_id = $1
                AND election_event_id = $2
                AND task_id = $3
                AND stopped_at IS NULL
            "#,
        )
        .await?;

    let task_id_s = task_id.to_string();

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &election_event_uuid, &task_id_s],
        )
        .await
        .map_err(|err| anyhow!("Error running the document query: {err}"))?;

    let scheduled_events = rows
        .into_iter()
        .map(|row| -> Result<PostgresScheduledEvent> { row.try_into() })
        .collect::<Result<Vec<PostgresScheduledEvent>>>()
        .with_context(|| "Error converting rows into documents")?;

    Ok(scheduled_events.get(0).cloned())
}
