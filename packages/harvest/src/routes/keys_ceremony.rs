// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::anyhow;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims, SERVER_DEFAULT_ROLE, USER_DEFAULT_ROLE};
use sequent_core::types::ceremonies::TrusteeModePolicy;
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::Display;
use tracing::{error, event, instrument, Level};
use windmill::postgres;
use windmill::postgres::election::get_elections;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::postgres::trustee::{
    get_trustee_by_name, get_trustee_mode_policy, update_trustee_key_for_event,
};
use windmill::services::ceremonies::keys_ceremony::{
    self, validate_permission_labels,
};
use windmill::services::database::get_hasura_pool;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::keycloak::add_board_to_trustee_authorized_boards;

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
    is_automatic_ceremony: bool,
}

#[derive(Debug, Display)]
pub enum CreateKeysError {
    #[strum(serialize = "permission-labels")]
    PermissionLabels,
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

    event!(
        Level::INFO,
        "Creating Keys Ceremony, electionEventId={}, electionId={:?}",
        input.election_event_id,
        input.election_id,
    );
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
    .await
    .map_err(|e| {
        (
            Status::BadRequest,
            format!("Error validating permission labels: {:?}", e),
        )
    })?;

    if !valid_permissions_label {
        error!("User does not have permission labels");
        return Ok(Json(CreateKeysCeremonyOutput {
            keys_ceremony_id: "".to_string(),
            error_message: Some(CreateKeysError::PermissionLabels.to_string()),
        }));
    }

