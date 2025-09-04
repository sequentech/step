// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::{
    serialization::deserialize_with_path::deserialize_value,
    types::{
        ceremonies::TallyExecutionStatus,
        hasura::core::{TallySession, TallySessionConfiguration},
    },
};
use serde::Serialize;
use serde_json::value::Value;
use std::str::FromStr;
use tokio_postgres::{row::Row, types::ToSql};
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
                .try_get::<_, Option<Value>>("configuration")?
                .map(|val| deserialize_value(val))
                .transpose()?,
            tally_type: item.try_get("tally_type")?,
            permission_label: item.get::<_, Option<Vec<String>>>("permission_label"),
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
    tally_type: &str,
    annotations: Value,
    permission_label: Vec<String>,
) -> Result<TallySession> {
    let configuration_json: Option<Value> = configuration
        .map(|value| serde_json::to_value(&value))
        .transpose()?;
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
                (tenant_id, election_event_id, election_ids, area_ids, id, keys_ceremony_id, execution_status, threshold, configuration, tally_type, annotations, permission_label)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    $9,
                    $10,
                    $11,
                    $12
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
                &tally_type.to_string(),
                &annotations,
                &permission_label,
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

#[instrument(err, skip(hasura_transaction))]
pub async fn get_tally_sessions_by_election_event_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    only_active: bool,
) -> Result<Vec<TallySession>> {
    let query = format!(
        r#"
        SELECT
            *
        FROM
            sequent_backend.tally_session
        WHERE
            tenant_id = $1 AND
            election_event_id = $2
            {}
        ORDER BY
            created_at DESC;
    "#,
        if only_active {
            r#" AND is_execution_completed IS FALSE
                AND execution_status = 'IN_PROGRESS'"#
        } else {
            ""
        }
    );
    let statement = hasura_transaction.prepare(&query).await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let elements: Vec<TallySession> = rows
        .into_iter()
        .map(|row| -> Result<TallySession> {
            row.try_into()
                .map(|res: TallySessionWrapper| -> TallySession { res.0 })
        })
        .collect::<Result<Vec<TallySession>>>()?;

    Ok(elements)
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_tally_session_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<TallySession> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.tally_session
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    id = $3;
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

    let elements: Vec<TallySession> = rows
        .into_iter()
        .map(|row| -> Result<TallySession> {
            row.try_into()
                .map(|res: TallySessionWrapper| -> TallySession { res.0 })
        })
        .collect::<Result<Vec<TallySession>>>()?;

    elements
        .get(0)
        .map(|tally_session: &TallySession| tally_session.clone())
        .ok_or(anyhow!("Tally Session {tally_session_id} not found"))
}

