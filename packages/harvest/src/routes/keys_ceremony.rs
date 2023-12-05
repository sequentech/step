// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::anyhow;
use anyhow::{Context, Result};
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::services::connection;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::hasura::election_event::get_election_event;
use windmill::hasura::keys_ceremony::insert_keys_ceremony;
use windmill::hasura::trustee::get_trustees_by_name;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::create_keys::{create_keys, CreateKeysBody};
use windmill::types::keys_ceremony::{
    CeremonyStatus, ExecutionStatus, Trustee, TrusteeStatus,
};

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
    authorize(&claims, true, None, vec![Permissions::TRUSTEE_READ])?;
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
    let auth_headers = keycloak::get_client_credentials()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let celery_app = get_celery_app().await;
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    // verify trustee names and fetch their objects to get their ids
    let trustees = get_trustees_by_name(
        auth_headers.clone(),
        tenant_id.clone(),
        input.trustee_names.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .data
    .with_context(|| "can't find trustees")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .sequent_backend_trustee;

    // obtain trustee ids list
    let trustee_ids = trustees
        .clone()
        .into_iter()
        .map(|trustee| trustee.id)
        .collect();

    // get the election event
    let election_event = &get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        input.election_event_id.clone(),
    )
    .await
    .with_context(|| "can't get election event")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .data
    .ok_or((
        Status::InternalServerError,
        "can't get election event".into(),
    ))?
    .sequent_backend_election_event[0];

    // check config is not already created
    let event_status: Option<ElectionEventStatus> =
        match election_event.status.clone() {
            Some(value) => serde_json::from_value(value).map_err(|e| {
                (Status::InternalServerError, format!("{:?}", e))
            })?,
            None => None,
        };
    if event_status
        .map(|val| val.is_config_created())
        .unwrap_or(false)
    {
        return Err((
            Status::BadRequest,
            "bulletin board config already created".into(),
        ));
    }
    // TODO cancel any previous ceremony or find if there's any and cancel this
    // one

    // generate default values
    let keys_ceremony_id: String = Uuid::new_v4().to_string();
    let execution_status: String = ExecutionStatus::NOT_STARTED.to_string();
    let status: Value = serde_json::to_value(CeremonyStatus {
        stop_date: None,
        public_key: None,
        logs: vec![],
        trustees: trustees
            .clone()
            .into_iter()
            .map(|trustee| {
                Ok(Trustee {
                    name: trustee.name.ok_or(anyhow!("empty trustee name"))?,
                    status: TrusteeStatus::WAITING,
                })
            })
            .collect::<Result<Vec<Trustee>>>()
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?,
    })
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    // insert keys-ceremony into the database using graphql
    insert_keys_ceremony(
        auth_headers.clone(),
        keys_ceremony_id.clone(),
        tenant_id.clone(),
        input.election_event_id.clone(),
        trustee_ids,
        /* status */ Some(status),
        /* execution_status */ Some(execution_status),
    )
    .await
    .with_context(|| "couldn't insert keys ceremony")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    // create the public keys in async task
    let task = celery_app
        .send_task(create_keys::new(
            CreateKeysBody {
                threshold: input.threshold,
                trustee_pks: trustees
                    .clone()
                    .into_iter()
                    .map(|trustee| {
                        Ok(trustee
                            .public_key
                            .ok_or(anyhow!("empty trustee pub key"))?)
                    })
                    .collect::<Result<Vec<String>>>()
                    .map_err(|e| {
                        (Status::InternalServerError, format!("{:?}", e))
                    })?,
            },
            tenant_id.clone(),
            input.election_event_id.clone(),
        ))
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    event!(Level::INFO, "Sent create_keys task {}", task.task_id);

    event!(
        Level::INFO,
        "Creating Keys Ceremony, electionEventId={}, keysCeremonyId={}",
        input.election_event_id,
        keys_ceremony_id,
    );
    Ok(Json(CreateKeysCeremonyOutput { keys_ceremony_id }))
}
