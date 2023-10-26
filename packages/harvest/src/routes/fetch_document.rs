// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use sequent_core::services::connection;
use tracing::instrument;
use windmill::hasura;

use crate::s3;

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

    let _document = &document_result
        .data
        .expect("expected data".into())
        .sequent_backend_document[0];

    let document_s3_key = s3::get_document_key(
        input.tenant_id,
        input.election_event_id,
        input.document_id,
    );
    let url = s3::get_document_url(document_s3_key).await?;

    Ok(Json(GetDocumentUrlResponse { url: url }))
}
