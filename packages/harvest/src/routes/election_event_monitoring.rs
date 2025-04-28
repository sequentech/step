// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::{jwt::JwtClaims, keycloak::get_event_realm};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_keycloak_pool;
use windmill::services::elections_monitoring::{
    MonitoringApproval, MonitoringAuthentication, MonitoringTransmissionStatus,
    MonitoringVotingStatus,
};
use windmill::services::users::count_keycloak_enabled_users;
use windmill::services::{
    database::get_hasura_pool,
    elections_monitoring::{
        get_election_event_monitoring, ElectionEventMonitoring,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventMonitoringInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventMonitoringOutput {
    total_eligible_voters: i64,
    total_enrolled_voters: i64,
    total_elections: i64,
    total_started_votes: i64,
    total_open_votes: i64,
    total_not_open_votes: i64,
    total_not_started_votes: i64,
    total_closed_votes: i64,
    total_not_closed_votes: i64,
    total_start_counting_votes: i64,
    total_not_start_counting_votes: i64,
    total_initialize: i64,
    total_not_initialize: i64,
    total_genereated_tally: i64,
    total_not_genereated_tally: i64,
    authentication_stats: MonitoringAuthentication,
    voting_stats: MonitoringVotingStatus,
    approval_stats: MonitoringApproval,
    transmission_stats: MonitoringTransmissionStatus,
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
        vec![Permissions::MONITORING_DASHBOARD_VIEW_ELECTION_EVENT],
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
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error creating a transaction: {err}"),
            )
        })?;
    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring keycloak db client from pool {:?}", e),
            )
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring keycloak transaction {:?}", e),
            )
        })?;

    let realm = get_event_realm(&tenant_id, &input.election_event_id);

    let election_event_data: ElectionEventMonitoring =
        get_election_event_monitoring(
            &hasura_transaction,
            &keycloak_transaction,
            &tenant_id,
            &realm,
            &input.election_event_id,
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error at get_election_event_monitoring {:?}", e),
            )
        })?;

    let total_eligible_voters: i64 =
        count_keycloak_enabled_users(&keycloak_transaction, &realm)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(ElectionEventMonitoringOutput {
        total_eligible_voters,
        total_started_votes: election_event_data.total_started_votes,
        total_not_started_votes: election_event_data.total_not_started_votes,
        total_open_votes: election_event_data.total_open_votes,
        total_not_open_votes: election_event_data.total_not_open_votes,
        total_closed_votes: election_event_data.total_closed_votes,
        total_not_closed_votes: election_event_data.total_not_closed_votes,
        total_enrolled_voters: election_event_data.total_enrolled_voters,
        total_elections: election_event_data.total_elections,
        total_start_counting_votes: election_event_data
            .total_start_counting_votes,
        total_not_start_counting_votes: election_event_data
            .total_not_start_counting_votes,
        total_initialize: election_event_data.total_initialize,
        total_not_initialize: election_event_data.total_not_initialize,
        total_genereated_tally: election_event_data.total_genereated_tally,
        total_not_genereated_tally: election_event_data
            .total_not_genereated_tally,
        authentication_stats: election_event_data.authentication_stats,
        voting_stats: election_event_data.voting_stats,
        approval_stats: election_event_data.approval_stats,
        transmission_stats: election_event_data.transmission_stats,
    }))
}
