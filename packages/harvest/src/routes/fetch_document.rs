// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::documents;

#[derive(Deserialize, Debug)]
pub struct GetDocumentUrlBody {
    election_event_id: String,
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
        vec![Permissions::DOCUMENT_UPLOAD],
    )?;
    let input = body.into_inner();
    let url = documents::fetch_document(
        claims.hasura_claims.tenant_id.clone(),
        input.election_event_id.clone(),
        input.document_id.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(GetDocumentUrlResponse { url: url }))
}
