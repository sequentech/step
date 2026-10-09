// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::access::document_extra_permissions;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::DocumentAnnotations;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::document::get_document;

#[derive(Deserialize, Debug)]
pub struct GetDocumentUrlBody {
    election_event_id: Option<String>,
    document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetDocumentUrlResponse {
    url: String,
}

/// The message of a missing document, with [`ErrorCode::DocumentNotFound`].
const DOCUMENT_NOT_FOUND: &str = "Document not found";

/// fetchDocument's error as JSON with a code, keeping the HTTP status:
/// Hasura forwards a JSON error's message and code to the portal also with
/// dev mode off, while it hides a plain-text body (R11 S4). Internal
/// details are logged, not answered.
pub(crate) fn fetch_document_error(
    (status, message): (Status, String),
) -> JsonError {
    if status == Status::NotFound {
        ErrorResponse::new(
            status,
            DOCUMENT_NOT_FOUND,
            ErrorCode::DocumentNotFound,
        )
    } else if status == Status::Unauthorized || status == Status::Forbidden {
        ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
    } else {
        tracing::error!("fetchDocument failed: {message}");
        ErrorResponse::new(
            status,
            "Could not fetch the document.",
            ErrorCode::InternalServerError,
        )
    }
}

#[instrument(skip(claims, services))]
#[post("/fetch-document", format = "json", data = "<body>")]
pub async fn fetch_document(
    body: Json<GetDocumentUrlBody>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<GetDocumentUrlResponse>, JsonError> {
    fetch_document_url(body, claims, services)
        .await
        .map_err(fetch_document_error)
}

async fn fetch_document_url(
    body: Json<GetDocumentUrlBody>,
    claims: JwtClaims,
    services: &HarvestServices,
) -> Result<Json<GetDocumentUrlResponse>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::DOCUMENT_DOWNLOAD],
    )?;

    let input = body.into_inner();

    let mut hasura_db_client: DbClient = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction = hasura_db_client.transaction().await.map_err(
        |err: tokio_postgres::Error| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        },
    )?;

    let document = get_document(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        input.election_event_id.clone(),
        &input.document_id,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error reading document: {error:?}"),
        )
    })?
    .ok_or_else(|| (Status::NotFound, DOCUMENT_NOT_FOUND.to_string()))?;
    let annotations = document
        .annotations
        .map(serde_json::from_value::<DocumentAnnotations>)
        .transpose()
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error reading document access policy: {error}"),
            )
        })?;
    let extra_permissions = document_extra_permissions(annotations.as_ref());
    if !extra_permissions.is_empty() {
        authorize(
            &claims,
            true,
            Some(claims.hasura_claims.tenant_id.clone()),
            extra_permissions,
        )?;
    }

    let url = services
        .documents
        .url(
            &hasura_transaction,
            &claims.hasura_claims.tenant_id,
            input.election_event_id.as_deref(),
            &input.document_id,
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
        .ok_or_else(|| (Status::NotFound, DOCUMENT_NOT_FOUND.to_string()))?;

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error committing transaction: {err}"),
        )
    })?;

    Ok(Json(GetDocumentUrlResponse { url }))
}

#[cfg(test)]
#[path = "../../tests/support/fetch_document_errors.rs"]
mod tests;

#[cfg(test)]
#[path = "../../tests/support/fetch_document_routes.rs"]
mod route_tests;
