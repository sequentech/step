// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A reviewed import must still refer to the exact uploaded bytes, and domain
//! failures must retain their HTTP meaning through anyhow context layers.

use super::*;

#[test]
fn source_digest_accepts_the_known_bytes_and_rejects_a_changed_upload() {
    // Published SHA-256 test vector for ASCII "abc", independent of hash_bytes.
    const ABC_DIGEST: &str =
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    for expected in [
        ABC_DIGEST.to_string(),
        format!("  {}\n", ABC_DIGEST.to_uppercase()),
    ] {
        assert!(verify_source_sha256(Some(&expected), b"abc").is_ok());
        assert!(verify_source_sha256(Some(&expected), b"abd").is_err());
        assert!(verify_source_sha256(Some(&expected), b"abc\n").is_err());
    }
    for malformed in ["abc", "not a digest", &"0".repeat(64)] {
        assert!(verify_source_sha256(Some(malformed), b"abc").is_err());
    }
}

#[test]
fn absent_or_blank_optional_digest_keeps_unreviewed_uploads_supported() {
    for expected in [None, Some(""), Some(" \t\n")] {
        assert!(verify_source_sha256(expected, b"unreviewed upload").is_ok());
    }
}

#[test]
fn import_domain_errors_keep_their_status_even_with_added_context() {
    for (error, status) in [
        (
            TallySheetImportError::NotFound("missing-import".into()),
            Status::NotFound,
        ),
        (
            TallySheetImportError::DocumentNotFound("missing-document".into()),
            Status::NotFound,
        ),
        (
            TallySheetImportError::DocumentTooLarge {
                document_id: "large-document".into(),
                size: 101,
                max: 100,
            },
            Status::PayloadTooLarge,
        ),
        (
            TallySheetImportError::InvalidReviewState {
                import_id: "reviewed-import".into(),
                status: "APPROVED".into(),
            },
            Status::Conflict,
        ),
    ] {
        let error = anyhow::Error::new(error).context("import request failed");
        let (actual, message) = map_tally_sheet_import_error(error);
        assert_eq!(actual, status);
        assert!(message.contains("import request failed"));
    }
    let (status, _) = map_tally_sheet_import_error(anyhow::anyhow!(
        "unavailable fixture database"
    ));
    assert_eq!(status, Status::InternalServerError);
}
