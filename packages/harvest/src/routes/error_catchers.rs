// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::Request;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::external::types::{
    DatafixErrorCode, DatafixResponse, JsonErrorResponse,
};

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

// Catchers scoped to `/api/datafix`: failures raised before a handler runs
// (request guards, body parsing) must still answer with the documented
// Datafix JSON error contract.

/// Missing/invalid Datafix headers or an unreadable request body.
#[instrument(skip_all)]
#[catch(400)]
pub fn datafix_invalid_request() -> JsonErrorResponse {
    DatafixResponse::error(DatafixErrorCode::InvalidRequest)
}

/// A request body that fails JSON deserialization surfaces as a 422; the
/// Datafix contract answers 400 `invalid-request` instead.
#[instrument(skip_all)]
#[catch(422)]
pub fn datafix_malformed_body() -> JsonErrorResponse {
    DatafixResponse::error(DatafixErrorCode::InvalidRequest)
}

/// Failed Datafix credentials surface as a 401; the contract answers 403
/// `forbidden`, the same as for a valid JWT lacking the required permission.
#[instrument(skip_all)]
#[catch(401)]
pub fn datafix_forbidden() -> JsonErrorResponse {
    DatafixResponse::error(DatafixErrorCode::Forbidden)
}
