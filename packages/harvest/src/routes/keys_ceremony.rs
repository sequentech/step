// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::anyhow;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::postgres;
use windmill::postgres::election::get_elections;
use windmill::services::ceremonies::keys_ceremony::{
    self, validate_permission_labels,
};
use windmill::services::database::get_hasura_pool;

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

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let is_valid = keys_ceremony::check_private_key(
        &hasura_transaction,
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

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

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

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let encrypted_private_key = keys_ceremony::get_private_key(
        &hasura_transaction,
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

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

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
    election_id: Option<String>,
    name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateKeysCeremonyOutput {
    keys_ceremony_id: String,
    error_message: Option<String>,
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
    let user_permission_labels = claims.hasura_claims.permission_labels;

    let username = claims.preferred_username.unwrap_or("-".to_string());

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let valid_permissions_label = validate_permission_labels(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        input.election_id.clone(),
        user_permission_labels,
    )
    .await;
    if let Err(err) = valid_permissions_label {
        return Ok(Json(CreateKeysCeremonyOutput {
            keys_ceremony_id: "".to_string(),
            error_message: Some(format!(
                "user permission labels are not valid"
            )),
        }));
    }

    let keys_ceremony_id = keys_ceremony::create_keys_ceremony(
        &hasura_transaction,
        tenant_id,
        &user_id,
        &username,
        input.election_event_id.clone(),
        input.threshold,
        input.trustee_names,
        input.election_id.clone(),
        input.name,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "Creating Keys Ceremony, electionEventId={}, keysCeremonyId={}, electionId={:?}",
        input.election_event_id,
        keys_ceremony_id,
        input.election_id,
    );
    Ok(Json(CreateKeysCeremonyOutput {
        keys_ceremony_id,
        error_message: None,
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListKeysCeremonyInput {
    election_event_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/list-keys-ceremonies", format = "json", data = "<body>")]
pub async fn list_keys_ceremonies(
    body: Json<ListKeysCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<DataList<KeysCeremony>>, (Status, String)> {
    let admin_auth = authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    );

    let trustee_auth = authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    );
    if admin_auth.is_err() {
        trustee_auth?;
    } else if trustee_auth.is_err() {
        admin_auth?;
    }
    let permission_labels = decode_permission_labels(&claims);

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let elections = get_elections(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        None,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let election_permission_labels: Vec<_> = elections
        .into_iter()
        .filter_map(|election| election.permission_label)
        .collect();

    let filtered_labels = if election_permission_labels.len() > 0 {
        permission_labels
    } else {
        vec![]
    };

    let keys_ceremonies = postgres::keys_ceremony::list_keys_ceremony(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &filtered_labels,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let count = keys_ceremonies.len() as i64;
    Ok(Json(DataList {
        items: keys_ceremonies,
        total: TotalAggregate {
            aggregate: Aggregate { count: count },
        },
    }))
}
