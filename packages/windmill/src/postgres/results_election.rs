// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::results::*;
use chrono::{DateTime, Local};
use sequent_core::types::results::ResultDocuments;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{info, instrument};
use uuid::Uuid;

pub struct ResultsElectionWrapper(pub ResultsElection);

impl TryFrom<Row> for ResultsElectionWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let documents_value: Option<Value> = item.try_get("documents")?;
        let documents: Option<ResultDocuments> = documents_value
            .map(|value| deserialize_value(value))
            .transpose()?;

        // Convert Option<f64> to Option<NotNan<f64>>
        let total_voters_percent = item
            .try_get::<&str, Option<f64>>("total_voters_percent")?
            .map(|val| val.try_into())
            .transpose()?;

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
            total_voters_percent,
            documents,
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

#[derive(Debug, Clone)]
pub struct InsertResultsElection {
    pub election_id: String,
    pub name: Option<String>,
    pub elegible_census: Option<i64>,
    pub total_voters: Option<i64>,
    pub total_voters_percent: Option<f64>,
}

#[instrument(err, skip(hasura_transaction))]
pub async fn insert_results_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
    elections: Vec<InsertResultsElection>,
) -> Result<Vec<ResultsElection>> {
    if elections.is_empty() {
        return Ok(Vec::new());
    }

    // Construct the base SQL query
    let mut sql = String::from(
        "INSERT INTO sequent_backend.results_election
        (tenant_id, election_event_id, results_event_id, election_id, name, elegible_census, total_voters, total_voters_percent)
        VALUES ",
    );

    let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();
    let mut placeholders = Vec::new();

    let tenant_uuid = Uuid::parse_str(tenant_id)?;
    let election_event_uuid = Uuid::parse_str(election_event_id)?;
    let results_event_uuid = Uuid::parse_str(results_event_id)?;

    // Create a vector to hold election UUIDs
    // Parse all election UUIDs beforehand to avoid mutable and immutable borrow conflicts
    let election_uuids: Vec<Uuid> = elections
        .iter()
        .map(|election| Uuid::parse_str(&election.election_id).context("Error parsing election id"))
        .collect::<Result<Vec<Uuid>>>()?;

    for (i, election) in elections.iter().enumerate() {
        let election_uuid_ref = &election_uuids[i];

        let param_offset = i * 8;
        let placeholder = format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            param_offset + 1,
            param_offset + 2,
            param_offset + 3,
            param_offset + 4,
            param_offset + 5,
            param_offset + 6,
            param_offset + 7,
            param_offset + 8
        );
        placeholders.push(placeholder);

        params.push(&tenant_uuid);
        params.push(&election_event_uuid);
        params.push(&results_event_uuid);
        params.push(election_uuid_ref);
        params.push(&election.name);
        params.push(&election.elegible_census);
        params.push(&election.total_voters);
        params.push(&election.total_voters_percent);
    }

    // Combine placeholders into the SQL query
    sql.push_str(&placeholders.join(", "));
    sql.push_str(" RETURNING *;");

    info!("SQL statement: {}", sql);
    // Prepare and execute the statement
    let statement = hasura_transaction.prepare(&sql).await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &params)
        .await
        .map_err(|err| anyhow!("Error inserting rows: {}", err))?;

    // Convert rows to ResultsElection instances
    let values: Vec<ResultsElection> = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: ResultsElectionWrapper| res.0))
        .collect::<Result<Vec<ResultsElection>>>()?;

    Ok(values)
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
