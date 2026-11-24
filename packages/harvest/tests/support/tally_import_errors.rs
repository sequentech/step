// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn typed_import_errors_keep_their_distinct_http_statuses() {
    for (error, status, message) in [
        (TallySheetImportError::NotFound("import-1".into()), Status::NotFound, "Tally sheet import import-1 not found"),
        (TallySheetImportError::DocumentNotFound("document-1".into()), Status::NotFound, "Document document-1 not found"),
        (TallySheetImportError::DocumentTooLarge { document_id: "document-1".into(), size: 101, max: 100 }, Status::PayloadTooLarge, "Document document-1 is too large (101 bytes, max 100 bytes)"),
        (TallySheetImportError::InvalidReviewState { import_id: "import-1".into(), status: "Completed".into() }, Status::Conflict, "Tally sheet import import-1 cannot be reviewed from status Completed"),
    ] {
        let mapped = map_tally_sheet_import_error(error.into());
        assert_eq!(mapped.0, status);
        assert!(mapped.1.starts_with(message), "{}", mapped.1);
    }
}

#[test]
fn context_wrapped_import_errors_still_map_by_type_and_retain_the_debug_chain()
{
    let error = anyhow::Error::new(TallySheetImportError::DocumentNotFound(
        "document-1".into(),
    ))
    .context("review import failed");
    let mapped = map_tally_sheet_import_error(error);
    assert_eq!(mapped.0, Status::NotFound);
    assert!(mapped.1.starts_with("review import failed\n\nCaused by:\n    Document document-1 not found"), "{}", mapped.1);
}

#[test]
fn an_untyped_import_error_is_internal_even_when_its_message_looks_like_a_domain_error(
) {
    let mapped = map_tally_sheet_import_error(anyhow::anyhow!(
        "Document document-1 not found"
    ));
    assert_eq!(mapped.0, Status::InternalServerError);
    assert!(mapped.1.starts_with("Document document-1 not found"));
}

#[tokio::test]
async fn an_unreadable_ess_file_fails_in_the_conversion_not_the_pool() {
    let conversion = convert_ess_xml_in_blocking_pool(
        b"not xml".to_vec(),
        VotingChannel::PAPER,
        HashMap::new(),
        HashSet::new(),
    )
    .await
    .expect("the blocking pool runs the conversion");
    assert!(conversion.is_err());
}

#[tokio::test]
async fn a_failed_blocking_task_is_an_internal_error() {
    let error = tokio::task::spawn_blocking(|| {
        panic!("conversion panicked");
    })
    .await
    .expect_err("the task panicked");
    assert_eq!(blocking_task_error(error).0, Status::InternalServerError);
}
