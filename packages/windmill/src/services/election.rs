// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::{User, VotesInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ElectionHead {
    pub id: String,
    pub name: String,
    pub alias: Option<String>,
}

impl TryFrom<Row> for ElectionHead {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(ElectionHead {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            name: item.get("name"),
            alias: item.get("alias"),
        })
    }
}

#[instrument(err)]
pub async fn get_election_event_elections(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ElectionHead>> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let elections_statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, name, alias
                FROM sequent_backend.election
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
            "#,
        )
        .await
        .with_context(|| "Error preparing election statement")?;
    let rows: Vec<Row> = hasura_transaction
        .query(&elections_statement, &[&tenant_uuid, &election_event_uuid])
        .await
        .with_context(|| "Error running the election query")?;
    let elections = rows
        .into_iter()
        .map(|row| -> Result<ElectionHead> { row.try_into() })
        .collect::<Result<Vec<ElectionHead>>>()
        .with_context(|| "Error collecting the elections")?;

    Ok(elections)
}
