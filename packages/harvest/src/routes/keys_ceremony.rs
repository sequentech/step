// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::access::authorize_any;
use crate::services::audit;
use crate::services::authorization::{
    authorize, authorize_tally_session_permission_labels,
};
use crate::services::dependencies::HarvestServices;
use crate::services::signing_gate::Guarded;
use crate::services::signing_http::authorize_403;
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
    self, validate_permission_labels, CustodyRequestDownloadUnavailable,
};
use windmill::services::database::get_hasura_pool;
use windmill::services::signing::custody_proofs::{
    take_custody_proof_step, CustodyProofError, CustodyProofInput,
    CustodyProofKind, CustodyProofOutcome, CustodyProofSignatureStatus,
    KeysCeremonyCustodyProof,
};
use windmill::services::signing::guard::SigningRequestSummary;
use windmill::services::signing::SigningCaller;
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /confirm-key-custody
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct ConfirmKeyCustodyInput {
    election_event_id: String,
    keys_ceremony_id: String,
    custody_proof_json: String,
    /// The SHA-256 the browser computed of the custody proof.
    #[serde(default)]
    custody_proof_sha256: Option<String>,
    /// The trustee's completed signing request, when the step needs it.
    #[serde(default)]
    signing_request_id: Option<Uuid>,
}

#[derive(Serialize, Debug)]
pub struct ConfirmKeyCustodyOutput {
    /// Whether the custody proof is the trustee's.
    is_valid: bool,
    /// The request the trustee signs before the check is recorded.
    signing_request: Option<SigningRequestSummary>,
}

/// A refused key step: the ceremony's refusals as the routes always
/// answered them, signing refusals in the signing contract's body.
pub fn custody_proof_failure(
    error: CustodyProofError,
) -> Guarded<(Status, String)> {
    match error {
        CustodyProofError::Step(error) => {
            Guarded::Route((Status::BadRequest, format!("{:?}", error)))
        }
        CustodyProofError::Signing(error) => Guarded::from(error),
    }
}

