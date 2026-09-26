// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn ballot_validation_responses_expose_the_validation_message_without_the_task_prefix(
) {
    let response = ballot_publication_error(
        true,
        "Acclamation changed after voting started",
        "Publish task task-1 failed: Acclamation changed after voting started",
    );
    assert_eq!(response.0, Status::BadRequest);
    assert_eq!(
        response.1.message,
        "Acclamation changed after voting started"
    );
    assert_eq!(response.1.extensions.code, "BallotPublicationValidation");
}

#[test]
fn internal_ballot_publication_responses_keep_the_existing_task_failure_details(
) {
    let response = ballot_publication_error(
        false,
        "storage unavailable",
        "Publish task task-1 failed: storage unavailable",
    );
    assert_eq!(response.0, Status::InternalServerError);
    assert_eq!(
        response.1.message,
        "Publish task task-1 failed: storage unavailable"
    );
    assert_eq!(response.1.extensions.code, "InternalServerError");
}
