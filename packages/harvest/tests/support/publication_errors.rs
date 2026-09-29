// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Portal clients distinguish authorization, stale state and unavailable
//! artifacts by status. Preserve those distinctions at the HTTP adapter.

use super::*;

#[test]
fn publication_errors_keep_their_http_status_and_service_message() {
    const MESSAGE: &str = "fixture explanation";
    for (error, expected) in [
        (
            ResultsPublicationServiceError::BadRequest(MESSAGE.into()),
            Status::BadRequest,
        ),
        (
            ResultsPublicationServiceError::Unauthorized(MESSAGE.into()),
            Status::Unauthorized,
        ),
        (
            ResultsPublicationServiceError::Forbidden(MESSAGE.into()),
            Status::Forbidden,
        ),
        (
            ResultsPublicationServiceError::NotFound(MESSAGE.into()),
            Status::NotFound,
        ),
        (
            ResultsPublicationServiceError::Conflict(MESSAGE.into()),
            Status::Conflict,
        ),
    ] {
        let (status, message) = map_service_error(error);
        assert_eq!(status, expected);
        assert_eq!(message, MESSAGE);
    }
}

#[test]
fn internal_publication_errors_hide_service_details_from_the_response() {
    let internal = anyhow::anyhow!("synthetic-token=do-not-expose")
        .context("storage request for private/internal/path failed");
    let (status, message) =
        map_service_error(ResultsPublicationServiceError::Internal(internal));
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(message, "Internal server error");
}
