// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::{
    ceremonies::TallyExecutionStatus,
    hasura::core::{TallySession, TallySessionConfiguration},
};
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
            keys_ceremony_id: item.try_get::<_, Uuid>("keys_ceremony_id")?.to_string(),
            execution_status: item.try_get("execution_status")?,
            threshold: item.try_get::<_, i32>("threshold")? as i64,
            configuration: item
                .try_get::<_, Option<serde_json::value::Value>>("configuration")?
                .map(|val| serde_json::from_value(val))
                .transpose()?,
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_tally_session(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: Vec<String>,
    area_ids: Vec<String>,
    tally_session_id: &str,
    keys_ceremony_id: &str,
    execution_status: TallyExecutionStatus,
    threshold: i32,
    configuration: Option<TallySessionConfiguration>,
) -> Result<TallySession> {
    let configuration_json = serde_json::to_value(&configuration)?;
    let election_uuids: Vec<Uuid> = election_ids
        .iter()
        .map(|id| Uuid::parse_str(&id).map_err(|err| anyhow!("{:?}", err)))
        .collect::<Result<Vec<Uuid>>>()?;
    let area_uuids: Vec<Uuid> = area_ids
        .iter()
        .map(|id| Uuid::parse_str(&id).map_err(|err| anyhow!("{:?}", err)))
        .collect::<Result<Vec<Uuid>>>()?;
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.tally_session
                (tenant_id, election_event_id, election_ids, area_ids, id, keys_ceremony_id, execution_status, threshold, configuration)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    $9
                )
                RETURNING
                    *;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &election_uuids,
                &area_uuids,
                &Uuid::parse_str(tally_session_id)?,
                &Uuid::parse_str(keys_ceremony_id)?,
                &Some(execution_status.to_string()),
                &threshold,
                &configuration_json,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting row: {}", err))?;

    let values: Vec<TallySession> = rows
        .into_iter()
        .map(|row| -> Result<TallySession> {
            row.try_into()
                .map(|res: TallySessionWrapper| -> TallySession { res.0 })
        })
        .collect::<Result<Vec<TallySession>>>()?;

    let Some(value) = values.first() else {
        return Err(anyhow!("Error inserting row"));
    };
    Ok(value.clone())
}
