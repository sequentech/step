// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The portals translate structured error extensions. Preserve their public
//! shape, including omitted optional fields and truncated validation totals.

use crate::types::error_response::{ErrorCode, ErrorResponse};
use rocket::http::Status;
use sequent_core::services::keycloak::UserProfileValidationError;
use serde_json::json;

#[test]
fn ordinary_errors_keep_the_status_message_and_code_without_unrelated_extensions(
) {
    for (status, code, expected) in [
        (
            Status::Unauthorized,
            ErrorCode::Unauthorized,
            "Unauthorized",
        ),
        (
            Status::BadRequest,
            ErrorCode::DeserializeBallotFailed,
            "DeserializeBallotFailed",
        ),
        (
            Status::NotFound,
            ErrorCode::DocumentPasswordUnavailable,
            "DocumentPasswordUnavailable",
        ),
        (
            Status::InternalServerError,
            ErrorCode::InternalServerError,
            "InternalServerError",
        ),
    ] {
        let response =
            ErrorResponse::new(status, "A readable explanation", code);
        assert_eq!(response.0, status);
        assert_eq!(
            serde_json::to_value(response.1.into_inner()).unwrap(),
            json!({
                "message":"A readable explanation", "extensions":{"code":expected}
            })
        );
    }
}

#[test]
fn password_rules_preserve_the_required_count_for_client_side_explanations() {
    let response = ErrorResponse::password_policy_violation(
        Status::BadRequest,
        "More digits are required",
        "digits",
        2,
    );
    assert_eq!(response.0, Status::BadRequest);
    assert_eq!(
        serde_json::to_value(response.1.into_inner()).unwrap(),
        json!({
            "message":"More digits are required", "extensions":{
                "code":"PasswordPolicyViolation", "password_policy_rule":"digits", "password_policy_required_count":2
            }
        })
    );
}

#[test]
fn profile_validation_preserves_field_parameters_and_the_untruncated_total() {
    let errors = [
        UserProfileValidationError {
            field: Some("postalCode".into()),
            error_message: Some("error-length".into()),
            params: Some(vec![json!(3), json!(12)]),
        },
        UserProfileValidationError {
            field: None,
            error_message: None,
            params: None,
        },
    ];
    let response = ErrorResponse::user_profile_validation(
        Status::BadRequest,
        "Check these fields",
        &errors,
        9,
    );
    assert_eq!(response.0, Status::BadRequest);
    assert_eq!(
        serde_json::to_value(response.1.into_inner()).unwrap(),
        json!({
            "message":"Check these fields", "extensions":{
                "code":"UserProfileValidation", "user_profile_errors_total":9,
                "user_profile_errors":[
                    {"field":"postalCode","error":"error-length","params":[3,12]},
                    {"field":null,"error":null,"params":[]}
                ]
            }
        })
    );
    // Mapping the errors into a response must not consume or alter the source.
    assert_eq!(errors[0].params, Some(vec![json!(3), json!(12)]));
    assert!(errors[1].params.is_none());
}

#[test]
fn an_empty_profile_error_list_is_distinguishable_from_an_ordinary_error() {
    let response = ErrorResponse::user_profile_validation(
        Status::BadRequest,
        "Validation failed",
        &[],
        0,
    );
    assert_eq!(
        serde_json::to_value(response.1.into_inner()).unwrap(),
        json!({
            "message":"Validation failed", "extensions":{
                "code":"UserProfileValidation", "user_profile_errors_total":0, "user_profile_errors":[]
            }
        })
    );
}
