// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use std::convert::From;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
struct CastVote {
    pub id: String,
    pub tenant_id: String,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub created_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub content: Option<String>,
    pub cast_ballot_signature: Option<String>,
    pub voter_id_string: Option<String>,
    pub election_event_i: String,
}

#[instrument(err)]
pub async fn find_area_ballots(
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
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
    let users = rows
        .into_iter()
        .map(|row| -> Result<CastVote> { row.try_into() })
        .collect::<Result<Vec<CastVote>>>()?;

    Ok(())
}
