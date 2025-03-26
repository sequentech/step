// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use b3::messages::newtypes::BatchNumber;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::TallySessionContest;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct TallySessionContestWrapper(pub TallySessionContest);

impl TryFrom<Row> for TallySessionContestWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(TallySessionContestWrapper(TallySessionContest {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            area_id: item.try_get::<_, Uuid>("area_id")?.to_string(),
            contest_id: item
                .try_get::<_, Option<Uuid>>("contest_id")?
                .map(|val| val.to_string()),
            session_id: item.try_get("session_id")?,
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            tally_session_id: item.try_get::<_, Uuid>("tally_session_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_tally_session_contest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    contest_id: Option<String>,
    session_id: BatchNumber,
    tally_session_id: &str,
    election_id: &str,
) -> Result<TallySessionContest> {
    let contest_uuid = contest_id.map(|val| Uuid::parse_str(&val)).transpose()?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.tally_session_contest
                (tenant_id, election_event_id, area_id, contest_id, session_id, tally_session_id, election_id)
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
                &Uuid::parse_str(area_id)?,
                &contest_uuid,
                &(session_id as i32),
                &Uuid::parse_str(tally_session_id)?,
                &Uuid::parse_str(election_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting row: {}", err))?;

    let values: Vec<TallySessionContest> = rows
        .into_iter()
        .map(|row| -> Result<TallySessionContest> {
            row.try_into()
                .map(|res: TallySessionContestWrapper| -> TallySessionContest { res.0 })
        })
        .collect::<Result<Vec<TallySessionContest>>>()?;

    let Some(value) = values.first() else {
        return Err(anyhow!("Error inserting row"));
    };
    Ok(value.clone())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_tally_session_highest_batch(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<BatchNumber> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, session_id
                FROM
                    sequent_backend.tally_session_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2
                ORDER BY session_id DESC
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
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting row: {}", err))?;

    let values: Vec<BatchNumber> = rows
        .into_iter()
        .map(|row| -> Result<BatchNumber> {
            let session_id: i32 = row.try_get("session_id")?;
            Ok(session_id as BatchNumber)
        })
        .collect::<Result<Vec<BatchNumber>>>()?;

    let Some(value) = values.first() else {
        return Ok(0);
    };
    Ok(value + 1)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_tally_session_contests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<Vec<TallySessionContest>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, session_id
                FROM
                    sequent_backend.tally_session_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    tally_session_id = $3;
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
        .await
        .map_err(|err| anyhow!("Error getting tally session contests rows: {}", err))?;

    let values: Vec<TallySessionContest> = rows
        .into_iter()
        .map(|row| -> Result<TallySessionContest> {
            row.try_into()
                .map(|res: TallySessionContestWrapper| -> TallySessionContest { res.0 })
        })
        .collect::<Result<Vec<TallySessionContest>>>()?;
    Ok(values)
}
