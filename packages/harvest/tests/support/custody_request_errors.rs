// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn wrapped_custody_request_unavailability_is_a_conflict_without_internal_context(
) {
    let error = anyhow::Error::new(CustodyRequestDownloadUnavailable)
        .context("private board data");
    let response =
        custody_request_download_error(error, "event-1", "ceremony-1");
    assert_eq!(response.0, Status::Conflict);
    assert_eq!(
        response.1.message,
        "Custody request download is no longer available"
    );
    assert_eq!(
        response.1.extensions.code,
        "CustodyRequestDownloadUnavailable"
    );
}

#[test]
fn other_custody_request_failures_have_a_generic_internal_response() {
    let response = custody_request_download_error(
        anyhow::anyhow!("Custody request download is no longer available"),
        "event-1",
        "ceremony-1",
    );
    assert_eq!(response.0, Status::InternalServerError);
    assert_eq!(response.1.message, "Failed to download custody request");
    assert_eq!(response.1.extensions.code, "InternalServerError");
}
#[test]
fn custody_confirmation_accepts_public_proof_json() {
    let input: super::ConfirmKeyCustodyInput =
        serde_json::from_value(serde_json::json!({
            "election_event_id": "event",
            "keys_ceremony_id": "ceremony",
            "custody_proof_json": "{\"trustee_id\":\"trustee\"}",
            "custody_proof_sha256": "abcd"
        }))
        .expect(
            "the custody API accepts a public proof without private-key fields",
        );
    let output = serde_json::to_value(input).unwrap();
    assert_eq!(output["custody_proof_json"], "{\"trustee_id\":\"trustee\"}");
    assert_eq!(output["custody_proof_sha256"], "abcd");
}
