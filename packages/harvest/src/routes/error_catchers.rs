// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::{json, Json, Value};
use rocket::serde::{Deserialize, Serialize};
use rocket::Request;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
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
