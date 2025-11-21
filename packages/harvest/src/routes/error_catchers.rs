// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::Request;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    message: String,
}

#[instrument]
#[catch(500)]
pub fn internal_error() -> Json<ErrorResponse> {
    Json(ErrorResponse {
        message: "Internal error".into(),
    })
}

#[instrument(skip_all)]
#[catch(404)]
pub fn not_found(_req: &Request) -> Json<ErrorResponse> {
    Json(ErrorResponse {
        message: "Not found".into(),
    })
}

#[instrument(skip_all)]
#[catch(default)]
pub fn default(_status: Status, _req: &Request) -> Json<ErrorResponse> {
    Json(ErrorResponse {
        message: "Unknown Error".into(),
    })
}
