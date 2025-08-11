// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Pool, PoolError, Runtime, Transaction};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::VotingChannels;
use sequent_core::types::tally_sheets::AreaContestResults;
use sequent_core::types::{
    hasura::core::TallySheet,
    tally_sheets::{TallySheetStatus, VotingChannel},
};
use serde_json::Value;
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::services::reports::status;

pub struct TallySheetWrapper(pub TallySheet);

impl TryFrom<Row> for TallySheetWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        let content_val: Option<Value> = item.try_get("content")?;
        let content: Option<AreaContestResults> =
            content_val.map(|val| deserialize_value(val)).transpose()?;
        Ok(TallySheetWrapper(TallySheet {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            contest_id: item.try_get::<_, Uuid>("contest_id")?.to_string(),
            area_id: item.try_get::<_, Uuid>("area_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            reviewed_at: item.get("reviewed_at"),
            reviewed_by_user_id: item.try_get("reviewed_by_user_id")?,
            status: item.try_get("status")?,
            version: item.try_get("version")?,
            content: content,
            channel: item.try_get("channel")?,
            deleted_at: item.get("deleted_at"),
            created_by_user_id: item.try_get("created_by_user_id")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn get_approved_tally_sheets_by_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<TallySheet>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.tally_sheet
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    reviewed_at IS NOT NULL AND
                    reviewed_by_user_id IS NOT NULL AND
                    status = 'APPROVED' AND
                    deleted_at IS NULL;
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

    let election_events: Vec<TallySheet> = rows
        .into_iter()
        .map(|row| -> Result<TallySheet> {
            row.try_into()
                .map(|res: TallySheetWrapper| -> TallySheet { res.0 })
        })
        .collect::<Result<Vec<TallySheet>>>()?;

    Ok(election_events)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn soft_delete_tally_sheet(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_sheet_id: &str,
    user_id: &str,
    version: i32,
) -> Result<Option<TallySheet>> {
    let statement = hasura_transaction
        .prepare(
            format!(
                r#"
        UPDATE sequent_backend.tally_sheet tally_sheet
        SET
            deleted_at = NOW(),
            last_updated_at = NOW()
        WHERE
            tally_sheet.tenant_id = $1 AND
            tally_sheet.election_event_id = $2 AND
            tally_sheet.id = $3 AND
            tally_sheet.deleted_at IS NULL AND
            tally_sheet.version = $4
        RETURNING *
    "#
            )
            .as_str(),
        )
        .await?;

    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let tally_sheet_uuid: uuid::Uuid = Uuid::parse_str(tally_sheet_id)
        .map_err(|err| anyhow!("Error parsing tally_sheet_id as UUID: {}", err))?;
    let params: Vec<&(dyn ToSql + Sync)> = vec![
        &tenant_uuid,
        &election_event_uuid,
        &tally_sheet_uuid,
        &user_id,
        &version,
    ];
    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let elements: Vec<TallySheet> = rows
        .into_iter()
        .map(|row| -> Result<TallySheet> {
            row.try_into()
                .map(|res: TallySheetWrapper| -> TallySheet { res.0 })
        })
        .collect::<Result<Vec<TallySheet>>>()?;

    match elements.len() {
        0 => Err(anyhow!("No rows affected")),
        1 => Ok(Some(elements[0].clone())),
        _ => Err(anyhow!("Unexpected rows affected {}", elements.len())),
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn review_tally_sheet_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_sheet_id: &str,
    user_id: &str,
    status: TallySheetStatus,
    version: i32,
) -> Result<Option<TallySheet>> {
    let statement = hasura_transaction
        .prepare(
            format!(
                r#"
        UPDATE sequent_backend.tally_sheet tally_sheet
        SET
            status = $5,
            reviewed_at = NOW(),
            reviewed_by_user_id = $6,
            last_updated_at = NOW()
        WHERE
            tally_sheet.tenant_id = $1 AND
            tally_sheet.election_event_id = $2 AND
            tally_sheet.id = $3 AND
            tally_sheet.deleted_at IS NULL AND
            tally_sheet.version = $4
        RETURNING *
    "#
            )
            .as_str(),
        )
        .await?;

    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let tally_sheet_uuid: uuid::Uuid = Uuid::parse_str(tally_sheet_id)
        .map_err(|err| anyhow!("Error parsing tally_sheet_id as UUID: {}", err))?;
    let status_str = status.to_string();
    let params: Vec<&(dyn ToSql + Sync)> = vec![
        &tenant_uuid,
        &election_event_uuid,
        &tally_sheet_uuid,
        &version,
        &status_str,
        &user_id,
    ];
    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let elements: Vec<TallySheet> = rows
        .into_iter()
        .map(|row| -> Result<TallySheet> {
            row.try_into()
                .map(|res: TallySheetWrapper| -> TallySheet { res.0 })
        })
        .collect::<Result<Vec<TallySheet>>>()?;

    match elements.len() {
        0 => Err(anyhow!("No rows affected")),
        1 => Ok(Some(elements[0].clone())),
        _ => Err(anyhow!("Unexpected rows affected {}", elements.len())),
    }
}

#[instrument(skip(hasura_transaction, content), err)]
pub async fn insert_tally_sheet(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    area_id: &str,
    content: &AreaContestResults,
    channel: &VotingChannel,
    created_by_user_id: &str,
    status: TallySheetStatus,
    version: i32,
) -> Result<TallySheet> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let contest_uuid: uuid::Uuid = Uuid::parse_str(contest_id)
        .map_err(|err| anyhow!("Error parsing contest_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid = Uuid::parse_str(area_id)
        .map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;
    let content_value = serde_json::to_value(content)?;
    let channel_str = channel.to_string();
    let status_str = status.to_string();

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.tally_sheet
                (tenant_id, election_event_id, election_id, contest_id, area_id, created_at, last_updated_at, content, channel, created_by_user_id, status, version)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    NOW(),
                    NOW(),
                    $6,
                    $7,
                    $8,
                    $9,
                    $10
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
                &tenant_uuid,
                &election_event_uuid,
                &election_uuid,
                &contest_uuid,
                &area_uuid,
                &content_value,
                &channel_str,
                &created_by_user_id.to_string(),
                &status_str,
                &version,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting tally sheet: {}", err))?;

    let elements: Vec<TallySheet> = rows
        .into_iter()
        .map(|row| -> Result<TallySheet> {
            row.try_into()
                .map(|res: TallySheetWrapper| -> TallySheet { res.0 })
        })
        .collect::<Result<Vec<TallySheet>>>()?;

    if 1 == elements.len() {
        Ok(elements[0].clone())
    } else {
        Err(anyhow!("Unexpected rows affected {}", elements.len()))
    }
}