/// What a key step answers: `is_valid`, and the request to sign while the
/// step waits for it. Commits unless the custody proof was not the trustee's.
pub async fn finish_custody_proof_step(
    hasura_transaction: deadpool_postgres::Transaction<'_>,
    outcome: CustodyProofOutcome,
) -> Result<(bool, Option<SigningRequestSummary>), (Status, String)> {
    let (answer, staged) = match outcome {
        // Nothing is recorded and no request is used.
        CustodyProofOutcome::Invalid => return Ok((false, None)),
        CustodyProofOutcome::Done(record) => ((true, None), record.is_some()),
        CustodyProofOutcome::SigningRequired(summary) => {
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

// The trustee custody action
#[instrument(skip(body, claims, services))]
#[post("/confirm-key-custody", format = "json", data = "<body>")]
pub async fn confirm_key_custody(
    body: Json<ConfirmKeyCustodyInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<ConfirmKeyCustodyOutput>, Guarded<(Status, String)>> {
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
    audit::act_for(&hasura_transaction, &claims).await?;

    let outcome = take_custody_proof_step(
        &hasura_transaction,
        &SigningCaller::from_claims(&claims),
        &KeysCeremonyCustodyProof {
            claims: &claims,
            tenant_id: &tenant_id,
            election_event_id: &input.election_event_id,
            keys_ceremony_id: &input.keys_ceremony_id,
        },
        &CustodyProofInput {
            tenant_id: &tenant_id,
            election_event_id: &input.election_event_id,
            kind: CustodyProofKind::Confirm,
            target_id: &input.keys_ceremony_id,
            custody_proof: &input.custody_proof_json,
            custody_proof_sha256: input.custody_proof_sha256.as_deref(),
            signing_request_id: input.signing_request_id,
        },
    )
    .await
    .map_err(custody_proof_failure)?;
    let (is_valid, signing_request) =
        finish_custody_proof_step(hasura_transaction, outcome).await?;
    kick_audit_log_outbox();

    event!(
        Level::INFO,
        "Confirming public custody proof, electionEventId={}, keysCeremonyId={}, is_valid={}, signing_request={:?}",
        input.election_event_id,
        input.keys_ceremony_id,
        is_valid,
        signing_request.as_ref().map(|request| request.id),
    );

    Ok(Json(ConfirmKeyCustodyOutput {
        is_valid,
        signing_request,
    }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /custody-proof-signature-status
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct CustodyProofSignatureStatusInput {
    election_event_id: String,
    /// The keys ceremony, for confirming a custody proof.
    #[serde(default)]
    keys_ceremony_id: Option<String>,
    /// The tally session, for contributing a custody proof.
    #[serde(default)]
    tally_session_id: Option<String>,
}

/// Whether the signed-in trustee's key step needs their signature, and
/// whether they already have a used signed step for it.
#[instrument(skip(claims, services))]
#[post("/custody-proof-signature-status", format = "json", data = "<body>")]
pub async fn custody_proof_signature_status(
    body: Json<CustodyProofSignatureStatusInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<CustodyProofSignatureStatus>, Guarded<(Status, String)>> {
    // A signing route: a missing permission is 403 with the signing error body.
    authorize_403(
        &claims,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let input = body.into_inner();
    let (kind, target_id) =
        match (&input.keys_ceremony_id, &input.tally_session_id) {
            (Some(id), None) => (CustodyProofKind::Confirm, id),
            (None, Some(id)) => (CustodyProofKind::Contribute, id),
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
        windmill::services::signing::custody_proofs::custody_proof_signature_status(
            &hasura_transaction,
            &SigningCaller::from_claims(&claims),
            &claims.hasura_claims.tenant_id,
            &input.election_event_id,
            kind,
            target_id,
        )
        .await
        .map_err(custody_proof_failure)?;
    Ok(Json(status))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /get-custody-request
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct GetCustodyRequestInput {
    election_event_id: String,
    keys_ceremony_id: String,
    #[serde(default)]
    tally_session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetCustodyRequestOutput {
    custody_request_json: String,
}

fn custody_request_download_unavailable() -> JsonError {
    ErrorResponse::new(
        Status::Conflict,
        "Custody request download is no longer available",
        ErrorCode::CustodyRequestDownloadUnavailable,
    )
}

fn custody_request_download_internal_error() -> JsonError {
    ErrorResponse::new(
        Status::InternalServerError,
        "Failed to download custody request",
        ErrorCode::InternalServerError,
    )
}

fn custody_request_download_error(
    error: anyhow::Error,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> JsonError {
    if error
        .downcast_ref::<CustodyRequestDownloadUnavailable>()
        .is_some()
    {
        custody_request_download_unavailable()
    } else {
        error!(
            election_event_id = %election_event_id,
            keys_ceremony_id = %keys_ceremony_id,
            "Failed to download custody request: {error:#}"
        );
        custody_request_download_internal_error()
    }
}

// The trustee custody action
#[instrument(skip(claims))]
#[post("/get-custody-request", format = "json", data = "<body>")]
pub async fn get_custody_request(
    body: Json<GetCustodyRequestInput>,
    claims: JwtClaims,
) -> Result<Json<GetCustodyRequestOutput>, JsonError> {
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
            error!("Failed to get database client for custody request download: {error:#}");
            custody_request_download_internal_error()
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|error| {
            error!(
                "Failed to start custody request download transaction: {error:#}"
            );
            custody_request_download_internal_error()
        })?;
    audit::act_for(&hasura_transaction, &claims).await.map_err(
        |(status, message)| {
            ErrorResponse::new(status, &message, ErrorCode::InternalServerError)
        },
    )?;

    if let Some(tally_id) = &input.tally_session_id {
        authorize_tally_session_permission_labels(
            &hasura_transaction,
            &claims,
            &input.election_event_id,
            tally_id,
        )
        .await
        .map_err(|(status, message)| {
            ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
        })?;
    }

    let custody_request_json =
        if let Some(tally_id) = &input.tally_session_id {
            windmill::services::trustee_updates::tally_custody_file(
                &hasura_transaction,
                &claims,
                &input.election_event_id,
                &input.keys_ceremony_id,
                tally_id,
            )
            .await
        } else {
            keys_ceremony::get_custody_request(
                &hasura_transaction,
                claims,
                tenant_id,
                input.election_event_id.clone(),
                input.keys_ceremony_id.clone(),
            )
            .await
        }
        .map_err(|error| {
            custody_request_download_error(
                error,
                &input.election_event_id,
                &input.keys_ceremony_id,
            )
        })?;

    event!(
        Level::INFO,
        "get_custody_request: electionEventId={}, keysCeremonyId={}",
        input.election_event_id.clone(),
        input.keys_ceremony_id.clone(),
    );

    hasura_transaction.commit().await.map_err(|error| {
        error!(
            "Failed to commit custody request download transaction: {error:#}"
        );
        custody_request_download_internal_error()
    })?;
    kick_audit_log_outbox();

    Ok(Json(GetCustodyRequestOutput {
        custody_request_json,
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
    let user_id = claims.hasura_claims.user_id.clone();
    let user_permission_labels = claims.hasura_claims.permission_labels.clone();

    let username = claims.preferred_username.clone().unwrap_or("-".to_string());

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    audit::act_for(&hasura_transaction, &claims).await?;

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
    kick_audit_log_outbox();

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
#[path = "../../tests/support/custody_request_errors.rs"]
mod custody_request_errors;
