// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::*;
use sequent_core::services::keycloak::get_client_credentials;
use serde_json::value::Value;
use std::default::Default;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub fn get_election_statistics(statistics_json_opt: Option<Value>) -> Option<ElectionStatistics> {
    statistics_json_opt.and_then(|statistics_json| serde_json::from_value(statistics_json).ok())
}

// #[instrument(err)]
// pub async fn update_election_statistics(
//     tenant_id: String,
//     election_event_id: String,
//     election_id: String,
//     statistics: ElectionStatistics,
// ) -> Result<()> {
//     let auth_headers = get_client_credentials().await?;
//
//     let statistics_json = serde_json::to_value(&statistics)?;
//
//     hasura::election::update_election_statistics(
//         auth_headers.clone(),
//         tenant_id.clone(),
//         election_event_id.clone(),
//         statistics_json,
//     )
//     .await?;
//
//     Ok(())
// }

#[instrument(skip(transaction), err)]
pub async fn get_count_distinct_voters(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<i64> {
    let total_distinct_voters_statement = transaction
        .prepare(
            r#"
            SELECT
                COUNT(DISTINCT voter_id_string) AS total_distinct_voters
            FROM
                sequent_backend.cast_vote
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                election_id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_distinct_voters_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
            ],
        )
        .await?;

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let total_distinct_voters: i64 = if rows.len() == 0 {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_distinct_voters")?
    };

    Ok(total_distinct_voters)
}

#[instrument(skip(transaction), err)]
pub async fn get_count_areas(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<i64> {
    let total_areas_statement = transaction
        .prepare(
            r#"
            SELECT
                count(DISTINCT a.id) as total_areas
            FROM
                sequent_backend.area a
            JOIN
                sequent_backend.area_contest ac ON
                    a.id = ac.area_id AND
                    a.election_event_id = ac.election_event_id AND
                    a.tenant_id = ac.tenant_id
            JOIN
                sequent_backend.contest c ON
                    ac.contest_id = c.id AND
                    ac.election_event_id = c.election_event_id AND
                    ac.tenant_id = c.tenant_id
            WHERE
                c.tenant_id = $1 AND
                c.election_event_id = $2 AND
                c.election_id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_areas_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
            ],
        )
        .await?;

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let total_areas: i64 = if rows.len() == 0 {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_areas")?
    };

    Ok(total_areas)
}
