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
use reqwest::StatusCode;
use rocket::http::Status;
use rocket::serde::json::Json;
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
use windmill::services::users::check_is_user_verified;
use windmill::tasks::send_template::send_template;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationChangeStatusOutput {
    message: Option<String>,
    error: Option<String>,
}

#[instrument(skip(claims))]
#[post("/change-application-status", format = "json", data = "<body>")]
pub async fn change_application_status(
    claims: jwt::JwtClaims,
    body: Json<ApplicationChangeStatusBody>,
) -> Result<Json<ApplicationChangeStatusOutput>, JsonError> {
    let input = body.into_inner();

    info!("Changing application status: {input:?}");
    info!("claims::: {:?}", &claims);

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

    let user_id = &claims.hasura_claims.user_id;
    let tenant_realm = get_tenant_realm(&input.tenant_id);
    let group_names =
        get_group_names(&tenant_realm, user_id).await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error getting group names: {:#?}", e),
                ErrorCode::InternalServerError,
            )
        })?;

    // Determine the action: Confirm or Reject
    if input.rejection_reason.is_some() {
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
            &claims
                .name
                .clone()
                .unwrap_or_else(|| claims.hasura_claims.user_id.clone()),
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
    } else if input.rejection_reason.is_none() {
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
            }));
        }
        //Confirmation logic
        confirm_application(
            &hasura_transaction,
            &input.id,
            &input.tenant_id,
            &input.election_event_id,
            &input.user_id,
            &claims.hasura_claims.user_id,
            &claims
                .name
                .clone()
                .unwrap_or_else(|| claims.hasura_claims.user_id.clone()),
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

    Ok(Json(ApplicationChangeStatusOutput {
        message: Some("Success".to_string()),
        error: None,
    }))
}
