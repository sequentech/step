// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
use windmill::{
    hasura::election::get_all_elections_for_event,
    postgres::election::get_elections, services::database::get_hasura_pool,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventMonitoringInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventMonitoringOutput {
    total_open_votes: i64,
    total_not_opened_votes: i64,
    total_closed_votes: i64,
    total_not_closed_votes: i64,
    total_transmitted_results: i64,
    total_not_transmitted_results: i64,
    total_genereated_er: i64,
    total_not_genereated_er: i64,
    total_start_counting_votes: i64,
    total_not_start_counting_votes: i64,
}

#[instrument(skip(claims))]
#[post("/election-event-monitoring", format = "json", data = "<body>")]
pub async fn get_election_event_monitoring_f(
    body: Json<ElectionEventMonitoringInput>,
    claims: JwtClaims,
) -> Result<Json<ElectionEventMonitoringOutput>, (Status, String)> {
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

    //     get_elections_

    // let total_distinct_voters: i64 = get_count_distinct_voters(
    //     &hasura_transaction,
    //     &tenant_id.as_str(),
    //     &input.election_event_id.as_str(),
    //     &input.election_id.as_str(),
    // )
    // .await
    // .map_err(|err| {
    //     (
    //         Status::InternalServerError,
    //         format!("Error retrieving total_distinct_voters: {err}"),
    //     )
    // })?;
    // let total_areas: i64 = get_count_areas(
    //     &hasura_transaction,
    //     &tenant_id.as_str(),
    //     &input.election_event_id.as_str(),
    //     &input.election_id.as_str(),
    // )
    // .await
    // .map_err(|err| {
    //     (
    //         Status::InternalServerError,
    //         format!("Error retrieving total_areas: {err}"),
    //     )
    // })?;

    // let votes_per_day: Vec<CastVotesPerDay> = get_count_votes_per_day(
    //     &hasura_transaction,
    //     &tenant_id.as_str(),
    //     &input.election_event_id.as_str(),
    //     &input.start_date.as_str(),
    //     &input.end_date.as_str(),
    //     Some(input.election_id),
    // )
    // .await
    // .map_err(|err| {
    //     (
    //         Status::InternalServerError,
    //         format!("Error retrieving votes_per_day: {err}"),
    //     )
    // })?;

    Ok(Json(ElectionEventMonitoringOutput {
        total_open_votes: 0,
        total_not_opened_votes: 0,
        total_closed_votes: 0,
        total_not_closed_votes: 0,
        total_transmitted_results: 0,
        total_not_transmitted_results: 0,
        total_genereated_er: 0,
        total_not_genereated_er: 0,
        total_start_counting_votes: 0,
        total_not_start_counting_votes: 0,
    }))
}
