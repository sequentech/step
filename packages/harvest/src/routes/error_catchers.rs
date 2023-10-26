// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use rocket::Request;
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
pub fn not_found(req: &Request) -> Json<ErrorResponse> {
    Json(ErrorResponse {
        message: "Not found".into(),
    })
}

#[instrument(skip_all)]
#[catch(default)]
pub fn default(status: Status, req: &Request) -> Json<ErrorResponse> {
    Json(ErrorResponse {
        message: "Unknown Error".into(),
    })
}
