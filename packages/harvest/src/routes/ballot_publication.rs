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
use windmill::services::ballot_publication::add_ballot_publication;

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotInput {
    election_event_id: String,
    election_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotOutput {
    ballot_publication_id: String,
}

#[instrument(skip(claims))]
#[post("/publish-ballot", format = "json", data = "<body>")]
pub async fn publish_ballot(
    body: Json<PublishBallotInput>,
    claims: JwtClaims,
) -> Result<Json<PublishBallotOutput>, (Status, String)> {
    authorize(&claims, true, None, vec![Permissions::PUBLISH_WRITE])?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();

    let ballot_publication_id = add_ballot_publication(
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.election_ids.clone(),
        user_id.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(PublishBallotOutput {
        ballot_publication_id,
    }))
}
