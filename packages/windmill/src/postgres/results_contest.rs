// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use ordered_float::NotNan;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::results::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::{info, instrument};
use uuid::Uuid;

pub struct ResultsContestWrapper(pub ResultsContest);

impl TryFrom<Row> for ResultsContestWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let documents_value: Option<Value> = item.try_get("documents")?;
        let documents: Option<ResultDocuments> = documents_value
            .map(|value| deserialize_value(value))
            .transpose()?;

        Ok(ResultsContestWrapper(ResultsContest {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            contest_id: item.try_get::<_, Uuid>("contest_id")?.to_string(),
            results_event_id: item.try_get::<_, Uuid>("results_event_id")?.to_string(),
            elegible_census: item
                .try_get::<_, Option<i32>>("elegible_census")?
                .map(|val| val as i64),
            total_valid_votes: item
                .try_get::<_, Option<i32>>("total_valid_votes")?
                .map(|val| val as i64),
            total_auditable_votes_percent: item
                .try_get::<_, Decimal>("total_auditable_votes_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            explicit_invalid_votes: item
                .try_get::<_, Option<i32>>("explicit_invalid_votes")?
                .map(|val| val as i64),
            implicit_invalid_votes: item
                .try_get::<_, Option<i32>>("implicit_invalid_votes")?
                .map(|val| val as i64),
            blank_votes: item
                .try_get::<_, Option<i32>>("blank_votes")?
                .map(|val| val as i64),
            voting_type: item.try_get("voting_type")?,
            counting_algorithm: item.try_get("counting_algorithm")?,
            name: item.try_get("name")?,
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            total_invalid_votes: item
                .try_get::<_, Option<i32>>("total_invalid_votes")?
                .map(|val| val as i64),
            total_invalid_votes_percent: item
                .try_get::<&str, Decimal>("total_invalid_votes_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            total_valid_votes_percent: item
                .try_get::<&str, Decimal>("total_valid_votes_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            explicit_invalid_votes_percent: item
                .try_get::<&str, Decimal>("explicit_invalid_votes_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            implicit_invalid_votes_percent: item
                .try_get::<&str, Decimal>("implicit_invalid_votes_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            blank_votes_percent: item
                .try_get::<&str, Decimal>("blank_votes_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            total_votes: item
                .try_get::<_, Option<i32>>("total_votes")?
                .map(|val| val as i64),
            total_votes_percent: item
                .try_get::<&str, Decimal>("total_votes_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            documents,
            total_auditable_votes: item
                .try_get::<_, Option<i32>>("total_auditable_votes")?
                .map(|val| val as i64),
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_results_contest_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    let documents_value = serde_json::to_value(documents.clone())?;
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let results_event_uuid: uuid::Uuid = Uuid::parse_str(&results_event_id)
        .map_err(|err| anyhow!("Error parsing results_event_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let contest_uuid: uuid::Uuid = Uuid::parse_str(&contest_id)
        .map_err(|err| anyhow!("Error parsing contest_id as UUID: {}", err))?;
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.results_contest
                SET
                    documents = $1
                WHERE
                    tenant_id = $2 AND
                    results_event_id = $3 AND
                    election_event_id = $4 AND
                    election_id = $5 AND
                    contest_id = $6
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
                &contest_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the areas query: {}", err))?;

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

#[instrument(skip(hasura_transaction), err)]
pub async fn get_results_contest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
) -> Result<ResultsContest> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let contest_uuid: uuid::Uuid = Uuid::parse_str(&contest_id)
        .map_err(|err| anyhow!("Error parsing contest_id as UUID: {}", err))?;
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.results_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    contest_id = $4
                RETURNING
                    id;
            "#,
        )
        .await?;
    let row: Option<Row> = hasura_transaction
        .query_opt(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &election_uuid,
                &contest_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the query: {}", err))?;

    if let Some(row) = row {
        let results_contest: ResultsContest = row
            .try_into()
            .map(|res: ResultsContestWrapper| -> ResultsContest { res.0 })?;
        Ok(results_contest)
    } else {
        Err(anyhow::anyhow!(
            "No results contest found with the provided data"
        ))
    }
}

#[instrument(err, skip(hasura_transaction, contests))]
pub async fn insert_results_contests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
    contests: Vec<ResultsContest>,
) -> Result<Vec<ResultsContest>> {
    if contests.is_empty() {
        return Ok(Vec::new());
    }

    #[derive(Serialize)]
    struct InsertContestData {
        tenant_id: Uuid,
        election_event_id: Uuid,
        election_id: Uuid,
        contest_id: Uuid,
        results_event_id: Uuid,
        elegible_census: Option<i64>,
        total_votes: Option<i64>,
        total_votes_percent: Option<f64>,
        total_auditable_votes: Option<i64>,
        total_auditable_votes_percent: Option<f64>,
        total_valid_votes: Option<i64>,
        total_valid_votes_percent: Option<f64>,
        total_invalid_votes: Option<i64>,
        total_invalid_votes_percent: Option<f64>,
        explicit_invalid_votes: Option<i64>,
        explicit_invalid_votes_percent: Option<f64>,
        implicit_invalid_votes: Option<i64>,
        implicit_invalid_votes_percent: Option<f64>,
        blank_votes: Option<i64>,
        blank_votes_percent: Option<f64>,
        voting_type: Option<String>,
        counting_algorithm: Option<String>,
        name: Option<String>,
        annotations: Option<serde_json::Value>,
    }

    let tenant_uuid = Uuid::parse_str(tenant_id)?;
    let election_event_uuid = Uuid::parse_str(election_event_id)?;
    let results_event_uuid = Uuid::parse_str(results_event_id)?;

    let insert_data: Vec<InsertContestData> = contests
        .iter()
        .map(|contest| {
            Ok(InsertContestData {
                tenant_id: tenant_uuid,
                election_event_id: election_event_uuid,
                election_id: Uuid::parse_str(&contest.election_id)?,
                contest_id: Uuid::parse_str(&contest.contest_id)?,
                results_event_id: results_event_uuid,
                elegible_census: contest.elegible_census,
                total_votes: contest.total_votes,
                total_votes_percent: contest.total_votes_percent.clone().map(|n| n.into()),
                total_auditable_votes: contest.total_auditable_votes,
                total_auditable_votes_percent: contest
                    .total_auditable_votes_percent
                    .clone()
                    .map(|n| n.into()),
                total_valid_votes: contest.total_valid_votes,
                total_valid_votes_percent: contest
                    .total_valid_votes_percent
                    .clone()
                    .map(|n| n.into()),
                total_invalid_votes: contest.total_invalid_votes,
                total_invalid_votes_percent: contest
                    .total_invalid_votes_percent
                    .clone()
                    .map(|n| n.into()),
                explicit_invalid_votes: contest.explicit_invalid_votes,
                explicit_invalid_votes_percent: contest
                    .explicit_invalid_votes_percent
                    .clone()
                    .map(|n| n.into()),
                implicit_invalid_votes: contest.implicit_invalid_votes,
                implicit_invalid_votes_percent: contest
                    .implicit_invalid_votes_percent
                    .clone()
                    .map(|n| n.into()),
                blank_votes: contest.blank_votes,
                blank_votes_percent: contest.blank_votes_percent.clone().map(|n| n.into()),
                voting_type: contest.voting_type.clone(),
                counting_algorithm: contest.counting_algorithm.clone(),
                name: contest.name.clone(),
                annotations: contest.annotations.clone(),
            })
        })
        .collect::<Result<Vec<InsertContestData>>>()?;

    let json_data = serde_json::to_value(&insert_data)?;

    // Construct the SQL query using jsonb_to_recordset
    let sql = r#"
        WITH data AS (
            SELECT * FROM jsonb_to_recordset($1::jsonb) AS t(
                tenant_id UUID,
                election_event_id UUID,
                election_id UUID,
                contest_id UUID,
                results_event_id UUID,
                elegible_census BIGINT,
                total_votes BIGINT,
                total_votes_percent FLOAT8,
                total_auditable_votes BIGINT,
                total_auditable_votes_percent FLOAT8,
                total_valid_votes BIGINT,
                total_valid_votes_percent FLOAT8,
                total_invalid_votes BIGINT,
                total_invalid_votes_percent FLOAT8,
                explicit_invalid_votes BIGINT,
                explicit_invalid_votes_percent FLOAT8,
                implicit_invalid_votes BIGINT,
                implicit_invalid_votes_percent FLOAT8,
                blank_votes BIGINT,
                blank_votes_percent FLOAT8,
                voting_type TEXT,
                counting_algorithm TEXT,
                name TEXT,
                annotations JSONB
            )
        )
        INSERT INTO sequent_backend.results_contest (
            tenant_id,
            election_event_id,
            election_id,
            contest_id,
            results_event_id,
            elegible_census,
            total_votes,
            total_votes_percent,
            total_auditable_votes,
            total_auditable_votes_percent,
            total_valid_votes,
            total_valid_votes_percent,
            total_invalid_votes,
            total_invalid_votes_percent,
            explicit_invalid_votes,
            explicit_invalid_votes_percent,
            implicit_invalid_votes,
            implicit_invalid_votes_percent,
            blank_votes,
            blank_votes_percent,
            voting_type,
            counting_algorithm,
            name,
            annotations
        )
        SELECT
            tenant_id,
            election_event_id,
            election_id,
            contest_id,
            results_event_id,
            elegible_census,
            total_votes,
            total_votes_percent,
            total_auditable_votes,
            total_auditable_votes_percent,
            total_valid_votes,
            total_valid_votes_percent,
            total_invalid_votes,
            total_invalid_votes_percent,
            explicit_invalid_votes,
            explicit_invalid_votes_percent,
            implicit_invalid_votes,
            implicit_invalid_votes_percent,
            blank_votes,
            blank_votes_percent,
            voting_type,
            counting_algorithm,
            name,
            annotations
        FROM data
        RETURNING *;
    "#;

    info!("SQL statement: {}", sql);

    let statement = hasura_transaction.prepare(sql).await?;
    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&json_data])
        .await
        .map_err(|err| anyhow!("Error inserting rows: {}", err))?;

    let values: Vec<ResultsContest> = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: ResultsContestWrapper| res.0))
        .collect::<Result<Vec<ResultsContest>>>()?;

    Ok(values)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_event_results_contest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ResultsContest>> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.results_contest
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

    let results = rows
        .into_iter()
        .map(|row| {
            row.try_into()
                .map(|res: ResultsContestWrapper| res.0)
                .map_err(|err| anyhow!("Error converting row to ResultsContest: {}", err))
        })
        .collect::<Result<Vec<ResultsContest>>>()?;

    Ok(results)
}
