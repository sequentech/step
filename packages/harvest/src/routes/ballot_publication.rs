// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::services::ceremonies::tally_ceremony;

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotInput {
    election_event_id: String,
    election_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotOutput {
    ballot_publication_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/publish-ballot", format = "json", data = "<body>")]
pub async fn publish_ballot(
    body: Json<PublishBallotInput>,
    claims: JwtClaims,
) -> Result<Json<PublishBallotOutput>, (Status, String)> {
    Ok(Json(PublishBallotOutput {
        ballot_publication_id: "".into(),
    }))
}
