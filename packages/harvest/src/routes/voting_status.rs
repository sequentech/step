// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::VotingStatus;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::services::ceremonies::tally_ceremony;
use windmill::services::election_event_status;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusInput {
    pub election_event_id: String,
    pub voting_status: VotingStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusOutput {
    pub election_event_id: String,
}

#[instrument(skip(claims))]
#[post("/update-event-voting-status", format = "json", data = "<body>")]
pub async fn update_event_status(
    body: Json<UpdateEventVotingStatusInput>,
    claims: JwtClaims,
) -> Result<Json<UpdateEventVotingStatusOutput>, (Status, String)> {
    authorize(&claims, true, None, vec![Permissions::ELECTION_STATE_WRITE])?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    election_event_status::update_event_voting_status(
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.voting_status.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(UpdateEventVotingStatusOutput {
        election_event_id: input.election_event_id.clone(),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateElectionVotingStatusInput {
    pub election_event_id: String,
    pub election_id: String,
    pub voting_status: VotingStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateElectionVotingStatusOutput {
    pub election_id: String,
}

#[instrument(skip(claims))]
#[post("/update-election-voting-status", format = "json", data = "<body>")]
pub async fn update_election_status(
    body: Json<UpdateElectionVotingStatusInput>,
    claims: JwtClaims,
) -> Result<Json<UpdateElectionVotingStatusOutput>, (Status, String)> {
    authorize(&claims, true, None, vec![Permissions::ELECTION_STATE_WRITE])?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    election_event_status::update_election_voting_status(
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.election_id.clone(),
        input.voting_status.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(UpdateElectionVotingStatusOutput {
        election_id: input.election_id.clone(),
    }))
}
