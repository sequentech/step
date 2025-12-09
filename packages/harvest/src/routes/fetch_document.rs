// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::{database::get_hasura_pool, documents};

#[derive(Deserialize, Debug)]
pub struct GetDocumentUrlBody {
    election_event_id: Option<String>,
    document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetDocumentUrlResponse {
    url: String,
}

#[instrument(skip(claims))]
#[post("/fetch-document", format = "json", data = "<body>")]
pub async fn fetch_document(
    body: Json<GetDocumentUrlBody>,
    claims: JwtClaims,
) -> Result<Json<GetDocumentUrlResponse>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::DOCUMENT_DOWNLOAD],
    )?;

    let input = body.into_inner();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
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

    let url = documents::get_document_url(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        input.election_event_id.as_deref(),
        &input.document_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .ok_or_else(|| (Status::NotFound, "Document not found".to_string()))?;

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error committing transaction: {err}"),
        )
    })?;

    Ok(Json(GetDocumentUrlResponse { url }))
}
