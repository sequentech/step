// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use crate::types::optional::OptionalId;
use anyhow::anyhow;
use anyhow::Result;
use deadpool_postgres::{Client as DbClient, Transaction};
use reqwest::StatusCode;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::{
    get_event_realm, get_tenant_realm, GroupInfo, KeycloakAdminClient,
};
use sequent_core::types::hasura;
use sequent_core::types::keycloak::User;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::format;
use std::iter::Map;
use std::str::FromStr;
use tracing::{info, instrument};
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::application::{
    confirm_application, get_group_names, reject_application,
    verify_application, ApplicationAnnotations, ApplicationVerificationResult,
};
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::electoral_log::ElectoralLog;
use windmill::tasks::send_template::send_template;
use windmill::types::application::{
    ApplicationStatus, ApplicationStatusUpdateEvent, ApplicationType,
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

    // Prepare log event to the electoral log
    let status_change = match result.application_status {
        ApplicationStatus::ACCEPTED => ApplicationStatusUpdateEvent {
            application_status: ApplicationStatus::ACCEPTED,
            application_type: ApplicationType::AUTOMATIC,
        },
        ApplicationStatus::REJECTED => ApplicationStatusUpdateEvent {
            application_status: ApplicationStatus::REJECTED,
            application_type: ApplicationType::AUTOMATIC,
        },
        ApplicationStatus::PENDING => ApplicationStatusUpdateEvent {
            application_status: ApplicationStatus::PENDING,
            application_type: ApplicationType::AUTOMATIC,
        },
    };

    info!("User id: {:?}", result.user_id);

    // The ACCEPTED ones should have the user_id of the applicant, the others
    // perhaps not when the application is rejected or pending, it depends on
    // the returned result from verify_application()
    let (user_id, username) = match result.user_id.clone() {
        Some(user_id) => {
            let username = get_user_name_from_keycloak(
                &input.tenant_id,
                &input.election_event_id,
                &user_id,
            )
            .await
            .map_err(|e| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("Error to get user name from Keycloak: {e:?}"),
                    ErrorCode::InternalServerError,
                )
            })?;
            (Some(user_id), username)
        }
        None => (None, None),
    };

    post_application_update_to_electoral_log(
        &hasura_transaction,
        &claims.hasura_claims.user_id,
        &input.tenant_id,
        &input.election_event_id,
        status_change,
        user_id,
        username,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error to post application update event: {e:?}"),
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
    let realm = get_tenant_realm(&input.tenant_id);
    let group_names = get_group_names(&realm, user_id).await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error getting group names: {:#?}", e),
            ErrorCode::InternalServerError,
        )
    })?;

    // Determine the action: Confirm or Reject
    let status_change: ApplicationStatusUpdateEvent = if input
        .rejection_reason
        .is_some()
    {
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
        ApplicationStatusUpdateEvent {
            application_status: ApplicationStatus::REJECTED,
            application_type: ApplicationType::MANUAL,
        }
    } else if input.rejection_reason.is_none() {
        // Confirmation logic
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
        ApplicationStatusUpdateEvent {
            application_status: ApplicationStatus::ACCEPTED,
            application_type: ApplicationType::MANUAL,
        }
    } else {
        return Err(JsonError::from(ErrorResponse::new(
            Status::BadRequest,
            "Invalid request: rejection_reason and rejection_message must either both be present or both absent",
            ErrorCode::InternalServerError,
        )));
    };

    info!("User id: {:?}", input.user_id);

    let (user_id, username) = match input.user_id.is_empty() {
        false => {
            let username = get_user_name_from_keycloak(
                &input.tenant_id,
                &input.election_event_id,
                &input.user_id,
            )
            .await
            .map_err(|e| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("Error to get user name from Keycloak: {e:?}"),
                    ErrorCode::InternalServerError,
                )
            })?;
            (Some(input.user_id.clone()), username)
        }
        true => (None, None),
    };

    post_application_update_to_electoral_log(
        &hasura_transaction,
        &claims.hasura_claims.user_id,
        &input.tenant_id,
        &input.election_event_id,
        status_change,
        user_id,
        username,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error to post application update event: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json("Success".to_string()))
}

#[instrument]
pub async fn get_user_name_from_keycloak(
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
) -> Result<Option<String>> {
    let realm = get_event_realm(tenant_id, election_event_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| anyhow!("error getting keycloak client: {e:?}"))?;

    let user = client
        .get_user(&realm, user_id)
        .await
        .map_err(|e| anyhow!("error getting user: {e:?}"))?;

    Ok(user.username)
}

#[instrument(skip(hasura_transaction))]
pub async fn post_application_update_to_electoral_log(
    hasura_transaction: &Transaction<'_>,
    hasura_user_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    status_change: ApplicationStatusUpdateEvent,
    user_id: Option<String>,
    username: Option<String>,
) -> Result<()> {
    let election_event = get_election_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(|e| anyhow!("error getting election event: {e:?}"))?;

    let board_name = get_election_event_board(
        election_event.bulletin_board_reference.clone(),
    )
    .ok_or(0)
    .map_err(|e| anyhow!("Error getting election event board"))?;

    let electoral_log =
        ElectoralLog::for_admin_user(&board_name, tenant_id, hasura_user_id)
            .await
            .map_err(|e| anyhow!("error getting electoral log: {e:?}"))?;

    let _ = electoral_log
        .post_application_status_update(
            status_change,
            election_event_id.to_string(),
            user_id,
            username,
        )
        .await
        .map_err(|e| anyhow!("error posting to the electoral log {e:?}"));

    Ok(())
}
