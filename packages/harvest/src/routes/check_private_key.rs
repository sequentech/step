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
pub struct CheckPrivateKeyInput {
    election_event_id: String,
    private_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckPrivateKeyOutput {
    private_key: Vec<u8>,
}

// The main function to get the private key
#[instrument(skip(claims))]
#[post("/check-private-key", format = "json", data = "<body>")]
pub async fn check_private_key(
    body: Json<CheckPrivateKeyInput>,
    claims: JwtClaims,
) -> Result<(), (Status, String)> {
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

    let private_key: Vec<u8> = Vec::new();
    /* TODO:
    let private_key = your_service::check_private_key(&input.election_event_id, &input.trustee_name, &input.private_key)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    */
    let result = private_key == input.private_key;
    event!(
        Level::INFO,
        "Checking given private key, electionEventId={}, trusteeName={}, result={}",
        input.election_event_id,
        trustee_name,
        result,
    );
    match result {
        true => Ok(()),
        false => Err((Status::BadRequest, "Invalid private key".to_string())),
    }
}
