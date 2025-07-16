// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
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

pub struct ResultsContestCandidateWrapper(pub ResultsContestCandidate);

impl TryFrom<Row> for ResultsContestCandidateWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let documents_value: Option<Value> = item.try_get("documents")?;
        let documents: Option<ResultDocuments> = documents_value
            .map(|value| deserialize_value(value))
            .transpose()?;

        Ok(ResultsContestCandidateWrapper(ResultsContestCandidate {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            contest_id: item.try_get::<_, Uuid>("contest_id")?.to_string(),
            candidate_id: item.try_get::<_, Uuid>("candidate_id")?.to_string(),
            results_event_id: item.try_get::<_, Uuid>("results_event_id")?.to_string(),
            cast_votes: item
                .try_get::<_, Option<i32>>("cast_votes")?
                .map(|val| val as i64),
            winning_position: item
                .try_get::<_, Option<i32>>("winning_position")?
                .map(|val| val as i64),
            points: item
                .try_get::<_, Option<i32>>("points")?
                .map(|val| val as i64),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            cast_votes_percent: item
                .try_get::<_, Decimal>("cast_votes_percent")?
                .to_f64()
                .map(NotNan::new)
                .transpose()?,
            documents,
        }))
    }
}

#[instrument(err, skip(hasura_transaction, contest_candidates))]
pub async fn insert_results_contest_candidates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
    contest_candidates: Vec<ResultsContestCandidate>,
) -> Result<Vec<ResultsContestCandidate>> {
    if contest_candidates.is_empty() {
        return Ok(Vec::new());
    }
    #[derive(Debug, Serialize)]
    pub struct InsertResultsContestCandidate {
        pub tenant_id: Uuid,
        pub election_event_id: Uuid,
        pub results_event_id: Uuid,
        pub election_id: Uuid,
        pub contest_id: Uuid,
        pub candidate_id: Uuid,
        pub cast_votes: Option<i64>,
        pub winning_position: Option<i64>,
        pub points: Option<i64>,
        pub cast_votes_percent: Option<f64>,
    }

    let tenant_uuid = Uuid::parse_str(tenant_id)?;
    let election_event_uuid = Uuid::parse_str(election_event_id)?;
    let results_event_uuid = Uuid::parse_str(results_event_id)?;

    let insert_data: Vec<InsertResultsContestCandidate> = contest_candidates
        .iter()
        .map(|contest_candidate| {
            Ok(InsertResultsContestCandidate {
                tenant_id: tenant_uuid,
                election_event_id: election_event_uuid,
                results_event_id: results_event_uuid,
                election_id: Uuid::parse_str(&contest_candidate.election_id)?,
                contest_id: Uuid::parse_str(&contest_candidate.contest_id)?,
                candidate_id: Uuid::parse_str(&contest_candidate.candidate_id)?,
                cast_votes: contest_candidate.cast_votes,
                winning_position: contest_candidate.winning_position,
                points: contest_candidate.points,
                cast_votes_percent: contest_candidate
                    .cast_votes_percent
                    .clone()
                    .map(|n| n.into()),
            })
        })
        .collect::<Result<Vec<InsertResultsContestCandidate>>>()?;

    let json_data = serde_json::to_value(&insert_data)?;

    // Construct the base SQL query
    let sql: &str = "WITH data AS (
            SELECT * FROM jsonb_to_recordset($1::jsonb) AS t(
                tenant_id UUID,
                election_event_id UUID,
                results_event_id UUID,
                election_id UUID,
                contest_id UUID,
                candidate_id UUID,
                cast_votes BIGINT,
                winning_position BIGINT,
                points BIGINT,
                cast_votes_percent FLOAT8
            )
        )
        INSERT INTO sequent_backend.results_contest_candidate (
            tenant_id,
            election_event_id,
            results_event_id,
            election_id,
            contest_id,
            candidate_id,
            cast_votes,
            winning_position,
            points,
            cast_votes_percent
        )
        SELECT
            tenant_id,
            election_event_id,
            results_event_id,
            election_id,
            contest_id,
            candidate_id,
            cast_votes,
            winning_position,
            points,
            cast_votes_percent
        FROM data
        RETURNING *;";

    info!("SQL statement: {}", sql);

    let statement = hasura_transaction.prepare(sql).await?;
    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&json_data])
        .await
        .map_err(|err| anyhow!("Error inserting rows: {}", err))?;

    // Convert rows to ResultsElection instances
    let values: Vec<ResultsContestCandidate> = rows
        .into_iter()
        .map(|row| {
            row.try_into()
                .map(|res: ResultsContestCandidateWrapper| res.0)
        })
        .collect::<Result<Vec<ResultsContestCandidate>>>()?;

    Ok(values)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_event_results_contest_candidates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ResultsContestCandidate>> {
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
                    sequent_backend.results_contest_candidate
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &election_event_uuid])
        .await
        .map_err(|err| anyhow!("Error running the areas query: {}", err))?;

    let results_contest_candidate: Vec<ResultsContestCandidate> = rows
        .into_iter()
        .map(|row| -> Result<ResultsContestCandidate> {
            row.try_into()
                .map(|res: ResultsContestCandidateWrapper| -> ResultsContestCandidate { res.0 })
        })
        .collect::<Result<Vec<ResultsContestCandidate>>>()?;

    Ok(results_contest_candidate)
}

