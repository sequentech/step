// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use crate::types::optional::OptionalId;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::Value;
use tracing::instrument;

#[derive(Deserialize, Debug)]
pub struct ApplicationVerifyBody {
    tenant_id: String,
    election_event_id: String,
    applicant_id: String,
    applicant_data: Value,
    applicant_search_attributes: String,
}

#[instrument(skip(claims))]
#[post("/verify-application", format = "json", data = "<body>")]
pub async fn verify_application(
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

    Ok(Json("Success".to_string()))
}

#[derive(Deserialize, Debug)]
pub struct ApplicationConfirmationBody {
    tenant_id: String,
    election_event_id: String,
    id: String,
    user_id: String,
}

#[instrument(skip(claims))]
#[post("/confirm-application", format = "json", data = "<body>")]
pub async fn confirm_application(
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

    Ok(Json("Success".to_string()))
}
