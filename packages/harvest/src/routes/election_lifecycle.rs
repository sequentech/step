// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use rocket::{http::Status, serde::json::Json};
use sequent_core::election_lifecycle::LifecycleState;
use sequent_core::{
    services::jwt::{has_fresh_board_authentication, JwtClaims},
    types::permissions::Permissions,
};
use serde::Deserialize;
use windmill::services::{database::get_hasura_pool, election_lifecycle};

#[derive(Deserialize)]
pub struct LifecycleInput {
    election_event_id: String,
}
fn internal(error: impl std::fmt::Debug) -> (Status, String) {
    tracing::error!(?error, "Election lifecycle request failed");
    (
        Status::InternalServerError,
        "Election lifecycle request failed".into(),
    )
}
#[post("/election-lifecycle/status", format = "json", data = "<input>")]
pub async fn lifecycle_status(
    input: Json<LifecycleInput>,
    claims: JwtClaims,
) -> Result<Json<LifecycleState>, (Status, String)> {
    let tenant = &claims.hasura_claims.tenant_id;
    authorize(
        &claims,
        true,
        Some(tenant.clone()),
        vec![Permissions::SELF_TEST_READ],
    )?;
    let mut client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    authorize_election_permission_labels(
        &tx,
        &claims,
        &input.election_event_id,
        None,
    )
    .await?;
    let state =
        election_lifecycle::state(&tx, tenant, &input.election_event_id)
            .await
            .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(state))
}
#[post(
    "/election-lifecycle/post-processing",
    format = "json",
    data = "<input>"
)]
pub async fn start_post_processing(
    input: Json<LifecycleInput>,
    claims: JwtClaims,
) -> Result<Json<LifecycleState>, (Status, String)> {
    let tenant = &claims.hasura_claims.tenant_id;
    authorize(
        &claims,
        true,
        Some(tenant.clone()),
        vec![Permissions::ELECTION_STATE_WRITE],
    )?;
    if !has_fresh_board_authentication(&claims) {
        return Err((
            Status::Forbidden,
            "Fresh external authentication is required".into(),
        ));
    }
    let mut client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    authorize_election_permission_labels(
        &tx,
        &claims,
        &input.election_event_id,
        None,
    )
    .await?;
    election_lifecycle::start_post_processing(
        &tx,
        tenant,
        &input.election_event_id,
    )
    .await
    .map_err(|error| (Status::BadRequest, error.to_string()))?;
    let state =
        election_lifecycle::state(&tx, tenant, &input.election_event_id)
            .await
            .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(state))
}