#[derive(Debug, Serialize)]
struct InsertableResultsContestCandidate {
    id: Uuid,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    contest_id: Uuid,
    candidate_id: Uuid,
    results_event_id: Uuid,
    cast_votes: Option<i64>,
    winning_position: Option<i64>,
    points: Option<i64>,
    created_at: Option<DateTime<Local>>,
    last_updated_at: Option<DateTime<Local>>,
    labels: Option<Value>,
    annotations: Option<Value>,
    cast_votes_percent: Option<f64>,
    documents: Option<Value>,
}

#[instrument(err, skip(hasura_transaction, candidates))]
pub async fn insert_many_results_contest_candidates(
    hasura_transaction: &Transaction<'_>,
    candidates: Vec<ResultsContestCandidate>,
) -> Result<Vec<ResultsContestCandidate>> {
    if candidates.is_empty() {
        return Ok(vec![]);
    }

    let insertable: Vec<InsertableResultsContestCandidate> = candidates
        .into_iter()
        .map(|c| {
            let documents_json = c.documents.map(|d| serde_json::to_value(&d)).transpose()?;

            Ok(InsertableResultsContestCandidate {
                id: Uuid::parse_str(&c.id)?,
                tenant_id: Uuid::parse_str(&c.tenant_id)?,
                election_event_id: Uuid::parse_str(&c.election_event_id)?,
                election_id: Uuid::parse_str(&c.election_id)?,
                contest_id: Uuid::parse_str(&c.contest_id)?,
                candidate_id: Uuid::parse_str(&c.candidate_id)?,
                results_event_id: Uuid::parse_str(&c.results_event_id)?,
                cast_votes: c.cast_votes,
                winning_position: c.winning_position,
                points: c.points,
                created_at: c.created_at,
                last_updated_at: c.last_updated_at,
                labels: c.labels.clone(),
                annotations: c.annotations.clone(),
                cast_votes_percent: c.cast_votes_percent.map(|v| v.into_inner()),
                documents: documents_json,
            })
        })
        .collect::<Result<_>>()?;

    let json_data = serde_json::to_value(&insertable)?;

    let sql = r#"
        WITH data AS (
            SELECT * FROM jsonb_to_recordset($1::jsonb) AS t(
                id UUID,
                tenant_id UUID,
                election_event_id UUID,
                election_id UUID,
                contest_id UUID,
                candidate_id UUID,
                results_event_id UUID,
                cast_votes BIGINT,
                winning_position BIGINT,
                points BIGINT,
                created_at TIMESTAMPTZ,
                last_updated_at TIMESTAMPTZ,
                labels JSONB,
                annotations JSONB,
                cast_votes_percent FLOAT8,
                documents JSONB
            )
        )
        INSERT INTO sequent_backend.results_contest_candidate (
            id, tenant_id, election_event_id, election_id, contest_id,
            candidate_id, results_event_id, cast_votes, winning_position, points,
            created_at, last_updated_at, labels, annotations,
            cast_votes_percent, documents
        )
        SELECT
            *
        FROM data
        RETURNING *;
    "#;

    let statement = hasura_transaction.prepare(sql).await?;
    let rows = hasura_transaction.query(&statement, &[&json_data]).await?;

    let inserted = rows
        .into_iter()
        .map(|row| {
            let wrapper: ResultsContestCandidateWrapper = row.try_into()?;
            Ok(wrapper.0)
        })
        .collect::<Result<Vec<ResultsContestCandidate>>>()?;

    Ok(inserted)
}
