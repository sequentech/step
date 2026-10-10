// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn wrapped_private_key_unavailability_is_a_conflict_without_internal_context() {
    let error = anyhow::Error::new(PrivateKeyDownloadUnavailable)
        .context("private board data");
    let response = private_key_download_error(error, "event-1", "ceremony-1");
    assert_eq!(response.0, Status::Conflict);
    assert_eq!(
        response.1.message,
        "Private key download is no longer available"
    );
    assert_eq!(response.1.extensions.code, "PrivateKeyDownloadUnavailable");
}

#[test]
fn other_private_key_failures_have_a_generic_internal_response() {
    let response = private_key_download_error(
        anyhow::anyhow!("Private key download is no longer available"),
        "event-1",
        "ceremony-1",
    );
    assert_eq!(response.0, Status::InternalServerError);
    assert_eq!(response.1.message, "Failed to download private key");
    assert_eq!(response.1.extensions.code, "InternalServerError");
}
