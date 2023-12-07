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

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventStatusInput {
    pub election_event_id: String,
    pub voting_status: VotingStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventStatusOutput {
    pub election_event_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/update-event-status", format = "json", data = "<body>")]
pub async fn update_event_status(
    body: Json<UpdateEventStatusInput>,
    claims: JwtClaims,
) -> Result<Json<UpdateEventStatusOutput>, (Status, String)> {
    Ok(Json(UpdateEventStatusOutput {
        election_event_id: "".into(),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateElectionStatusInput {
    pub election_id: String,
    pub voting_status: VotingStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateElectionStatusOutput {
    pub election_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/update-election-status", format = "json", data = "<body>")]
pub async fn update_election_status(
    body: Json<UpdateElectionStatusInput>,
    claims: JwtClaims,
) -> Result<Json<UpdateElectionStatusOutput>, (Status, String)> {
    Ok(Json(UpdateElectionStatusOutput {
        election_id: "".into(),
    }))
}
