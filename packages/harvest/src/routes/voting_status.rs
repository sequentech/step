// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use crate::services::voting_status;
use anyhow::{Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::VotingStatus;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::election_event_status;
use windmill::services::electoral_log::*;

#[instrument(skip(claims))]
#[post("/update-event-voting-status", format = "json", data = "<body>")]
pub async fn update_event_status(
    body: Json<voting_status::UpdateEventVotingStatusInput>,
    claims: JwtClaims,
) -> Result<Json<voting_status::UpdateEventVotingStatusOutput>, (Status, String)>
{
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_STATE_WRITE],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let result = voting_status::update_event_status(input, tenant_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(result))
}

#[instrument(skip(claims))]
#[post("/update-election-voting-status", format = "json", data = "<body>")]
pub async fn update_election_status(
    body: Json<voting_status::UpdateElectionVotingStatusInput>,
    claims: JwtClaims,
) -> Result<
    Json<voting_status::UpdateElectionVotingStatusOutput>,
    (Status, String),
> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_STATE_WRITE],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let result = voting_status::update_election_status(input, tenant_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(result))
}
