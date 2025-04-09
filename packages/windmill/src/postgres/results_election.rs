// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use ordered_float::NotNan;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::results::ResultDocuments;
use sequent_core::types::results::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::row::Row;
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
            total_voters_percent: item
                .try_get::<&str, Decimal>("total_voters_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            documents,
        }))
    }
}

#[instrument(skip_all, err)]
pub async fn update_results_election_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    election_id: &str,
    documents: &ResultDocuments,
    json_hash: &str,
) -> Result<()> {
    let documents_value = serde_json::to_value(documents.clone())?;
    let json_hash_value = serde_json::Value::String(json_hash.to_string()); // Convert json_hash to JSON
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
                    documents = $1,
                    annotations = jsonb_set(COALESCE(annotations, '{}'), '{results_hash}', $2)
                WHERE
                    tenant_id = $3 AND
                    results_event_id = $4 AND
                    election_event_id = $5 AND
                    election_id = $6
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
                &json_hash_value,
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

#[instrument(err, skip(hasura_transaction, elections))]
pub async fn insert_results_elections(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
    elections: Vec<ResultsElection>,
) -> Result<Vec<ResultsElection>> {
    if elections.is_empty() {
        return Ok(Vec::new());
    }
    #[derive(Debug, Serialize)]
    pub struct InsertResultsElection {
        pub tenant_id: Uuid,
        pub election_event_id: Uuid,
        pub results_event_id: Uuid,
        pub election_id: Uuid,
        pub name: Option<String>,
        pub elegible_census: Option<i64>,
        pub total_voters: Option<i64>,
        pub total_voters_percent: Option<f64>,
    }

    let tenant_uuid = Uuid::parse_str(tenant_id)?;
    let election_event_uuid = Uuid::parse_str(election_event_id)?;
    let results_event_uuid = Uuid::parse_str(results_event_id)?;

    let insert_data: Vec<InsertResultsElection> = elections
        .iter()
        .map(|election| {
            Ok(InsertResultsElection {
                tenant_id: tenant_uuid,
                election_event_id: election_event_uuid,
                results_event_id: results_event_uuid,
                election_id: Uuid::parse_str(&election.election_id)?,
                name: election.name.clone(),
                elegible_census: election.elegible_census,
                total_voters: election.total_voters,
                total_voters_percent: election.total_voters_percent.clone().map(|n| n.into()),
            })
        })
        .collect::<Result<Vec<InsertResultsElection>>>()?;

    let json_data = serde_json::to_value(&insert_data)?;

    // Construct the base SQL query
    let sql: &str = "WITH data AS (
            SELECT * FROM jsonb_to_recordset($1::jsonb) AS t(
                tenant_id UUID,
                election_event_id UUID,
                results_event_id UUID,
                election_id UUID,
                name TEXT,
                elegible_census BIGINT,
                total_voters BIGINT,
                total_voters_percent FLOAT8
            )
        )
        INSERT INTO sequent_backend.results_election (
            tenant_id, election_event_id, results_event_id, election_id, name, elegible_census, total_voters, total_voters_percent
        )
        SELECT
            tenant_id, election_event_id, results_event_id, election_id, name, elegible_census, total_voters, total_voters_percent
        FROM data
        RETURNING *;";

    info!("SQL statement: {}", sql);

    let statement = hasura_transaction.prepare(sql).await?;
    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&json_data])
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

#[instrument(err)]
pub async fn get_results_election_by_results_event_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_id: &str,
    results_event_id: &str,
) -> Result<ResultsElection> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let results_event_uuid: uuid::Uuid = Uuid::parse_str(results_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM 
                    sequent_backend.results_election
                WHERE
                    tenant_id = $1 AND
                    election_id = $2 AND
                    results_event_id = $3
                ORDER BY created_at DESC
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &election_uuid, &results_event_uuid],
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

    results
        .get(0)
        .map(|val| val.clone())
        .ok_or(anyhow!("Results election not found"))
}

#[instrument(err)]
pub async fn get_event_results_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ResultsElection>> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM 
                    sequent_backend.results_election
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(&statement, &[&tenant_uuid, &election_event_uuid])
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
