// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CastVote {
    pub id: String,
    pub tenant_id: String,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_updated_at: Option<DateTime<Utc>>,
    pub content: Option<String>,
    pub cast_ballot_signature: Option<Vec<u8>>,
    pub voter_id_string: Option<String>,
    pub election_event_id: String,
}

impl TryFrom<Row> for CastVote {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(CastVote {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_id: item
                .try_get::<_, Option<Uuid>>("election_id")?
                .map(|val| val.to_string()),
            area_id: item
                .try_get::<_, Option<Uuid>>("area_id")?
                .map(|val| val.to_string()),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            content: item.try_get("content")?,
            cast_ballot_signature: item.try_get("cast_ballot_signature")?,
            voter_id_string: item.try_get("voter_id_string")?,
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
        })
    }
}

#[instrument(err)]
pub async fn find_area_ballots(
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
) -> Result<Vec<CastVote>> {
    let hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura db client")?;

    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid = Uuid::parse_str(area_id)
        .map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;
    let areas_statement = hasura_db_client
        .prepare(
            r#"
                    SELECT DISTINCT ON (election_id, voter_id_string)
                        id,
                        tenant_id,
                        election_id,
                        area_id,
                        created_at,
                        last_updated_at,
                        content,
                        cast_ballot_signature,
                        voter_id_string,
                        election_event_id
                    FROM "sequent_backend".cast_vote
                    WHERE
                        tenant_id = $1 AND
                        election_event_id = $2 AND
                        area_id = $3
                    ORDER BY election_id, voter_id_string, created_at DESC
                "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_db_client
        .query(
            &areas_statement,
            &[&tenant_uuid, &election_event_uuid, &area_uuid],
        )
        .await
        .map_err(|err| anyhow!("Error running the areas query: {}", err))?;
    let cast_votes = rows
        .into_iter()
        .map(|row| -> Result<CastVote> { row.try_into() })
        .collect::<Result<Vec<CastVote>>>()?;

    Ok(cast_votes)
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CastVotesPerDay {
    pub day: String,
    pub day_count: i64,
}

impl TryFrom<Row> for CastVotesPerDay {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(CastVotesPerDay {
            day: item.try_get::<_, chrono::NaiveDate>("day")?.to_string(),
            day_count: item.try_get::<_, i64>("day_count")?,
        })
    }
}

#[instrument(skip(transaction), err)]
pub async fn get_count_votes_per_day(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    start_date: &str,
    end_date: &str,
    election_id: Option<String>,
) -> Result<Vec<CastVotesPerDay>> {
    let start_date_naive = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .with_context(|| "Error parsing start_date")?;
    let end_date_naive = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .with_context(|| "Error parsing end_date")?;
    let total_areas_statement = transaction
        .prepare(
            r#"
            WITH date_series AS (
                SELECT
                    t.day::date 
                FROM 
                    generate_series(
                        $3::date,
                        $4::date,
                        interval  '1 day'
                    ) AS t(day)
            )
            SELECT
                ds.day,
                COALESCE(COUNT(v.created_at), 0) AS day_count
            FROM
                date_series ds
            LEFT JOIN sequent_backend.cast_vote v ON ds.day = DATE(v.created_at)
                AND v.tenant_id = $1
                AND v.election_event_id = $2
            WHERE
                (
                    DATE(v.created_at) >= $3 AND
                    DATE(v.created_at) <= $4
                )
                OR v.created_at IS NULL
            GROUP BY ds.day
            ORDER BY ds.day;
            
            "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_areas_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &start_date_naive,
                &end_date_naive,
            ],
        )
        .await?;
    let cast_votes_by_day = rows
        .into_iter()
        .map(|row| -> Result<CastVotesPerDay> { row.try_into() })
        .collect::<Result<Vec<CastVotesPerDay>>>()?;

    Ok(cast_votes_by_day)
}
