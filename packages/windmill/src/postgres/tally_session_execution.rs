// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::{ceremonies::TallyCeremonyStatus, hasura::core::TallySessionExecution};
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

#[instrument(skip(hasura_transaction, status), err)]
pub async fn insert_tally_session_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    current_message_id: i32,
    tally_session_id: &str,
    status: Option<TallyCeremonyStatus>,
    results_event_id: Option<String>,
    session_ids: Option<Vec<i32>>,
) -> Result<TallySessionExecution> {
    let json_status = match status {
        Some(value) => Some(serde_json::to_value(value)?),
        None => None,
    };
    let results_event_uuid = match results_event_id {
        Some(value) => Some(Uuid::parse_str(&value)?),
        None => None,
    };
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.tally_session_execution
                (tenant_id, election_event_id, current_message_id, tally_session_id, status, results_event_id, session_ids)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7
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
                &current_message_id,
                &Uuid::parse_str(tally_session_id)?,
                &json_status,
                &results_event_uuid,
                &session_ids,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting row: {}", err))?;

    let values: Vec<TallySessionExecution> = rows
        .into_iter()
        .map(|row| -> Result<TallySessionExecution> {
            row.try_into()
                .map(|res: TallySessionExecutionWrapper| -> TallySessionExecution { res.0 })
        })
        .collect::<Result<Vec<TallySessionExecution>>>()?;

    let Some(value) = values.first() else {
        return Err(anyhow!("Error inserting row"));
    };
    Ok(value.clone())
}

pub async fn get_tally_session_executions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<Vec<TallySessionExecution>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.tally_session_execution
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    tally_session_id = $3
                ORDER BY created_at DESC;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(tally_session_id)?,
            ],
        )
        .await?;

    let elements: Vec<TallySessionExecution> = rows
        .into_iter()
        .map(|row| -> Result<TallySessionExecution> {
            row.try_into()
                .map(|res: TallySessionExecutionWrapper| -> TallySessionExecution { res.0 })
        })
        .collect::<Result<Vec<TallySessionExecution>>>()?;

    Ok(elements)
}

pub async fn get_last_tally_session_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<Option<TallySessionExecution>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.tally_session_execution
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    tally_session_id = $3
                ORDER BY created_at DESC
                LIMIT 1;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(tally_session_id)?,
            ],
        )
        .await?;

    let elements: Vec<TallySessionExecution> = rows
        .into_iter()
        .map(|row| -> Result<TallySessionExecution> {
            row.try_into()
                .map(|res: TallySessionExecutionWrapper| -> TallySessionExecution { res.0 })
        })
        .collect::<Result<Vec<TallySessionExecution>>>()?;

    Ok(elements.first().cloned())
}

pub async fn get_event_tally_session_executions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<TallySessionExecution>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.tally_session_execution
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let elements: Vec<TallySessionExecution> = rows
        .into_iter()
        .map(|row| -> Result<TallySessionExecution> {
            row.try_into()
                .map(|res: TallySessionExecutionWrapper| -> TallySessionExecution { res.0 })
        })
        .collect::<Result<Vec<TallySessionExecution>>>()?;

    Ok(elements)
}
