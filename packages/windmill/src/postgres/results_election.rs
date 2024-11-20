// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::types::results::ResultDocuments;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsElection {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub results_event_id: String,
    pub name: Option<String>,
    pub elegible_census: Option<i64>,
    pub total_voters: Option<i64>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub documents: Option<Value>,
}
pub struct ResultsElectionWrapper(pub ResultsElection);
impl TryFrom<Row> for ResultsElectionWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(ResultsElectionWrapper(ResultsElection {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            results_event_id: item.try_get::<_, Uuid>("results_event_id")?.to_string(),
            name: item.try_get("name")?,
            elegible_census: item
                .try_get::<_, Option<i32>>("elegible_census")?
                .map(|v| v as i64),
            total_voters: item
                .try_get::<_, Option<i32>>("total_voters")?
                .map(|v| v as i64),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            documents: item.try_get("documents")?,
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_results_election_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    election_id: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    let documents_value = serde_json::to_value(documents.clone())?;
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let results_event_uuid: uuid::Uuid = Uuid::parse_str(&results_event_id)
        .map_err(|err| anyhow!("Error parsing results_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.results_election
                SET
                    documents = $1
                WHERE
                    tenant_id = $2 AND
                    results_event_id = $3 AND
                    election_event_id = $4 AND
                    election_id = $5
                RETURNING
                    id;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &documents_value,
                &tenant_uuid,
                &results_event_uuid,
                &election_event_uuid,
                &election_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the results_election query: {}", err))?;

    if 1 == rows.len() {
        Ok(())
    } else if rows.len() > 1 {
        Err(anyhow!(
            "Too many affected rows in table results_contest: {}",
            rows.len()
        ))
    } else {
        Err(anyhow!("Rows not found in table results_contest"))
    }
}

#[instrument(err)]
pub async fn get_election_results(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Vec<ResultsElection>> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM 
                    sequent_backend.results_election
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3
                ORDER BY created_at DESC
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &election_event_uuid, &election_uuid],
        )
        .await
        .map_err(|err| anyhow!("Error running the query: {}", err))?;

    // Convert rows into ResultsElection objects
    let results = rows
        .into_iter()
        .map(|row| {
            row.try_into()
                .map(|res: ResultsElectionWrapper| res.0)
                .map_err(|err| anyhow!("Error converting row to ResultsElection: {}", err))
        })
        .collect::<Result<Vec<ResultsElection>>>()?;

    Ok(results)
}
