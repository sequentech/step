// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::str::FromStr;

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use crate::types::optional::OptionalId;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::Value;
use tracing::instrument;
use windmill::services::application::{
    confirm_application, verify_application,
};
use windmill::services::database::get_hasura_pool;
use windmill::types::application::{ApplicationStatus, ApplicationType};

#[derive(Deserialize, Debug)]
pub struct ApplicationVerifyBody {
    applicant_id: String,
    applicant_data: Value,
    tenant_id: String,
    election_event_id: String,
    area_id: String,
    labels: Option<Value>,
    annotations: Option<Value>,
}

#[instrument(skip(claims))]
#[post("/verify-application", format = "json", data = "<body>")]
pub async fn verify_user_application(
    claims: jwt::JwtClaims,
    body: Json<ApplicationVerifyBody>,
) -> Result<Json<String>, JsonError> {
    let input = body.into_inner();

    info!("Verifiying application: {input:?}");

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

    verify_application(
        &hasura_transaction,
        &input.applicant_id,
        &input.applicant_data,
        &input.tenant_id,
        &input.election_event_id,
        &input.area_id,
        &input.labels,
        &input.annotations,
    ).await.map_err(|e| {
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

    Ok(Json("Success".to_string()))
}

#[derive(Deserialize, Debug)]
pub struct ApplicationConfirmationBody {
    tenant_id: String,
    election_event_id: String,
    area_id: String,
    id: String,
    user_id: String,
}

#[instrument(skip(claims))]
#[post("/confirm-application", format = "json", data = "<body>")]
pub async fn confirm_user_application(
    claims: jwt::JwtClaims,
    body: Json<ApplicationConfirmationBody>,
) -> Result<Json<String>, JsonError> {
    let input = body.into_inner();

    info!("Confirming application: {input:?}");

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

    confirm_application(&hasura_transaction, input.id, input.tenant_id, input.election_event_id, input.area_id, input.user_id).await.map_err(|e| {
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

    Ok(Json("Success".to_string()))
}
