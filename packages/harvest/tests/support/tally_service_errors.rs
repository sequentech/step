// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn wrapped_tally_validation_uses_the_domain_message_without_internal_context() {
    let error =
        anyhow::Error::new(TallyValidationError::new("Voting must be closed."))
            .context("private transaction context");
    let response = tally_response_error(tally_service_error(error));
    assert_eq!(response.0, Status::BadRequest);
    assert_eq!(response.1.message, "Voting must be closed.");
    assert_eq!(response.1.extensions.code, "TallyValidation");
}

#[test]
fn untyped_tally_failures_hide_internal_details() {
    let response = tally_response_error(tally_service_error(anyhow::anyhow!(
        "Voting must be closed."
    )));
    assert_eq!(response.0, Status::InternalServerError);
    assert_eq!(
        response.1.message,
        "Could not complete the tally operation."
    );
    assert_eq!(response.1.extensions.code, "InternalServerError");
}

#[test]
fn tally_authorization_failures_preserve_the_original_status_and_denial() {
    for status in [Status::Unauthorized, Status::Forbidden] {
        let response =
            tally_response_error((status, "Missing permission".into()));
        assert_eq!(response.0, status);
        assert_eq!(response.1.message, "Missing permission");
        assert_eq!(response.1.extensions.code, "Unauthorized");
    }
}

#[test]
fn other_tally_statuses_keep_their_status_while_hiding_the_message() {
    for status in [
        Status::NotFound,
        Status::Conflict,
        Status::ServiceUnavailable,
    ] {
        let response = tally_response_error((
            status,
            "private infrastructure details".into(),
        ));
        assert_eq!(response.0, status);
        assert_eq!(
            response.1.message,
            "Could not complete the tally operation."
        );
        assert_eq!(response.1.extensions.code, "InternalServerError");
    }
}
