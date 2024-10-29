// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::results::*;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
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
            cast_votes: item.try_get("cast_votes")?,
            winning_position: item.try_get("winning_position")?,
            points: item.try_get("points")?,
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            cast_votes_percent: item
                .try_get::<&str, Option<f64>>("cast_votes_percent")?
                .map(|val| val.try_into())
                .transpose()?,
            documents,
        }))
    }
}
