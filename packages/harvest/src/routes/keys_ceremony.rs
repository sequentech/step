// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::access::authorize_any;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::signing_gate::Guarded;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::anyhow;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::Display;
use tracing::{error, event, instrument, Level};
use uuid::Uuid;
use windmill::postgres;
use windmill::postgres::election::get_elections;
use windmill::services::ceremonies::keys_ceremony::{
    self, validate_permission_labels, PrivateKeyDownloadUnavailable,
};
use windmill::services::database::get_hasura_pool;
use windmill::services::signing::guard::SigningRequestSummary;
use windmill::services::signing::key_shares::{
    take_key_share_step, KeyShareError, KeyShareInput, KeyShareKind,
    KeyShareOutcome, KeyShareSignatureStatus, KeysCeremonyKeyShare,
};
use windmill::services::signing::SigningCaller;
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /check-private-key
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct CheckPrivateKeyInput {
    election_event_id: String,
    keys_ceremony_id: String,
    private_key_base64: String,
    /// The SHA-256 the browser computed of the key share.
    #[serde(default)]
    key_share_sha256: Option<String>,
    /// The trustee's completed signing request, when the step needs it.
    #[serde(default)]
    signing_request_id: Option<Uuid>,
}

#[derive(Serialize, Debug)]
pub struct CheckPrivateKeyOutput {
    /// Whether the key share is the trustee's.
    is_valid: bool,
    /// The request the trustee signs before the check is recorded.
    signing_request: Option<SigningRequestSummary>,
}

/// A refused key step: the ceremony's refusals as the routes always
/// answered them, signing refusals in the signing contract's body.
pub fn key_share_failure(error: KeyShareError) -> Guarded<(Status, String)> {
    match error {
        KeyShareError::Step(error) => {
            Guarded::Route((Status::BadRequest, format!("{:?}", error)))
        }
        KeyShareError::Signing(error) => Guarded::from(error),
    }
}

/// What a key step answers: `is_valid`, and the request to sign while the
/// step waits for it. Commits unless the key share was not the trustee's.
pub async fn finish_key_share_step(
    hasura_transaction: deadpool_postgres::Transaction<'_>,
    outcome: KeyShareOutcome,
) -> Result<(bool, Option<SigningRequestSummary>), (Status, String)> {
    let (answer, staged) = match outcome {
        // Nothing is recorded and no request is used.
        KeyShareOutcome::Invalid => return Ok((false, None)),
        KeyShareOutcome::Done(record) => ((true, None), record.is_some()),
        KeyShareOutcome::SigningRequired(summary) => {
            ((true, Some(summary)), true)
        }
    };
    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;
    // Only a signing step stages log entries.
    if staged {
        kick_signing_log_outbox();
    }
    Ok(answer)
}

