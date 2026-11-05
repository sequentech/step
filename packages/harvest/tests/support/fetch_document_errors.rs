// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! fetchDocument answers JSON errors with a code, so the portals can tell a
//! missing document from a failure that may pass also when Hasura hides
//! the response body (dev mode off).

use super::fetch_document_error;
use crate::types::error_response::ErrorCode;
use rocket::http::Status;
use serde_json::json;

fn body(status: Status, message: &str) -> (Status, serde_json::Value) {
    let response = fetch_document_error((status, message.to_string()));
    (
        response.0,
        serde_json::to_value(response.1.into_inner()).unwrap(),
    )
}

#[test]
fn a_missing_document_keeps_its_status_and_gets_its_own_code() {
    assert_eq!(
        body(Status::NotFound, "Document not found"),
        (
            Status::NotFound,
            json!({"message": "Document not found", "extensions": {"code": "DocumentNotFound"}})
        )
    );
    assert_eq!(ErrorCode::DocumentNotFound.as_ref(), "DocumentNotFound");
}

#[test]
fn a_refused_permission_keeps_its_status_and_message() {
    for status in [Status::Unauthorized, Status::Forbidden] {
        assert_eq!(
            body(status, "Missing permission"),
            (
                status,
                json!({"message": "Missing permission", "extensions": {"code": "Unauthorized"}})
            )
        );
    }
}

#[test]
fn other_failures_keep_their_status_without_internal_details() {
    let (status, value) = body(
        Status::InternalServerError,
        "Error reading document: db secret",
    );
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(value["extensions"]["code"], "InternalServerError");
    assert!(!value["message"].as_str().unwrap().contains("db secret"));
}
