// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::decode_permission_labels;
use sequent_core::types::ceremonies::TallyResolution;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::ceremonies::{
    RestorePrivateKeyOutcome, TallyExecutionStatus,
};
use sequent_core::types::permissions::Permissions;
use sequent_core::{
    services::jwt::JwtClaims, types::hasura::core::TallySessionConfiguration,
};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::postgres::tally_session::get_tally_session_by_id;
use windmill::services::celery_app::get_celery_app;
use windmill::services::ceremonies::tally_ceremony::{self};
use windmill::services::ceremonies::tally_resolution;
use windmill::services::ceremonies::tally_validation::TallyValidationError;
use windmill::services::database::get_hasura_pool;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
use windmill::tasks::execute_tally_session::execute_tally_session;

fn tally_response_error((status, message): (Status, String)) -> JsonError {
    let code = if status == Status::BadRequest {
        ErrorCode::TallyValidation
    } else if status == Status::Unauthorized || status == Status::Forbidden {
        ErrorCode::Unauthorized
    } else {
        tracing::error!("Tally request failed: {message}");
        return ErrorResponse::new(
            status,
            "Could not complete the tally operation.",
            ErrorCode::InternalServerError,
        );
    };
    ErrorResponse::new(status, &message, code)
}

fn tally_service_error(error: anyhow::Error) -> (Status, String) {
    if let Some(validation) = error.downcast_ref::<TallyValidationError>() {
        (Status::BadRequest, validation.to_string())
    } else {
        tracing::error!("Tally operation failed: {error:?}");
        (
            Status::InternalServerError,
            "Could not complete the tally operation.".into(),
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallyCeremonyInput {
    election_event_id: String,
    election_ids: Vec<String>,
    configuration: Option<TallySessionConfiguration>,
    tally_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallyCeremonyOutput {
    tally_session_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/create-tally-ceremony", format = "json", data = "<body>")]
pub async fn create_tally_ceremony(
    body: Json<CreateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, JsonError> {
    create_tally_ceremony_response(body, claims)
        .await
        .map_err(tally_response_error)
}

async fn create_tally_ceremony_response(
    body: Json<CreateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id: String = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.clone().hasura_claims.user_id;
    let username = claims
        .clone()
        .preferred_username
        .unwrap_or(claims.name.clone().unwrap_or_else(|| user_id.clone()));
    let permission_labels = decode_permission_labels(&claims);

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let tally_session_id = tally_ceremony::create_tally_ceremony(
        &hasura_transaction,
        tenant_id,
        &user_id,
        input.election_event_id.clone(),
        input.election_ids,
        input.configuration,
        input.tally_type.clone(),
        &permission_labels,
        username,
    )
    .await
    .map_err(tally_service_error)?;

    let _commit = hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;
    event!(
        Level::INFO,
        "Created Tally Ceremony, type={}, electionEventId={}, tallySessionId={}",
        input.tally_type,
        input.election_event_id,
        tally_session_id,
    );

    Ok(Json(CreateTallyCeremonyOutput { tally_session_id }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTallyCeremonyInput {
    election_event_id: String,
    tally_session_id: String,
    status: TallyExecutionStatus,
}

#[instrument(skip(claims))]
#[post("/update-tally-ceremony", format = "json", data = "<body>")]
pub async fn update_tally_ceremony(
    body: Json<UpdateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, JsonError> {
    update_tally_ceremony_response(body, claims)
        .await
        .map_err(tally_response_error)
}

async fn update_tally_ceremony_response(
    body: Json<UpdateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let user_id = claims.clone().hasura_claims.user_id;
    let username = claims
        .clone()
        .preferred_username
        .unwrap_or(claims.name.clone().unwrap_or_else(|| user_id.clone()));

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
    )
    .await
    .map_err(|_| {
        (
            Status::InternalServerError,
            format!(
                "Could not find tally session by id {}",
                input.election_event_id
            ),
        )
    })?;
    tally_ceremony::update_tally_ceremony(
        &hasura_transaction,
        tenant_id,
        input.election_event_id.clone(),
        tally_session.clone(),
        input.status.clone(),
        user_id.clone(),
        username.clone(),
    )
    .await
    .map_err(tally_service_error)?;

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    Ok(Json(CreateTallyCeremonyOutput {
        tally_session_id: input.tally_session_id.clone(),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RecountTallySessionInput {
    election_event_id: String,
    tally_session_id: String,
}

#[instrument(skip(claims))]
#[post("/recount-tally-session", format = "json", data = "<body>")]
pub async fn recount_tally_session(
    body: Json<RecountTallySessionInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_RECOUNT_EXECUTE],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
    )
    .await
    .map_err(|_| {
        (
            Status::NotFound,
            format!(
                "Could not find tally session by id {}",
                input.tally_session_id
            ),
        )
    })?;

    if tally_session.execution_status.as_deref()
        != Some(TallyExecutionStatus::SUCCESS.to_string().as_str())
        || !tally_session.is_execution_completed
    {
        return Err((
            Status::BadRequest,
            "Only completed tally sessions can be recounted".to_string(),
        ));
    }

    let election_ids = tally_session.election_ids.clone().unwrap_or_default();
    let recount_started = tally_ceremony::begin_tally_session_recount(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &election_ids,
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error starting tally session recount: {err:?}"),
        )
    })?;
    if !recount_started {
        return Err((
            Status::Conflict,
            "Only a completed tally session with execution history can be recounted".to_string(),
        ));
    }

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(execute_tally_session::new(
            tenant_id.clone(),
            input.election_event_id.clone(),
            input.tally_session_id.clone(),
            tally_session.tally_type.clone(),
            tally_session.election_ids.clone(),
            true, // force_new_results_id: manual recount always produces a fresh results event
        ))
        .await;

    match task {
        Ok(_) => event!(
            Level::INFO,
            "Sent recount tally task for election_event_id={}, tally_session_id={}",
            input.election_event_id,
            input.tally_session_id,
        ),
        Err(err) => event!(
            Level::ERROR,
            "Recount request is durable but its immediate task nudge failed for \
             election_event_id={}, tally_session_id={}; process_board will retry: {err:?}",
            input.election_event_id,
            input.tally_session_id,
        ),
    }

    Ok(Json(CreateTallyCeremonyOutput {
        tally_session_id: input.tally_session_id,
    }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /restore-private-key
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct SetPrivateKeyInput {
    election_event_id: String,
    private_key_base64: String,
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetPrivateKeyOutput {
    is_valid: bool,
    outcome: RestorePrivateKeyOutcome,
}

// The main function to restore the private key
#[instrument(skip(body, claims))]
#[post("/restore-private-key", format = "json", data = "<body>")]
pub async fn restore_private_key(
    body: Json<SetPrivateKeyInput>,
    claims: JwtClaims,
) -> Result<Json<SetPrivateKeyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let outcome = tally_ceremony::set_private_key(
        &hasura_transaction,
        &claims,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &input.private_key_base64,
    )
    .await
    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "Restoring given private key, election_event_id={}, tally_session_id={}, outcome={:?}",
        input.election_event_id,
        input.tally_session_id,
        outcome,
    );

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;
    Ok(Json(SetPrivateKeyOutput {
        is_valid: outcome != RestorePrivateKeyOutcome::Invalid,
        outcome,
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitTallyResolutionInput {
    election_event_id: String,
    tally_session_id: String,
    resolutions: Vec<TallyResolution>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitTallyResolutionOutput {
    success: bool,
    tally_session_id: String,
    resolved_count: usize,
}

/// Submit multiple tally resolutions for a paused tally (batch operation)
#[instrument(skip(claims))]
#[post("/submit-tally-resolution", format = "json", data = "<body>")]
pub async fn submit_tally_resolution(
    body: Json<SubmitTallyResolutionInput>,
    claims: JwtClaims,
) -> Result<Json<SubmitTallyResolutionOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_RESOLUTION_SUBMIT],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();

    if input.resolutions.is_empty() {
        return Err((
            Status::BadRequest,
            "At least one resolution required".to_string(),
        ));
    }

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let resolved_count = tally_resolution::submit_tally_resolution(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &input.resolutions,
        &user_id,
        claims.preferred_username.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    event!(
        Level::INFO,
        "Batch tally resolution submission completed for tally session {}, resolved {} contest(s)",
        input.tally_session_id,
        resolved_count
    );

    Ok(Json(SubmitTallyResolutionOutput {
        success: true,
        tally_session_id: input.tally_session_id,
        resolved_count,
    }))
}

#[cfg(test)]
mod tally_error_tests {
    use super::*;
    use rocket::http::ContentType;
    use rocket::local::asynchronous::Client;

    #[get("/tally-validation-error")]
    fn invalid_tally() -> JsonError {
        tally_response_error(tally_service_error(
            TallyValidationError::new(
                "Election selected: end its voting period before tallying.",
            )
            .into(),
        ))
    }

    #[rocket::async_test]
    async fn validation_response_has_the_json_contract_required_by_hasura() {
        let client =
            Client::tracked(rocket::build().mount("/", routes![invalid_tally]))
                .await
                .unwrap();
        let response = client.get("/tally-validation-error").dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(response.content_type(), Some(ContentType::JSON));
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(
            body["message"],
            "Election selected: end its voting period before tallying."
        );
        assert_eq!(body["extensions"]["code"], "TallyValidation");
    }

    #[test]
    fn internal_errors_do_not_expose_service_diagnostics() {
        let response = tally_response_error((
            Status::InternalServerError,
            "private database details".into(),
        ));
        assert_eq!(response.0, Status::InternalServerError);
        assert_eq!(
            response.1.message,
            "Could not complete the tally operation."
        );
        assert_eq!(response.1.extensions.code, "InternalServerError");
    }
}
