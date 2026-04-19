// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::cast_votes::{
    get_count_votes_per_day, CastVotesPerDay,
};
use windmill::services::database::get_hasura_pool;
use windmill::services::election_statistics::get_count_areas;
use windmill::services::election_statistics::get_count_distinct_voters;

/// Request body for [`get_election_stats`].
#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionStatsInput {
    /// Election event UUID.
    election_event_id: String,
    /// Election UUID.
    election_id: String,
    /// Inclusive start of the date range (ISO-8601 date or datetime string).
    start_date: String,
    /// Inclusive end of the date range (ISO-8601 date or datetime string).
    end_date: String,
    /// User's timezone for date calculations.
    user_timezone: String,
}

/// Aggregated statistics for a single election.
#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionStatsOutput {
    /// Total number of distinct voters.
    total_distinct_voters: i64,
    /// Number of areas in the election.
    total_areas: i64,
    /// Vote counts grouped by calendar day.
    votes_per_day: Vec<CastVotesPerDay>,
}

/// Returns voter counts, area counts, and daily vote totals for one election.
#[instrument(skip(claims))]
#[post("/election/stats", format = "json", data = "<body>")]
pub async fn get_election_stats(
    body: Json<ElectionStatsInput>,
    claims: JwtClaims,
) -> Result<Json<ElectionStatsOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_DASHBOARD_VIEW],
    )?;
    let input = body.into_inner();
    let tenant_id: String = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error loading hasura db client: {err}"),
            )
        })?;
    let mut hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error creating a transaction: {err}"),
            )
        })?;

    let total_distinct_voters: i64 = get_count_distinct_voters(
        &hasura_transaction,
        tenant_id.as_str(),
        input.election_event_id.as_str(),
        input.election_id.as_str(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error retrieving total_distinct_voters: {err}"),
        )
    })?;
    let total_areas: i64 = get_count_areas(
        &hasura_transaction,
        tenant_id.as_str(),
        input.election_event_id.as_str(),
        input.election_id.as_str(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error retrieving total_areas: {err}"),
        )
    })?;

    let votes_per_day: Vec<CastVotesPerDay> = get_count_votes_per_day(
        &hasura_transaction,
        tenant_id.as_str(),
        input.election_event_id.as_str(),
        input.start_date.as_str(),
        input.end_date.as_str(),
        Some(input.election_id),
        input.user_timezone.as_str(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error retrieving votes_per_day: {err}"),
        )
    })?;

    Ok(Json(ElectionStatsOutput {
        total_distinct_voters,
        total_areas,
        votes_per_day,
    }))
}
