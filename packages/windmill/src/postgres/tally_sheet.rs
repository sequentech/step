// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::tally_sheets::AreaContestResults;
use sequent_core::types::{
    hasura::core::TallySheet,
    tally_sheets::{TallySheetStatus, VotingChannel},
};
use serde_json::Value;
use std::str::FromStr;
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::instrument;
use uuid::Uuid;

pub struct TallySheetWrapper(pub TallySheet);

impl TryFrom<Row> for TallySheetWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        let content_val: Option<Value> = item.try_get("content")?;
        let content: Option<AreaContestResults> = content_val.map(deserialize_value).transpose()?;
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
            status: TallySheetStatus::from_str(&item.try_get::<_, String>("status")?)
                .map_err(|err| anyhow!("Invalid tally sheet status: {err}"))?,
            version: item.try_get("version")?,
            content,
            channel: item.try_get("channel")?,
            deleted_at: item.get("deleted_at"),
            created_by_user_id: item.try_get("created_by_user_id")?,
            import_id: item
                .try_get::<_, Option<Uuid>>("import_id")?
                .map(|value| value.to_string()),
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await?;

    let tally_sheets: Vec<TallySheet> = rows
        .into_iter()
        .map(|row| -> Result<TallySheet> {
            row.try_into()
                .map(|res: TallySheetWrapper| -> TallySheet { res.0 })
        })
        .collect::<Result<Vec<TallySheet>>>()?;

    Ok(tally_sheets)
}

#[instrument(err, skip_all)]
pub async fn get_latest_approved_tally_sheet(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    contest_id: &str,
    channel: &VotingChannel,
) -> Result<Option<TallySheet>> {
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
                    election_id = $3 AND
                    area_id = $4 AND
                    contest_id = $5 AND
                    channel = $6 AND
                    status = 'APPROVED' AND
                    deleted_at IS NULL
                ORDER BY
                    version DESC
                LIMIT 1;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
                &parse_uuid_v4(area_id)?,
                &parse_uuid_v4(contest_id)?,
                &channel.to_string(),
            ],
        )
        .await?;

    let elements = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: TallySheetWrapper| res.0))
        .collect::<Result<Vec<TallySheet>>>()?;

    Ok(elements.first().cloned())
}

/// Serializes concurrent version assignment for the same ballot box (tenant, election_event,
/// election, area, contest, channel) by taking a transaction-scoped Postgres advisory lock.
/// The lock is automatically released when the transaction commits or rolls back, so a second
/// concurrent request for the same ballot box will block here until the first one finishes,
/// instead of racing to read the same "latest version" and then colliding on the unique index.
#[instrument(err, skip_all)]
pub async fn lock_ballot_box_version_assignment(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    contest_id: &str,
    channel: &VotingChannel,
) -> Result<()> {
    let lock_key =
        format!("{tenant_id}:{election_event_id}:{election_id}:{area_id}:{contest_id}:{channel}");
    hasura_transaction
        .query(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&lock_key],
        )
        .await?;
    Ok(())
}

/// Get the latest version of a ballot box, this is for the same area_id, contest_id and channel.
/// Returns 0 if no ballot box exists yet for the given paramenters
#[instrument(err, skip_all)]
pub async fn get_latest_ballot_box_version(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    contest_id: &str,
    channel: &VotingChannel,
) -> Result<i32> {
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
                    election_id = $3 AND
                    area_id = $4 AND
                    contest_id = $5 AND
                    channel = $6
                ORDER BY
                    version DESC
                LIMIT 1
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
                &parse_uuid_v4(area_id)?,
                &parse_uuid_v4(contest_id)?,
                &channel.to_string(),
            ],
        )
        .await?;

    let versions: Vec<i32> = rows
        .into_iter()
        .map(|row| -> Result<i32> {
            row.try_into()
                .map(|res: TallySheetWrapper| -> i32 { res.0.version })
        })
        .collect::<Result<Vec<i32>>>()?;

    match versions.len() {
        0 => Ok(0),
        1 => Ok(versions[0]),
        _ => Err(anyhow!("Multiple entries found but LIMIT should be 1.")),
    }
}

#[instrument(err, skip_all)]
pub async fn get_latest_ballot_box_tally_sheet(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    contest_id: &str,
    channel: &VotingChannel,
) -> Result<Option<TallySheet>> {
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
                    election_id = $3 AND
                    area_id = $4 AND
                    contest_id = $5 AND
                    channel = $6 AND
                    deleted_at IS NULL
                ORDER BY
                    version DESC
                LIMIT 1;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
                &parse_uuid_v4(area_id)?,
                &parse_uuid_v4(contest_id)?,
                &channel.to_string(),
            ],
        )
        .await?;

    let elements = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: TallySheetWrapper| res.0))
        .collect::<Result<Vec<TallySheet>>>()?;

    Ok(elements.first().cloned())
}

