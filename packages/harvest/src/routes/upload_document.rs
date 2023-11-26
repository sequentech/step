// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::s3;

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentInput {
    path: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentOutput {
    url: String,
}

#[instrument]
#[post("/upload_document", format = "json", data = "<body>")]
pub async fn upload_document(
    body: Json<UploadDocumentInput>,
) -> Result<Json<UploadDocumentOutput>, (Status, String)> {
    let inner = body.into_inner();
    let url = s3::get_upload_url(inner.path).await.unwrap();
    Ok(Json(UploadDocumentOutput { url: url }))
}
