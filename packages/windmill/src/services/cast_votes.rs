// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
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
pub struct CountData {
    pub count: i64,
}

impl TryFrom<Row> for CountData {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(CountData {
            count: item.get("count"),
        })
    }
}

#[instrument(err)]
pub async fn count_cast_votes_election(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<i64> {
    let hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura db client")?;

    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let areas_statement = hasura_db_client
        .prepare(
            r#"
                SELECT COUNT(DISTINCT voter_id_string)
                FROM sequent_backend.cast_vote
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_db_client
        .query(
            &areas_statement,
            &[&tenant_uuid, &election_event_uuid, &election_uuid],
        )
        .await
        .map_err(|err| anyhow!("Error running the areas query: {}", err))?;
    let count_data = rows
        .into_iter()
        .map(|row| -> Result<CountData> { row.try_into() })
        .collect::<Result<Vec<CountData>>>()?;

    Ok(count_data[0].count)
}
