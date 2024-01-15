// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::documents;

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentInput {
    name: String,
    media_type: String,
    size: usize,
    is_public: bool,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentOutput {
    document_id: String,
    url: String,
}

#[instrument(skip(claims))]
#[post("/get-upload-url", format = "json", data = "<body>")]
pub async fn upload_document(
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
    let auth_headers = keycloak::get_client_credentials()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let (document, url) = documents::get_upload_url(
        auth_headers,
        &inner.name,
        &inner.media_type,
        inner.size,
        &claims.hasura_claims.tenant_id,
        &inner.is_public,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(UploadDocumentOutput {
        document_id: document.id,
        url: url,
    }))
}