// The main function to get the private key
#[instrument(skip(body, claims, services))]
#[post("/check-private-key", format = "json", data = "<body>")]
pub async fn check_private_key(
    body: Json<CheckPrivateKeyInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<CheckPrivateKeyOutput>, Guarded<(Status, String)>> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let outcome = take_key_share_step(
        &hasura_transaction,
        &SigningCaller::from_claims(&claims),
        &KeysCeremonyKeyShare {
            claims: &claims,
            tenant_id: &tenant_id,
            election_event_id: &input.election_event_id,
            keys_ceremony_id: &input.keys_ceremony_id,
        },
        &KeyShareInput {
            tenant_id: &tenant_id,
            election_event_id: &input.election_event_id,
            kind: KeyShareKind::Confirm,
            target_id: &input.keys_ceremony_id,
            key_share: &input.private_key_base64,
            key_share_sha256: input.key_share_sha256.as_deref(),
            signing_request_id: input.signing_request_id,
        },
    )
    .await
    .map_err(key_share_failure)?;
    let (is_valid, signing_request) =
        finish_key_share_step(hasura_transaction, outcome).await?;

    event!(
        Level::INFO,
        "Checking given private key, electionEventId={}, keysCeremonyId={}, is_valid={}, signing_request={:?}",
        input.election_event_id,
        input.keys_ceremony_id,
        is_valid,
        signing_request.as_ref().map(|request| request.id),
    );

    Ok(Json(CheckPrivateKeyOutput {
        is_valid,
        signing_request,
    }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /key-share-signature-status
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyShareSignatureStatusInput {
    election_event_id: String,
    /// The keys ceremony, for confirming a key share.
    #[serde(default)]
    keys_ceremony_id: Option<String>,
    /// The tally session, for contributing a key share.
    #[serde(default)]
    tally_session_id: Option<String>,
}

/// Whether the signed-in trustee's key step needs their signature, and
/// whether they already have a used signed step for it.
#[instrument(skip(claims, services))]
#[post("/key-share-signature-status", format = "json", data = "<body>")]
pub async fn key_share_signature_status(
    body: Json<KeyShareSignatureStatusInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<KeyShareSignatureStatus>, Guarded<(Status, String)>> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let input = body.into_inner();
    let (kind, target_id) =
        match (&input.keys_ceremony_id, &input.tally_session_id) {
            (Some(id), None) => (KeyShareKind::Confirm, id),
            (None, Some(id)) => (KeyShareKind::Contribute, id),
            _ => {
                return Err(Guarded::Route((
                    Status::BadRequest,
                    "Name either a keys ceremony or a tally session."
                        .to_owned(),
                )))
            }
        };
    let mut hasura_db_client: DbClient = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let status =
        windmill::services::signing::key_shares::key_share_signature_status(
            &hasura_transaction,
            &SigningCaller::from_claims(&claims),
            &claims.hasura_claims.tenant_id,
            &input.election_event_id,
            kind,
            target_id,
        )
        .await
        .map_err(key_share_failure)?;
    Ok(Json(status))
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

fn private_key_download_unavailable() -> JsonError {
    ErrorResponse::new(
        Status::Conflict,
        "Private key download is no longer available",
        ErrorCode::PrivateKeyDownloadUnavailable,
    )
}

fn private_key_download_internal_error() -> JsonError {
    ErrorResponse::new(
        Status::InternalServerError,
        "Failed to download private key",
        ErrorCode::InternalServerError,
    )
}

fn private_key_download_error(
    error: anyhow::Error,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> JsonError {
    if error
        .downcast_ref::<PrivateKeyDownloadUnavailable>()
        .is_some()
    {
        private_key_download_unavailable()
    } else {
        error!(
            election_event_id = %election_event_id,
            keys_ceremony_id = %keys_ceremony_id,
            "Failed to download private key: {error:#}"
        );
        private_key_download_internal_error()
    }
}

// The main function to get the private key
#[instrument(skip(claims))]
#[post("/get-private-key", format = "json", data = "<body>")]
pub async fn get_private_key(
    body: Json<GetPrivateKeyInput>,
    claims: JwtClaims,
) -> Result<Json<GetPrivateKeyOutput>, JsonError> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )
    .map_err(|(status, message)| {
        let code =
            if status == Status::Unauthorized || status == Status::Forbidden {
                ErrorCode::Unauthorized
            } else {
                ErrorCode::UnknownError
            };
        ErrorResponse::new(status, &message, code)
    })?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| {
            error!("Failed to get database client for private key download: {error:#}");
            private_key_download_internal_error()
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|error| {
            error!(
                "Failed to start private key download transaction: {error:#}"
            );
            private_key_download_internal_error()
        })?;

    let encrypted_private_key = keys_ceremony::get_private_key(
        &hasura_transaction,
        claims,
        tenant_id,
        input.election_event_id.clone(),
        input.keys_ceremony_id.clone(),
    )
    .await
    .map_err(|error| {
        private_key_download_error(
            error,
            &input.election_event_id,
            &input.keys_ceremony_id,
        )
    })?;

    event!(
        Level::INFO,
        "get_private_key: electionEventId={}, keysCeremonyId={}",
        input.election_event_id.clone(),
        input.keys_ceremony_id.clone(),
    );

    hasura_transaction.commit().await.map_err(|error| {
        error!("Failed to commit private key download transaction: {error:#}");
        private_key_download_internal_error()
    })?;

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
        input.is_automatic_ceremony,
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
    authorize_any(
        authorize(
            &claims,
            true,
            Some(claims.hasura_claims.tenant_id.clone()),
            vec![Permissions::ADMIN_CEREMONY],
        ),
        authorize(
            &claims,
            true,
            Some(claims.hasura_claims.tenant_id.clone()),
            vec![Permissions::TRUSTEE_CEREMONY],
        ),
    )?;
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

#[cfg(test)]
#[path = "../../tests/support/private_key_errors.rs"]
mod private_key_errors;
