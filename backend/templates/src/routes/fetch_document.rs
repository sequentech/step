
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::response::Debug;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use rocket::serde::{Deserialize, Serialize};
use serde_json::json;

use crate::connection;
use crate::s3;
use crate::hasura;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct GetDocumentUrlBody {
    tenant_id: String,
    election_event_id: String,
    document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct GetDocumentUrlResponse {
    url: String,
}

#[post("/fetch-document", format = "json", data = "<body>")]
pub async fn fetch_document(
    body: Json<GetDocumentUrlBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<GetDocumentUrlResponse>, Debug<reqwest::Error>> {
    let input = body.into_inner();
    let document_result = hasura::document::find_document(
        auth_headers,
        input.tenant_id.clone(),
        input.election_event_id.clone(),
        input.document_id.clone(),
    );

    let document = &document_result
        .data
        .expect("expected data".into())
        .sequent_backend_document
        .unwrap()
        .returning[0];

    let document_s3_key = s3::get_document_key(
        input.tenant_id,
        input.election_event_id,
        input.document_id,
    );
    let url = s3::get_document_url(document_s3_key).await.unwrap();

    Ok(Json(GetDocumentUrlResponse { url: url }))
}