// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::s3;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentInput {
    name: String,
    media_type: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentOutput {
    url: String,
}

#[instrument(skip(claims))]
#[post("/upload_document", format = "json", data = "<body>")]
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
    document_upload::get_upload_url(inner.name, inner.media_type, claims.hasura_claims.tenant_id)
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(UploadDocumentOutput { url: url }))
}
