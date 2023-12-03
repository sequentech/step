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
use crate::hasura::trustee::get_trustees_by_name;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /check-private-key
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckPrivateKeyInput {
    election_event_id: String,
    keys_ceremony_id: String,
    private_key_base64: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckPrivateKeyOutput {
    is_valid: bool,
}

// The main function to get the private key
#[instrument(skip(claims))]
#[post("/check-private-key", format = "json", data = "<body>")]
pub async fn check_private_key(
    body: Json<CheckPrivateKeyInput>,
    claims: JwtClaims,
) -> Result<Json<CheckPrivateKeyOutput>, (Status, String)> {
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

    let private_key_base64: String = "".into();
    /* TODO:
    let private_key = your_service::check_private_key(
        &input.election_event_id,
        &input.keys_ceremony_id,
        &input.trustee_name,
        &input.private_key)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    */
    let is_valid = private_key_base64 == input.private_key_base64;
    event!(
        Level::INFO,
        "Checking given private key, electionEventId={}, keysCeremonyId={}, trusteeName={}, is_valid={}",
        input.election_event_id,
        input.keys_ceremony_id,
        trustee_name,
        is_valid,
    );
    Ok(Json(CheckPrivateKeyOutput { is_valid }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /check-private-key
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPrivateKeyInput {
    election_event_id: String,
    keys_ceremony_id: String,
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
    let private_key_base64 = your_service::retrieve_private_key(
        &input.election_event_id,
        &input.keys_ceremony_id,
        &input.trustee_name
    )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    */

    event!(
        Level::INFO,
        "Retrieved private key for electionEventId={}, keysCeremonyId={}, trusteeName={}",
        input.election_event_id,
        input.keys_ceremony_id,
        trustee_name
    );
    Ok(Json(GetPrivateKeyOutput { private_key_base64 }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /create-keys-ceremony
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateKeysCeremonyInput {
    election_event_id: String,
    threshold: String,
    trustee_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateKeysCeremonyOutput {
    keys_ceremony_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/create-keys-ceremony", format = "json", data = "<body>")]
pub async fn create_keys_ceremony(
    body: Json<CreateKeysCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateKeysCeremonyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        None,
        vec![Permissions::ADMIN_CEREMONY],
    )?;
    let input = body.into_inner();

    let trustees = get_trustees_by_name(
        auth_headers.clone(),
        tenant_id.clone(),
        body.trustee_names.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find trustees")?
    .sequent_backend_trustee;

    let keys_ceremony_id: String = Uuid::new_v4().to_string();
    /* TODO:
    let keys_ceremony_id = your_service::create_keys_ceremony(
        &input.election_event_id
    )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    */
    event!(
        Level::INFO,
        "Creating Keys Ceremony, electionEventId={}, keysCeremonyId={}",
        input.election_event_id,
        keys_ceremony_id,
    );
    Ok(Json(CreateKeysCeremonyOutput { keys_ceremony_id }))
}
