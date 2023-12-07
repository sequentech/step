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
    authorize(&claims, true, None, vec![Permissions::TRUSTEE_READ])?;
    let input = body.into_inner();
    // The trustee name is simply the username of the user
    let trustee_name = claims
        .clone()
        .preferred_username
        .ok_or((Status::Unauthorized, "Empty username".to_string()))?;

    let GetPrivateKeyOutput {private_key_base64} =
        get_private_key(
            Json(GetPrivateKeyInput {
                election_event_id: input.election_event_id.clone(),
                keys_ceremony_id: input.keys_ceremony_id.clone(),
            }),
            claims.clone(),
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
        .into_inner()
    else {
        return Err((
            Status::BadRequest,
            "Keys ceremony not in ExecutionStatus::IN_PROCESS".into()
        ));
    };

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
    authorize(&claims, true, None, vec![Permissions::TRUSTEE_READ])?;
    let input = body.into_inner();
    let auth_headers = keycloak::get_client_credentials()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    // The trustee name is simply the username of the user
    let trustee_name = claims
        .preferred_username
        .ok_or((Status::Unauthorized, "Empty username".to_string()))?;

    // get the keys ceremonies for this election event
    let keys_ceremony = get_keys_ceremonies(
        auth_headers.clone(),
        tenant_id.clone(),
        input.election_event_id.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .data
    .with_context(|| "error listing existing keys ceremonies")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .sequent_backend_keys_ceremony
    .into_iter()
    .find(|ceremony| ceremony.id == input.keys_ceremony_id)
    .ok_or((Status::BadRequest, "Keys ceremony not found".into()))?;
    // check keys_ceremony has correct execution status
    if keys_ceremony.execution_status
        != Some(ExecutionStatus::IN_PROCESS.to_string())
    {
        return Err((
            Status::BadRequest,
            "Keys ceremony not in ExecutionStatus::IN_PROCESS".into(),
        ));
    }
    // get ceremony status
    let current_status: CeremonyStatus = serde_json::from_value(
        keys_ceremony
            .status
            .clone()
            .ok_or(anyhow!("Empty keys ceremony status"))
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?,
    )
    .with_context(|| "error parsing keys ceremony current status")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    // check the trustee is part of this ceremony
    if let None = current_status
        .trustees
        .clone()
        .into_iter()
        .find(|trustee| trustee.name == trustee_name)
    {
        return Err((
            Status::BadRequest,
            "Trustee not part of the keys ceremony".into(),
        ));
    }

    // fetch election_event
    let election_event = &get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        input.election_event_id.clone(),
    )
    .await
    .with_context(|| "error fetching election event")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .data
    .with_context(|| "error fetching election event")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .sequent_backend_election_event[0];

    // get board name
    let board_name = get_election_event_board(
        election_event.bulletin_board_reference.clone(),
    )
    .with_context(|| "missing bulletin board")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let trustee_public_key = get_trustees_by_name(
        auth_headers.clone(),
        tenant_id.clone(),
        vec![trustee_name.clone()],
    )
    .await
    .with_context(|| "can't find trustee in the database")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .data
    .with_context(|| "error fetching election event")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .sequent_backend_trustee[0]
        .public_key
        .clone()
        .ok_or((
            Status::InternalServerError,
            "can't get election event".into(),
        ))?;

    // get the encrypted private key
    let encrypted_private_key = get_trustee_encrypted_private_key(
        board_name.as_str(),
        trustee_public_key.as_str(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "Retrieved private key for electionEventId={}, keysCeremonyId={}, trusteeName={}",
        input.election_event_id.clone(),
        input.keys_ceremony_id.clone(),
        trustee_name.clone()
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
    authorize(&claims, true, None, vec![Permissions::ADMIN_CEREMONY])?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let keys_ceremony_id = keys_ceremony::create_keys_ceremony(
        tenant_id,
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