#[instrument(err, skip_all)]
pub async fn update_tally_session_annotation(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    annotations: Value,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                sequent_backend.tally_session
            SET
                annotations = $1
            WHERE
                id = $2 AND
                tenant_id = $3 AND
                election_event_id = $4;
        "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &annotations,
                &Uuid::parse_str(tally_session_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(&election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running query: {err}"))?;

    Ok(())
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_tally_sessions_by_election_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Vec<TallySession>> {
    let query = format!(
        r#"
        SELECT
            *
        FROM
            sequent_backend.tally_session
        WHERE
            tenant_id = $1
            AND election_event_id = $2
            AND $3 = ANY(election_ids)
        ORDER BY
            created_at DESC;
        "#
    );

    let statement = hasura_transaction.prepare(&query).await?;

    // Note: tenant_id is parsed as a UUID while election_id is a string.
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(&election_event_id)?,
                &Uuid::parse_str(&election_id)?,
            ],
        )
        .await?;

    let tally_sessions: Vec<TallySession> = rows
        .into_iter()
        .map(|row| -> Result<TallySession> { row.try_into().map(|res: TallySessionWrapper| res.0) })
        .collect::<Result<Vec<TallySession>>>()?;

    Ok(tally_sessions)
}

#[instrument(err, skip_all)]
pub async fn update_tally_session_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    execution_status: &TallyExecutionStatus,
    is_execution_completed: bool,
) -> Result<()> {
    println!("Updating tally session status:{:?}", &tally_session_id);
    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                sequent_backend.tally_session
            SET
                execution_status = $1,
                is_execution_completed = $5
            WHERE
                id = $2 AND
                tenant_id = $3 AND
                election_event_id = $4;
        "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &execution_status.to_string(),
                &Uuid::parse_str(tally_session_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(&election_event_id)?,
                &is_execution_completed,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running query update tally sesstion status: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn set_tally_session_completed(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    execution_status: TallyExecutionStatus,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                sequent_backend.tally_session
            SET
                execution_status = $1,
                is_execution_completed = TRUE
            WHERE
                id = $2 AND
                tenant_id = $3 AND
                election_event_id = $4;
        "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &execution_status.to_string(),
                &Uuid::parse_str(tally_session_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(&election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running query update tally sesstion status: {err}"))?;

    Ok(())
}

#[derive(Debug, Serialize)]
struct InsertableTallySession {
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
    keys_ceremony_id: Uuid,
    election_ids: Vec<Uuid>,
    area_ids: Vec<Uuid>,
    execution_status: Option<String>,
    threshold: i32,
    configuration: Option<Value>,
    tally_type: Option<String>,
    annotations: Option<Value>,
    permission_label: Option<Vec<String>>,
}

#[instrument(skip(hasura_transaction, sessions), err)]
pub async fn insert_many_tally_sessions(
    hasura_transaction: &Transaction<'_>,
    sessions: Vec<TallySession>,
) -> Result<Vec<TallySession>> {
    if sessions.is_empty() {
        return Ok(vec![]);
    }

    let insertable_sessions: Vec<InsertableTallySession> = sessions
        .into_iter()
        .map(|session| {
            let configuration_json: Option<Value> = session
                .configuration
                .map(|value| serde_json::to_value(&value))
                .transpose()?;

            let election_ids = session
                .election_ids
                .unwrap_or_default()
                .into_iter()
                .map(|id| {
                    Uuid::parse_str(&id).map_err(|e| anyhow!("Invalid election_id: {id} - {e}"))
                })
                .collect::<Result<Vec<Uuid>>>()?;

            let area_ids = session
                .area_ids
                .unwrap_or_default()
                .into_iter()
                .map(|id| Uuid::parse_str(&id).map_err(|e| anyhow!("Invalid area_id: {id} - {e}")))
                .collect::<Result<Vec<Uuid>>>()?;

            Ok(InsertableTallySession {
                tenant_id: Uuid::parse_str(&session.tenant_id)?,
                election_event_id: Uuid::parse_str(&session.election_event_id)?,
                id: Uuid::parse_str(&session.id)?,
                keys_ceremony_id: Uuid::parse_str(&session.keys_ceremony_id)?,
                election_ids,
                area_ids,
                execution_status: session.execution_status.clone(),
                threshold: session.threshold as i32,
                configuration: configuration_json,
                tally_type: session.tally_type.clone(),
                annotations: session.annotations.clone(),
                permission_label: session.permission_label.clone(),
            })
        })
        .collect::<Result<_>>()?;

    let json_data = serde_json::to_value(&insertable_sessions)?;

    let sql = r#"
        WITH data AS (
            SELECT * FROM jsonb_to_recordset($1::jsonb) AS t(
                tenant_id UUID,
                election_event_id UUID,
                id UUID,
                keys_ceremony_id UUID,
                election_ids UUID[],
                area_ids UUID[],
                execution_status TEXT,
                threshold INTEGER,
                configuration JSONB,
                tally_type TEXT,
                annotations JSONB,
                permission_label TEXT[]
            )
        )
        INSERT INTO sequent_backend.tally_session (
            tenant_id, election_event_id, id, keys_ceremony_id,
            election_ids, area_ids, execution_status, threshold,
            configuration, tally_type, annotations, permission_label
        )
        SELECT
            tenant_id, election_event_id, id, keys_ceremony_id,
            election_ids, area_ids, execution_status, threshold,
            configuration, tally_type, annotations, permission_label
        FROM data
        RETURNING *;
    "#;

    let statement = hasura_transaction.prepare(sql).await?;
    let rows = hasura_transaction.query(&statement, &[&json_data]).await?;

    let result_sessions = rows
        .into_iter()
        .map(|row| {
            let wrapper: TallySessionWrapper = row.try_into()?;
            Ok(wrapper.0)
        })
        .collect::<Result<Vec<TallySession>>>()?;

    Ok(result_sessions)
}

#[instrument(err, skip_all)]
pub async fn get_tally_session_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<TallyExecutionStatus> {
    // 1) prepare the SELECT
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                execution_status
            FROM
                sequent_backend.tally_session
            WHERE
                id = $1
                AND tenant_id = $2
                AND election_event_id = $3
            LIMIT 1;
            "#,
        )
        .await?;

    // 2) run the query and grab the single row
    let row = hasura_transaction
        .query_one(
            &statement,
            &[
                &Uuid::parse_str(tally_session_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error fetching tally session status: {err}"))?;

    // 3) extract the string and convert back into your enum
    let status_str: String = row.get("execution_status");
    let status =
        TallyExecutionStatus::from_str(&status_str).unwrap_or(TallyExecutionStatus::STARTED);
    Ok(status)
}
