// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use windmill::services::s3;
use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::hasura;

#[derive(Deserialize, Debug)]
pub struct GetDocumentUrlBody {
    tenant_id: String,
    election_event_id: String,
    document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetDocumentUrlResponse {
    url: String,
}

#[instrument]
#[post("/fetch-document", format = "json", data = "<body>")]
pub async fn fetch_document(
    body: Json<GetDocumentUrlBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<GetDocumentUrlResponse>, Debug<anyhow::Error>> {
    let input = body.into_inner();
    let document_result = hasura::document::find_document(
        auth_headers,
        input.tenant_id.clone(),
        input.election_event_id.clone(),
        input.document_id.clone(),
    )
    .await?;

    let document = &document_result
        .data
        .expect("expected data".into())
        .sequent_backend_document[0];

    let document_s3_key = s3::get_document_key(
        input.tenant_id,
        input.election_event_id,
        input.document_id,
    );
    let bucket = if document.is_public.unwrap_or(false) {
        s3::get_public_bucket()
    } else {
        s3::get_private_bucket()
    };
    let url = s3::get_document_url(document_s3_key, bucket).await?;

    Ok(Json(GetDocumentUrlResponse { url: url }))
}
