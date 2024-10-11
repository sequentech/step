// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::anyhow;
use anyhow::{Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::{CeremonyStatus, ExecutionStatus};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::hasura;
use windmill::hasura::election_event::get_election_event;
use windmill::hasura::keys_ceremony::get_keys_ceremonies;
use windmill::hasura::trustee::get_trustees_by_name;
use windmill::services::ceremonies::keys_ceremony;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::private_keys::get_trustee_encrypted_private_key;

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
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let is_valid = keys_ceremony::check_private_key(
        claims,
        tenant_id,
        input.election_event_id.clone(),
        input.keys_ceremony_id.clone(),
        input.private_key_base64.clone(),
    )
    .await
    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "Checking given private key, electionEventId={}, keysCeremonyId={}, is_valid={}",
        input.election_event_id,
        input.keys_ceremony_id,
        is_valid,
    );
    Ok(Json(CheckPrivateKeyOutput { is_valid }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /get-private-key
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
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let encrypted_private_key = keys_ceremony::get_private_key(
        claims,
        tenant_id,
        input.election_event_id.clone(),
        input.keys_ceremony_id.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "get_private_key: electionEventId={}, keysCeremonyId={}",
        input.election_event_id.clone(),
        input.keys_ceremony_id.clone(),
    );
    Ok(Json(GetPrivateKeyOutput {
        private_key_base64: encrypted_private_key,
    }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /create-keys-ceremony
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateKeysCeremonyInput {
    election_event_id: String,
    threshold: usize,
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
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id;

    let keys_ceremony_id = keys_ceremony::create_keys_ceremony(
        tenant_id,
        &user_id,
        input.election_event_id.clone(),
        input.threshold,
        input.trustee_names,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "Creating Keys Ceremony, electionEventId={}, keysCeremonyId={}",
        input.election_event_id,
        keys_ceremony_id,
    );
    Ok(Json(CreateKeysCeremonyOutput { keys_ceremony_id }))
}
