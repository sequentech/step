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
use windmill::services::ballot_publication::{add_ballot_publication, update_publish_ballot};
use windmill::services::ceremonies::tally_ceremony;

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateBallotPublicationInput {
    election_event_id: String,
    election_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateBallotPublicationOutput {
    ballot_publication_id: String,
}

#[instrument(skip(claims))]
#[post("/generate-ballot-publication", format = "json", data = "<body>")]
pub async fn generate_ballot_publication(
    body: Json<GenerateBallotPublicationInput>,
    claims: JwtClaims,
) -> Result<Json<GenerateBallotPublicationOutput>, (Status, String)> {
    authorize(&claims, true, None, vec![Permissions::PUBLISH_WRITE])?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();

    let ballot_publication_id = add_ballot_publication(
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.election_id.clone(),
        user_id.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(GenerateBallotPublicationOutput {
        ballot_publication_id,
    }))
}


#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotInput {
    election_event_id: String,
    ballot_publication_id: String,
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

    update_publish_ballot(
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.ballot_publication_id.clone(),
    ).await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(PublishBallotOutput {
        ballot_publication_id: input.ballot_publication_id.clone(),
    }))
}