/// When a tally sheet is approved, the other versions must be soft deleted.
/// It will soft-delete all the sheets with the same area_id, contest_id and channel but different tally_sheet_id
/// than the tally_sheet passed as a parameter.
#[instrument(skip(hasura_transaction), err)]
pub async fn soft_delete_tally_sheet_leftover_versions(
    hasura_transaction: &Transaction<'_>,
    tally_sheet: &TallySheet,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
        UPDATE sequent_backend.tally_sheet tally_sheet
        SET
            deleted_at = NOW(),
            last_updated_at = NOW()
        WHERE
            tally_sheet.tenant_id = $1 AND
            tally_sheet.election_event_id = $2 AND
            tally_sheet.election_id = $3 AND
            tally_sheet.area_id = $4 AND
            tally_sheet.contest_id = $5 AND
            tally_sheet.channel = $6 AND
            tally_sheet.id != $7 AND
            tally_sheet.deleted_at IS NULL
    "#,
        )
        .await?;

    let tenant_uuid: uuid::Uuid = parse_uuid_v4(tally_sheet.tenant_id.as_str())
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(tally_sheet.election_event_id.as_str())
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = parse_uuid_v4(tally_sheet.election_id.as_str())
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid = parse_uuid_v4(tally_sheet.area_id.as_str())
        .map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;
    let contest_uuid: uuid::Uuid = parse_uuid_v4(tally_sheet.contest_id.as_str())
        .map_err(|err| anyhow!("Error parsing contest_id as UUID: {}", err))?;
    let channel_str = tally_sheet
        .channel
        .as_deref()
        .ok_or(anyhow!("Channel is None"))?;
    let tally_sheet_uuid: uuid::Uuid = parse_uuid_v4(tally_sheet.id.as_str())
        .map_err(|err| anyhow!("Error parsing tally_sheet_id as UUID: {}", err))?;
    let params: Vec<&(dyn ToSql + Sync)> = vec![
        &tenant_uuid,
        &election_event_uuid,
        &election_uuid,
        &area_uuid,
        &contest_uuid,
        &channel_str,
        &tally_sheet_uuid,
    ];
    hasura_transaction
        .execute(&statement, params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    Ok(())
}

/// Outcome of attempting to review a tally sheet, distinguishing a sheet
/// that doesn't exist from one that exists but is no longer reviewable
/// (e.g. already approved/disapproved) so callers can map each case to the
/// right HTTP status instead of a blanket 404.
pub enum ReviewTallySheetOutcome {
    Reviewed(TallySheet),
    NotPending(TallySheet),
    NotFound,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn review_tally_sheet_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_sheet_id: &str,
    user_id: &str,
    status: TallySheetStatus,
) -> Result<ReviewTallySheetOutcome> {
    let statement = hasura_transaction
        .prepare(
            r#"
        UPDATE sequent_backend.tally_sheet tally_sheet
        SET
            status = $4,
            reviewed_at = NOW(),
            reviewed_by_user_id = $5,
            last_updated_at = NOW()
        WHERE
            tally_sheet.tenant_id = $1 AND
            tally_sheet.election_event_id = $2 AND
            tally_sheet.id = $3 AND
            tally_sheet.status = 'PENDING' AND
            tally_sheet.deleted_at IS NULL
        RETURNING *
    "#,
        )
        .await?;

    let tenant_uuid: uuid::Uuid = parse_uuid_v4(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let tally_sheet_uuid: uuid::Uuid = parse_uuid_v4(tally_sheet_id)
        .map_err(|err| anyhow!("Error parsing tally_sheet_id as UUID: {}", err))?;
    let status_str = status.to_string();
    let params: Vec<&(dyn ToSql + Sync)> = vec![
        &tenant_uuid,
        &election_event_uuid,
        &tally_sheet_uuid,
        &status_str,
        &user_id,
    ];
    let rows: Vec<Row> = hasura_transaction
        .query(&statement, params.as_slice())
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
        1 => return Ok(ReviewTallySheetOutcome::Reviewed(elements[0].clone())),
        0 => {}
        _ => return Err(anyhow!("Unexpected rows affected {}", elements.len())),
    }

    // No row was updated because it doesn't exist, or it exists but isn't
    // `PENDING` anymore. Look it up (ignoring status) to tell those apart.
    let existing_statement = hasura_transaction
        .prepare(
            r#"
        SELECT * FROM sequent_backend.tally_sheet tally_sheet
        WHERE
            tally_sheet.tenant_id = $1 AND
            tally_sheet.election_event_id = $2 AND
            tally_sheet.id = $3 AND
            tally_sheet.deleted_at IS NULL
    "#,
        )
        .await?;
    let existing_params: Vec<&(dyn ToSql + Sync)> =
        vec![&tenant_uuid, &election_event_uuid, &tally_sheet_uuid];
    let existing_rows: Vec<Row> = hasura_transaction
        .query(&existing_statement, existing_params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    match existing_rows.into_iter().next() {
        Some(row) => {
            let existing: TallySheet = TallySheetWrapper::try_from(row)?.0;
            Ok(ReviewTallySheetOutcome::NotPending(existing))
        }
        None => Ok(ReviewTallySheetOutcome::NotFound),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Preserve the existing database API argument order for transaction, record scope and column values."
)]
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
    import_id: Option<&str>,
) -> Result<TallySheet> {
    let tenant_uuid: uuid::Uuid = parse_uuid_v4(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = parse_uuid_v4(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let contest_uuid: uuid::Uuid = parse_uuid_v4(contest_id)
        .map_err(|err| anyhow!("Error parsing contest_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid =
        parse_uuid_v4(area_id).map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;
    let content_value = serde_json::to_value(content)?;
    let channel_str = channel.to_string();
    let status_str = status.to_string();
    let import_uuid = import_id.map(parse_uuid_v4).transpose()?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.tally_sheet
                (tenant_id, election_event_id, election_id, contest_id, area_id, created_at, last_updated_at, content, channel, created_by_user_id, status, version, import_id)
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
                    $10,
                    $11
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
                &import_uuid,
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