    let (keys_ceremony_id, board_name) = keys_ceremony::create_keys_ceremony(
        &hasura_transaction,
        tenant_id.clone(),
        &user_id,
        &username,
        input.election_event_id.clone(),
        input.threshold,
        input.trustee_names.clone(),
        input.election_id.clone(),
        input.name,
        input.is_automatic_ceremony,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    // Update each trustee's authorized-boards in Keycloak so their JWT
    // contains the board_name required by BoardAccessValidator.
    for trustee_name in &input.trustee_names {
        add_board_to_trustee_authorized_boards(
            &tenant_id,
            &board_name,
            trustee_name,
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!(
                    "Error adding board to trustee's authorized boards in Keycloak: {:?}",
                    e
                ),
            )
        })?;
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "Created Keys Ceremony, electionEventId={}, keysCeremonyId={}, electionId={:?}",
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

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /register-trustee-key
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterTrusteeKeyInput {
    pub public_key: String,
    pub election_event_id: String,
    pub keys_ceremony_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterTrusteeKeyOutput {
    pub success: bool,
}

#[instrument(skip(claims))]
#[post("/register-trustee-key", format = "json", data = "<body>")]
pub async fn register_trustee_key(
    body: Json<RegisterTrusteeKeyInput>,
    claims: JwtClaims,
) -> Result<Json<RegisterTrusteeKeyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let trustee_name = claims.trustee.ok_or_else(|| {
        (
            Status::Unauthorized,
            "trustee name not found in token".to_string(),
        )
    })?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let trustee =
        get_trustee_by_name(&hasura_transaction, &tenant_id, &trustee_name)
            .await
            .map_err(|e| {
                (
                    Status::NotFound,
                    format!("Trustee '{trustee_name}' not found: {e:?}"),
                )
            })?;

    // Validate caller mode matches trustee mode. /register-trustee-key is
    // exclusively for browser-based trustees (BBT flow). Server-based trustees
    // use /get-private-key and /check-private-key instead.
    let trustee_mode = get_trustee_mode_policy(&trustee);

    // Map JWT default_role claim to caller's trustee mode policy.
    // SERVER_DEFAULT_ROLE ("server", set by native-trustee Keycloak client)
    // maps to SERVER_BASED; USER_DEFAULT_ROLE ("user", set by voting-portal
    // and browser clients) maps to BROWSER_BASED. Any other value is rejected.
    let caller_mode = match claims.hasura_claims.default_role.as_str() {
        SERVER_DEFAULT_ROLE => TrusteeModePolicy::SERVER_BASED,
        USER_DEFAULT_ROLE => TrusteeModePolicy::BROWSER_BASED,
        unknown => {
            return Err((
                Status::Unauthorized,
                format!(
                    "Unrecognized default_role: '{}'; expected '{}' or '{}'",
                    unknown, SERVER_DEFAULT_ROLE, USER_DEFAULT_ROLE
                ),
            ))
        }
    };

    // Caller and trustee modes must match
    if trustee_mode != caller_mode {
        return Err((
            Status::Forbidden,
            format!(
                "Trustee '{}' is configured as {:?} but caller is {:?}",
                trustee_name, trustee_mode, caller_mode
            ),
        ));
    }

    // /register-trustee-key is exclusively for browser-based trustees
    if trustee_mode != TrusteeModePolicy::BROWSER_BASED {
        return Err((
            Status::Forbidden,
            format!(
                "Trustee '{}' is {:?}; this endpoint is only for browser-based trustees",
                trustee_name, trustee_mode
            ),
        ));
    }

    update_trustee_key_for_event(
        &hasura_transaction,
        &tenant_id,
        &trustee.id,
        &input.election_event_id,
        &input.keys_ceremony_id,
        &input.public_key,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    event!(
        Level::INFO,
        "Registered trustee key: trustee={trustee_name}, election_event_id={}, keys_ceremony_id={}",
        input.election_event_id,
        input.keys_ceremony_id,
    );

    Ok(Json(RegisterTrusteeKeyOutput { success: true }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /active-ceremonies
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscoverActiveCeremoniesInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub election_event_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActiveCeremony {
    pub keys_ceremony_id: String,
    pub election_event_id: String,
    pub tenant_id: String,
    pub board_name: String,
    pub execution_status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscoverActiveCeremoniesOutput {
    pub ceremonies: Vec<ActiveCeremony>,
}

/// Discover every active keys ceremony for this trustee.
/// Returns all ceremonies in AWAITING_TRUSTEE_KEYS or IN_PROGRESS status where
/// the caller is registered as a trustee — one per election event the trustee
/// participates in. Optionally narrowed to a single event via `election_event_id`.
#[instrument(skip(claims))]
#[post("/active-ceremonies", format = "json", data = "<body>")]
pub async fn discover_active_ceremonies(
    body: Json<DiscoverActiveCeremoniesInput>,
    claims: JwtClaims,
) -> Result<Json<DiscoverActiveCeremoniesOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let trustee_name = claims.trustee.ok_or_else(|| {
        (
            Status::Unauthorized,
            "trustee name not found in token".to_string(),
        )
    })?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    // Resolve the caller's trustee id from the JWT-provided name.
    let trustee = get_trustee_by_name(&hasura_transaction, &tenant_id, &trustee_name)
        .await
        .map_err(|e| {
            (
                Status::NotFound,
                format!("Trustee '{trustee_name}' not found: {e:?}"),
            )
        })?;

    let keys_ceremonies = postgres::keys_ceremony::get_active_ceremonies_for_trustee(
        &hasura_transaction,
        &tenant_id,
        &trustee.id,
        input.election_event_id.as_deref(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    // Resolve the authoritative board name for each ceremony from its election
    // event's bulletin_board_reference, so the caller never reconstructs it.
    let mut ceremonies: Vec<ActiveCeremony> = Vec::with_capacity(keys_ceremonies.len());
    for keys_ceremony in keys_ceremonies {
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &tenant_id,
            &keys_ceremony.election_event_id,
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

        let board_name = get_election_event_board(election_event.bulletin_board_reference)
            .ok_or_else(|| {
                (
                    Status::InternalServerError,
                    format!(
                        "Election event {} has no bulletin board reference",
                        keys_ceremony.election_event_id
                    ),
                )
            })?;

        let execution_status = keys_ceremony.execution_status().map_err(|e| {
            (
                Status::InternalServerError,
                format!("Invalid execution_status: {e:?}"),
            )
        })?;

        ceremonies.push(ActiveCeremony {
            keys_ceremony_id: keys_ceremony.id,
            election_event_id: keys_ceremony.election_event_id,
            tenant_id: tenant_id.clone(),
            board_name,
            execution_status: execution_status.to_string(),
        });
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    event!(
        Level::INFO,
        "Discovered {} active ceremonies for trustee={trustee_name}",
        ceremonies.len(),
    );

    Ok(Json(DiscoverActiveCeremoniesOutput { ceremonies }))
}
