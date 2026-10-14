// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::iter::Map;
use std::str::FromStr;

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::signing_gate::{caller, waiting, Guarded};
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use crate::types::optional::OptionalId;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use reqwest::StatusCode;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::{
    get_event_realm, get_tenant_realm, GroupInfo, KeycloakAdminClient,
};
use sequent_core::types::keycloak::User;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;
use windmill::services::application::{
    confirm_application, get_group_names, reject_application,
    verify_application, ApplicationAnnotations, ApplicationVerificationResult,
};
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::signing::actions::voter::{
    cancel_for_rejection, gate_voter_approval,
};
use windmill::services::signing::guard::SigningRequestSummary;
use windmill::services::users::check_is_user_verified;
use windmill::tasks::send_template::send_template;
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;
use windmill::types::application::{
    ApplicationStatus, ApplicationType, ApplicationsError,
};

#[derive(Deserialize, Debug)]
pub struct ApplicationVerifyBody {
    applicant_id: String,
    applicant_data: HashMap<String, String>,
    tenant_id: String,
    election_event_id: String,
    area_id: Option<String>,
    labels: Option<Value>,
    annotations: ApplicationAnnotations,
}

#[instrument(skip(claims))]
#[post("/verify-application", format = "json", data = "<body>")]
pub async fn verify_user_application(
    claims: jwt::JwtClaims,
    body: Json<ApplicationVerifyBody>,
) -> Result<Json<ApplicationVerificationResult>, JsonError> {
    let input: ApplicationVerifyBody = body.into_inner();

    info!("Verifiying application");

    let required_perm: Permissions = Permissions::SERVICE_ACCOUNT;

    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{:?}", e),
            ErrorCode::Unauthorized,
        )
    })?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{:?}", e),
                ErrorCode::GetTransactionFailed,
            )
        })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{:?}", e),
                ErrorCode::GetTransactionFailed,
            )
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{:?}", e),
                ErrorCode::GetTransactionFailed,
            )
        })?;

    let result = verify_application(
        &hasura_transaction,
        &keycloak_transaction,
        &input.applicant_id,
        &input.applicant_data,
        &input.tenant_id,
        &input.election_event_id,
        &input.labels,
        &input.annotations,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("{:?}", e),
            ErrorCode::InternalServerError,
        )
    })?;

    let _commit = hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(result))
}

#[derive(Deserialize, Debug)]
pub struct ApplicationChangeStatusBody {
    tenant_id: String,
    election_event_id: String,
    area_id: Option<String>,
    id: String,
    user_id: String,
    rejection_reason: Option<String>, // Optional for rejection
    rejection_message: Option<String>, // Optional for rejection
}

#[derive(Serialize, Debug)]
pub struct ApplicationChangeStatusOutput {
    message: Option<String>,
    error: Option<String>,
    /// While approving the voter waits for signatures.
    #[serde(skip_serializing_if = "Option::is_none")]
    signing_request: Option<SigningRequestSummary>,
}

fn internal(message: String) -> JsonError {
    JsonError::from(ErrorResponse::new(
        Status::InternalServerError,
        &message,
        ErrorCode::InternalServerError,
    ))
}

async fn group_names_of(
    tenant_id: &str,
    user_id: &str,
) -> Result<Vec<String>, JsonError> {
    get_group_names(&get_tenant_realm(tenant_id), user_id)
        .await
        .map_err(|e| internal(format!("Error getting group names: {:#?}", e)))
}

#[instrument(skip(claims, services))]
#[post("/change-application-status", format = "json", data = "<body>")]
pub async fn change_application_status(
    claims: jwt::JwtClaims,
    body: Json<ApplicationChangeStatusBody>,
    services: &State<HarvestServices>,
) -> Result<Json<ApplicationChangeStatusOutput>, Guarded<JsonError>> {
    let input = body.into_inner();

    info!("Changing application status: {input:?}");

    let required_perm: Permissions = Permissions::APPLICATION_WRITE;
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{:?}", e),
            ErrorCode::Unauthorized,
        )
    })?;

    let mut hasura_db_client: DbClient =
        services.databases.hasura().await.get().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error obtaining hasura pool: {:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error obtaining transaction: {:?}", e),
                ErrorCode::GetTransactionFailed,
            )
        })?;

    let user_id = &claims.hasura_claims.user_id;
    let admin_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Determine the action: Confirm or Reject
    if input.rejection_reason.is_some() {
        // A rejected application's approval no longer waits for signatures.
        cancel_for_rejection(
            &hasura_transaction,
            &caller(&claims),
            &input.tenant_id,
            &input.election_event_id,
            &input.id,
        )
        .await?;
        let group_names = group_names_of(&input.tenant_id, user_id).await?;
        // Rejection logic
        reject_application(
            &hasura_transaction,
            &input.id,
            &input.tenant_id,
            &input.election_event_id,
            &input.user_id,
            &claims.hasura_claims.user_id,
            input.rejection_reason,
            input.rejection_message,
            &admin_name,
            &group_names,
        )
        .await
        .map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error rejecting application: {:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;
    } else {
        let mut keycloak_db_client: DbClient = services
            .databases
            .keycloak()
            .await
            .get()
            .await
            .map_err(|e| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("{:?}", e),
                    ErrorCode::GetTransactionFailed,
                )
            })?;
        let keycloak_transaction =
            keycloak_db_client.transaction().await.map_err(|e| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("{:?}", e),
                    ErrorCode::GetTransactionFailed,
                )
            })?;

        let realm = get_event_realm(&input.tenant_id, &input.election_event_id);

        let is_user_verified = check_is_user_verified(
            &keycloak_transaction,
            &realm,
            &input.user_id,
        )
        .await
        .map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error in check_is_user_verified: {:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;

        if is_user_verified {
            return Ok(Json(ApplicationChangeStatusOutput {
                message: None,
                error: Some(ApplicationsError::APPROVED_VOTER.to_string()),
                signing_request: None,
            }));
        }

        // Approving a voter manually may need signatures first.
        let outcome = gate_voter_approval(
            &hasura_transaction,
            &keycloak_transaction,
            &caller(&claims),
            &input.tenant_id,
            &input.election_event_id,
            &input.id,
            &input.user_id,
        )
        .await?;
        if let Some(signing_request) = waiting(outcome) {
            hasura_transaction
                .commit()
                .await
                .map_err(|e| internal(format!("Commit failed: {e:?}")))?;
            kick_signing_log_outbox();
            return Ok(Json(ApplicationChangeStatusOutput {
                message: None,
                error: None,
                signing_request: Some(signing_request),
            }));
        }

        let group_names = group_names_of(&input.tenant_id, user_id).await?;
        //Confirmation logic
        confirm_application(
            &hasura_transaction,
            &input.id,
            &input.tenant_id,
            &input.election_event_id,
            &input.user_id,
            &claims.hasura_claims.user_id,
            &admin_name,
            &group_names,
        )
        .await
        .map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error confirming application: {:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;
    };

    hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(ApplicationChangeStatusOutput {
        message: Some("Success".to_string()),
        error: None,
        signing_request: None,
    }))
}
