// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::iter::Map;
use std::str::FromStr;

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use crate::types::optional::OptionalId;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::keycloak::User;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::Value;
use tracing::instrument;
use windmill::services::application::{
    confirm_application, reject_application, verify_application,
    ApplicationVerificationResult,
};
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::tasks::send_template::send_template;
use windmill::types::application::{ApplicationStatus, ApplicationType};

#[derive(Deserialize, Debug)]
pub struct ApplicationVerifyBody {
    applicant_id: String,
    applicant_data: Value,
    tenant_id: String,
    election_event_id: String,
    area_id: Option<String>,
    labels: Option<Value>,
    annotations: Option<Value>,
}

#[instrument(skip(claims))]
#[post("/verify-application", format = "json", data = "<body>")]
pub async fn verify_user_application(
    claims: jwt::JwtClaims,
    body: Json<ApplicationVerifyBody>,
) -> Result<Json<ApplicationVerificationResult>, JsonError> {
    let input = body.into_inner();

    info!("Verifiying application: {input:?}");

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
        &None,
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

#[instrument(skip(claims))]
#[post("/change-application-status", format = "json", data = "<body>")]
pub async fn change_application_status(
    claims: jwt::JwtClaims,
    body: Json<ApplicationChangeStatusBody>,
) -> Result<Json<String>, JsonError> {
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
        get_hasura_pool().await.get().await.map_err(|e| {
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

    // Determine the action: Confirm or Reject
    let action_result = if input.rejection_reason.is_some() {
        // Rejection logic
        reject_application(
            &hasura_transaction,
            &input.id,
            &input.tenant_id,
            &input.election_event_id,
            &input.user_id,
            input.rejection_reason.as_ref().unwrap(),
            input.rejection_message.as_ref().unwrap(),
            &claims.hasura_claims.user_id,
        )
        .await
        .map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error rejecting application: {:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;
    } else if input.rejection_reason.is_none() {
        // Confirmation logic
        confirm_application(
            &hasura_transaction,
            &input.id,
            &input.tenant_id,
            &input.election_event_id,
            &input.user_id,
            &claims.hasura_claims.user_id,
        )
        .await
        .map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error confirming application: {:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;
    } else {
        return Err(JsonError::from(ErrorResponse::new(
            Status::BadRequest,
            "Invalid request: rejection_reason and rejection_message must either both be present or both absent",
            ErrorCode::InternalServerError,
        )));
    };

    hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json("Success".to_string()))
}
