// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ResultsAreaContestCandidate {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub contest_id: String,
    pub area_id: String,
    pub candidate_id: String,
    pub results_event_id: String,
    pub cast_votes: Option<i64>,
    pub winning_position: Option<i64>,
    pub points: Option<i64>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub cast_votes_percent: Option<i64>,
    pub documents: Option<Value>,
}
pub struct ResultsAreaContestCandidateWrapper(pub ResultsAreaContestCandidate);
impl TryFrom<Row> for ResultsAreaContestCandidateWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(ResultsAreaContestCandidateWrapper(
            ResultsAreaContestCandidate {
                id: item.try_get::<_, Uuid>("id")?.to_string(),
                tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
                election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
                annotations: item.try_get("annotations")?,
                election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
                contest_id: item.try_get::<_, Uuid>("contest_id")?.to_string(),
                area_id: item.try_get::<_, Uuid>("area_id")?.to_string(),
                candidate_id: item.try_get::<_, Uuid>("candidate_id")?.to_string(),
                results_event_id: item.try_get::<_, Uuid>("results_event_id")?.to_string(),
                cast_votes: item.try_get("cast_votes")?,
                winning_position: item.try_get("winning_position")?,
                points: item.try_get("points")?,
                created_at: item.try_get("created_at")?,
                last_updated_at: item.try_get("last_updated_at")?,
                labels: item.try_get("labels")?,
                cast_votes_percent: item.try_get("cast_votes_percent")?,
                documents: item.try_get("documents")?,
            },
        ))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_results_area_contest_candidates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    candidate_id: &str,
) -> Result<(Option<ResultsAreaContestCandidate>)> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let contest_uuid: uuid::Uuid = Uuid::parse_str(&contest_id)
        .map_err(|err| anyhow!("Error parsing contest_id as UUID: {}", err))?;
    let candidate_uuid: uuid::Uuid = Uuid::parse_str(&candidate_id)
        .map_err(|err| anyhow!("Error parsing candidate_id as UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.results_area_contest_candidate
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    contest_id = $4 AND
                    candidate_id = $5;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &election_uuid,
                &contest_uuid,
                &candidate_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the areas query: {}", err))?;

    let results_area_contest_candidate: Vec<ResultsAreaContestCandidate> = rows
        .into_iter()
        .map(|row| -> Result<ResultsAreaContestCandidate> {
            row.try_into().map(
                |res: ResultsAreaContestCandidateWrapper| -> ResultsAreaContestCandidate { res.0 },
            )
        })
        .collect::<Result<Vec<ResultsAreaContestCandidate>>>()?;

    Ok(results_area_contest_candidate.get(0).cloned())
}
