// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::{
    database::get_hasura_pool, documents,
    providers::transactions_provider::provide_hasura_transaction,
};
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentInput {
    name: String,
    media_type: String,
    size: usize,
    is_public: bool,
    is_local: Option<bool>,
    election_event_id: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentOutput {
    document_id: String,
    url: String,
}

#[instrument(skip(claims))]
#[post("/get-upload-url", format = "json", data = "<body>")]
pub async fn get_upload_url(
    claims: JwtClaims,
    body: Json<UploadDocumentInput>,
) -> Result<Json<UploadDocumentOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::DOCUMENT_UPLOAD],
    )?;

    let inner = body.into_inner();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error loading hasura db client: {err}"),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error creating a transaction: {err}"),
            )
        })?;

    let (document, url) = documents::get_upload_url(
        &hasura_transaction,
        &inner.name,
        &inner.media_type,
        inner.size,
        &claims.hasura_claims.tenant_id,
        inner.is_public,
        inner.is_local,
        inner.election_event_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Failed to commit Hasura transactionn: {err}"),
        )
    })?;

    Ok(Json(UploadDocumentOutput {
        document_id: document.id,
        url,
    }))
}
