// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn results_service_denials_keep_their_status_and_message() {
    for (error, status, message) in [
        (
            ResultsPublicationServiceError::BadRequest("invalid route".into()),
            Status::BadRequest,
            "invalid route",
        ),
        (
            ResultsPublicationServiceError::Unauthorized(
                "missing reader identity".into(),
            ),
            Status::Unauthorized,
            "missing reader identity",
        ),
        (
            ResultsPublicationServiceError::Forbidden(
                "area unavailable".into(),
            ),
            Status::Forbidden,
            "area unavailable",
        ),
        (
            ResultsPublicationServiceError::NotFound(
                "publication missing".into(),
            ),
            Status::NotFound,
            "publication missing",
        ),
        (
            ResultsPublicationServiceError::Conflict(
                "publication not active".into(),
            ),
            Status::Conflict,
            "publication not active",
        ),
    ] {
        assert_eq!(map_service_error(error), (status, message.into()));
    }
}

#[test]
fn internal_results_failures_hide_the_complete_error_chain() {
    let error = anyhow::anyhow!("private database details")
        .context("publication write failed");
    assert_eq!(
        map_service_error(ResultsPublicationServiceError::Internal(error)),
        (Status::InternalServerError, "Internal server error".into())
    );
}
