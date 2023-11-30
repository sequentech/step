// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPrivateKeyInput {
    election_event_id: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPrivateKeyOutput {
    private_key_base64: String,
}

// The main function to get the private key
#[instrument(skip(claims))]
#[post("/get-private-key", format = "json", data = "<body>")]
pub async fn get_private_key(
    body: Json<GetPrivateKeyInput>,
    claims: JwtClaims,
) -> Result<Json<GetPrivateKeyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        None,
        vec![Permissions::TRUSTEE_READ],
    )?;
    let input = body.into_inner();
    // The trustee name is simply the username of the user
    let trustee_name = claims
        .preferred_username
        .ok_or((Status::Unauthorized, "Empty username".to_string()))?;

    let private_key_base64 = "".into();
    /* TODO:
    let private_key_base64 = your_service::retrieve_private_key(&input.election_event_id, &input.trustee_name)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    */

    event!(
        Level::INFO,
        "Retrieved private key for electionEventId={}, trusteeName={}",
        input.election_event_id,
        trustee_name
    );
    Ok(Json(GetPrivateKeyOutput { private_key_base64 }))
}
