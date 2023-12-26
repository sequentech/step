// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use anyhow::Result;
use sequent_core::ballot::*;
use sequent_core::services::keycloak::get_client_credentials;
use serde_json::value::Value;
use tracing::instrument;
use deadpool_postgres::Transaction;
use tokio_postgres::row::Row;

pub fn get_election_event_statistics(
    statistics_json_opt: Option<Value>,
) -> Option<ElectionEventStatistics> {
    statistics_json_opt.and_then(|statistics_json| serde_json::from_value(statistics_json).ok())
}

#[instrument(err)]
pub async fn update_election_event_statistics(
    tenant_id: String,
    election_event_id: String,
    statistics: ElectionEventStatistics,
) -> Result<()> {
    let auth_headers = get_client_credentials().await?;
    let statistics_json = serde_json::to_value(&statistics)?;

    hasura::election_event::update_election_event_statistics(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        statistics_json,
    )
    .await?;

    Ok(())
}

#[instrument(skip(transaction))]
pub async fn get_count_distinct_voters(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str
) -> Result<i64> {
    let total_distinct_voters_statement = transaction
        .prepare(
            r#"
            SELECT DISTINCT ON (election_id, voter_id_string)
                COUNT(*) AS total_distinct_voters
            FROM sequent_backend.cast_vote
            WHERE
                tenant_id = $1 AND
                election_event_id = $2
            ORDER BY election_id, voter_id_string, created_at DESC
        "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_distinct_voters_statement,
            &[
                &election_event_id,
                &tenant_id,
            ]
        ).await?;

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let total_distinct_voters: i64 = if rows.len() == 0 {
        0
    } else {
        rows[0]
            .try_get::<&str, i64>("total_distinct_voters")?
    };

    Ok(total_distinct_voters)
}
