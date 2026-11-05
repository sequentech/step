// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use keycloak::KeycloakError;
use serde_json::json;

#[test]
fn a_wrapped_keycloak_profile_rejection_has_structured_field_details() {
    let text = r#"{"field":"roll","errorMessage":"invalid-length","params":["roll",1,12]}"#;
    let error = anyhow::Error::new(KeycloakError::HttpFailure {
        status: 400,
        body: serde_json::from_str(text).ok(),
        text: text.into(),
    })
    .context("private Keycloak context");
    let response = keycloak_user_error(error, "Failed to create user");
    assert_eq!(response.0, Status::BadRequest);
    assert_eq!(
        serde_json::to_value(&response.1 .0).unwrap(),
        json!({
            "message": "Invalid value for \"roll\": invalid-length",
            "extensions": {"code": "UserProfileValidation", "user_profile_errors_total": 1,
                "user_profile_errors": [{"field": "roll", "error": "invalid-length", "params": ["roll",1,12]}]}
        })
    );
}

#[test]
fn untyped_user_errors_preserve_the_existing_context_and_debug_diagnostics() {
    let error = anyhow::anyhow!("Keycloak unavailable");
    let expected_debug = format!("{error:?}");
    let response = keycloak_user_error(error, "Failed to create user");
    assert_eq!(response.0, Status::InternalServerError);
    assert_eq!(
        response.1.message,
        format!("Failed to create user: {expected_debug}")
    );
    assert_eq!(response.1.extensions.code, "InternalServerError");
    assert!(response.1.extensions.user_profile_errors.is_none());
}

#[test]
fn incomplete_user_profile_errors_keep_fallback_labels_and_empty_arguments() {
    let response = user_profile_error(&[UserProfileValidationError {
        field: None,
        error_message: None,
        params: None,
    }]);
    assert_eq!(
        serde_json::to_value(&response.1 .0).unwrap(),
        json!({
            "message": "Invalid value for \"unknown attribute\": invalid value",
            "extensions": {"code": "UserProfileValidation", "user_profile_errors_total": 1,
                "user_profile_errors": [{"field": null, "error": null, "params": []}]}
        })
    );
}

#[test]
fn exactly_ten_user_profile_errors_are_reported_without_a_truncation_suffix() {
    let errors: Vec<_> = (0..10)
        .map(|index| UserProfileValidationError {
            field: Some(format!("field-{index}")),
            error_message: Some("required".into()),
            params: None,
        })
        .collect();
    let response = user_profile_error(&errors);
    assert_eq!(
        response
            .1
            .extensions
            .user_profile_errors
            .as_ref()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(response.1.extensions.user_profile_errors_total, Some(10));
    assert!(!response.1.message.contains("more"));
    assert!(response
        .1
        .message
        .ends_with("Invalid value for \"field-9\": required"));
}

#[test]
fn the_eleventh_user_profile_error_is_counted_but_not_disclosed() {
    let errors: Vec<_> = (0..11)
        .map(|index| UserProfileValidationError {
            field: Some(format!("field-{index}")),
            error_message: Some("required".into()),
            params: None,
        })
        .collect();
    let response = user_profile_error(&errors);
    assert_eq!(
        response
            .1
            .extensions
            .user_profile_errors
            .as_ref()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(response.1.extensions.user_profile_errors_total, Some(11));
    assert!(response.1.message.ends_with(" (and 1 more)"));
    assert!(!response.1.message.contains("field-10"));
}

#[test]
fn edit_user_errors_keep_the_route_status_and_select_its_existing_error_code() {
    for (status, code) in [
        (Status::Unauthorized, "Unauthorized"),
        (Status::Forbidden, "Unauthorized"),
        (Status::InternalServerError, "InternalServerError"),
        (Status::BadRequest, "UnknownError"),
        (Status::NotFound, "UnknownError"),
    ] {
        let response =
            EditUserError::from((status, "original denial".into())).0;
        assert_eq!(response.0, status);
        assert_eq!(response.1.message, "original denial");
        assert_eq!(response.1.extensions.code, code);
    }
}

#[test]
fn edit_user_debug_output_omits_the_response_message() {
    let error = EditUserError::from((
        Status::BadRequest,
        "private response details".into(),
    ));
    let debug = format!("{error:?}");
    assert!(debug.contains("UnknownError"));
    assert!(!debug.contains("private response details"));
}
