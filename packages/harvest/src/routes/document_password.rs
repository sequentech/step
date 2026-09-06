// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::DocumentAnnotations;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
use windmill::postgres::document::get_document;
use windmill::services::database::get_hasura_pool;
use windmill::services::document_password::read_password;

#[derive(Debug, Deserialize)]
pub struct GetDocumentPasswordInput {
    document_id: String,
}

#[derive(Serialize)]
pub struct GetDocumentPasswordOutput {
    password: String,
}

fn unavailable() -> JsonError {
    ErrorResponse::new(
        Status::NotFound,
        "Document password is not available",
        ErrorCode::DocumentPasswordUnavailable,
    )
}

fn internal_error(message: &str) -> JsonError {
    ErrorResponse::new(
        Status::InternalServerError,
        message,
        ErrorCode::InternalServerError,
    )
}

#[instrument(skip_all)]
#[post("/get-document-password", format = "json", data = "<input>")]
pub async fn get_document_password(
    claims: JwtClaims,
    input: Json<GetDocumentPasswordInput>,
) -> Result<Json<GetDocumentPasswordOutput>, JsonError> {
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![
            Permissions::DOCUMENT_DOWNLOAD,
            Permissions::DOCUMENT_PASSWORD_READ,
        ],
    )
    .map_err(|_| {
        ErrorResponse::new(
            Status::Forbidden,
            "Authorization failed",
            ErrorCode::Unauthorized,
        )
    })?;

    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|_| internal_error("Failed to get database client"))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|_| internal_error("Failed to start database transaction"))?;
    let document = get_document(&transaction, &tenant_id, None, &input.document_id)
        .await
        .map_err(|error| {
            error!(document_id = %input.document_id, "Failed to read document: {error:#}");
            internal_error("Failed to read document")
        })?
        .ok_or_else(unavailable)?;
    let annotations = document
        .annotations
        .map(serde_json::from_value::<DocumentAnnotations>)
        .transpose()
        .map_err(|error| {
            error!(document_id = %document.id, "Invalid document access annotations: {error:#}");
            internal_error("Failed to read document access metadata")
        })?
        .ok_or_else(unavailable)?;
    if annotations.requires_voter_secret_attribute_read() {
        authorize(
            &claims,
            true,
            Some(tenant_id.clone()),
            vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
        )
        .map_err(|_| {
            ErrorResponse::new(
                Status::Forbidden,
                "Authorization failed",
                ErrorCode::Unauthorized,
            )
        })?;
    }
    let password_secret_id =
        annotations.password_secret_id().ok_or_else(unavailable)?;
    let secret = read_password(
        &transaction,
        &tenant_id,
        document.election_event_id.as_deref(),
        &document.id,
        password_secret_id,
    )
    .await
    .map_err(|error| {
        error!(document_id = %document.id, "Failed to retrieve document password: {error:#}");
        internal_error("Failed to retrieve document password")
    })?
    .ok_or_else(unavailable)?;

    transaction
        .commit()
        .await
        .map_err(|_| internal_error("Failed to finish password retrieval"))?;

    Ok(Json(GetDocumentPasswordOutput {
        password: secret.password,
    }))
}
