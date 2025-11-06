// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
use windmill::postgres::area::get_areas_by_election_id;
use windmill::services::database::get_keycloak_pool;
use windmill::services::elections_monitoring::{
    get_election_monitoring, ElectionMonitoring, MonitoringApproval,
    MonitoringAuthentication, MonitoringVotingStatus,
};
use windmill::services::users::{list_users, ListUsersFilter};
use windmill::services::{
    database::get_hasura_pool,
    elections_monitoring::{
        get_election_event_monitoring, ElectionEventMonitoring,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventMonitoringInput {
    election_event_id: String,
    election_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionMonitoringOutput {
    total_eligible_voters: i64,
    total_enrolled_voters: i64,
    total_voted: i64,
    authentication_stats: MonitoringAuthentication,
    approval_stats: MonitoringApproval,
}

#[instrument(skip(claims))]
#[post("/election-monitoring", format = "json", data = "<body>")]
pub async fn get_election_monitoring_f(
    body: Json<ElectionEventMonitoringInput>,
    claims: JwtClaims,
) -> Result<Json<ElectionMonitoringOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::MONITORING_DASHBOARD_VIEW_ELECTION],
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

    let a = get_areas_by_election_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.election_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error at get_areas_by_election_id {:?}", e),
        )
    })?;

    let election_data: ElectionMonitoring = get_election_monitoring(
        &hasura_transaction,
        &keycloak_transaction,
        &tenant_id,
        &realm,
        &input.election_event_id,
        &input.election_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error at get_election_monitoring {:?}", e),
        )
    })?;

    Ok(Json(ElectionMonitoringOutput {
        total_eligible_voters: election_data.total_eligible_voters,
        total_enrolled_voters: election_data.total_enrolled_voters,
        total_voted: election_data.total_voted,
        authentication_stats: election_data.authentication_stats,
        approval_stats: election_data.approval_stats,
    }))
}
