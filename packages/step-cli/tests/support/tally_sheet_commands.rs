// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Each tally-sheet command runs against in-memory GraphQL and document
//! adapters, so the requests it sends and how it reads the answers are checked
//! without a server.

use super::*;
use crate::adapters::memory::documents::{Download, MemoryDocuments, Upload, UPLOADED_DOCUMENT_ID};
use crate::adapters::memory::graphql::QueuedGraphql;
use serde_json::json;

const EVENT: &str = "synthetic-event";
const EVENT_UUID: &str = "aaaaaaaa-0000-4000-8000-000000000001";
const IMPORT_UUID: &str = "bbbbbbbb-0000-4000-8000-000000000002";
const TALLY_UUID: &str = "cccccccc-0000-4000-8000-000000000003";
const SOURCE_DOCUMENT_UUID: &str = "dddddddd-0000-4000-8000-000000000004";
const ABC_DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const OUTAGE: &str = "HTTP Status: 503 Service Unavailable\nError Message: synthetic outage";

type Run = fn(&QueuedGraphql) -> Result<Value, Box<dyn Error>>;

fn answering(field: &str, payload: Value) -> QueuedGraphql {
    QueuedGraphql::default().respond(json!({"data": {field: payload}}))
}

/// The operation name and variables of the only request sent.
fn only_request(graphql: &QueuedGraphql) -> (String, Value) {
    let requests = graphql.requests();
    assert_eq!(requests.len(), 1, "{requests:?}");
    let operation = requests[0]["operationName"].as_str().unwrap().to_string();
    (operation, requests[0]["variables"].clone())
}

fn import_row(source_file_name: Value) -> Value {
    json!({
        "id": IMPORT_UUID,
        "source_document_id": SOURCE_DOCUMENT_UUID,
        "source_file_name": source_file_name,
        "source_sha256": ABC_DIGEST,
        "source_format": "ESS_ENHANCED_XML",
        "selected_channel": "PAPER",
        "status": "PENDING",
        "created_at": "2026-09-24T10:00:00+00:00",
        "created_by_user_id": "synthetic-user",
        "summary": {"sheets": 1},
        "validation_report": null,
        "canonical_csv_sha256": null
    })
}

fn import_with_items(source_file_name: Value) -> Value {
    let mut row = import_row(source_file_name);
    row["items"] = json!([{
        "id": "eeeeeeee-0000-4000-8000-000000000005",
        "election_id": "eeeeeeee-0000-4000-8000-000000000006",
        "area_id": "eeeeeeee-0000-4000-8000-000000000007",
        "contest_id": "eeeeeeee-0000-4000-8000-000000000008",
        "channel": "PAPER",
        "generated_tally_sheet_id": null,
        "baseline_approved_tally_sheet_id": null,
        "baseline_approved_version": null,
        "change_type": "NEW",
        "status": "PENDING",
        "source_refs": null,
        "validation_warnings": []
    }]);
    row
}

/// Every command that talks to GraphQL, with the response field it reads and
/// a valid payload for that field.
fn commands() -> [(&'static str, &'static str, Value, Run); 9] {
    [
        (
            "create",
            "create_new_tally_sheet",
            json!({"id": "sheet-1", "status": "PENDING", "version": 1}),
            |graphql| {
                create_tally_sheet(
                    graphql,
                    EVENT,
                    "area-1",
                    "contest-1",
                    VotingChannelArg::Paper,
                    json!({}),
                )
            },
        ),
        (
            "review",
            "review_tally_sheet",
            json!({"id": "sheet-1", "status": "APPROVED", "version": 2}),
            |graphql| review_tally_sheet(graphql, EVENT, "sheet-1", TallySheetStatusArg::Approved),
        ),
        (
            "import-preview",
            "preview_tally_sheet_import",
            json!({"preview": {"sheets": 1}}),
            |graphql| {
                preview_tally_sheet_import(
                    graphql,
                    EVENT,
                    "document-1",
                    None,
                    TallySheetImportSourceFormatArg::EssEnhancedXml,
                    VotingChannelArg::Paper,
                )
            },
        ),
        (
            "import-create",
            "create_tally_sheet_import",
            json!({"tally_sheet_import": {"id": "import-1"}}),
            |graphql| {
                create_tally_sheet_import(
                    graphql,
                    EVENT,
                    "document-1",
                    None,
                    TallySheetImportSourceFormatArg::EssEnhancedXml,
                    VotingChannelArg::Paper,
                )
            },
        ),
        (
            "import-review",
            "review_tally_sheet_import",
            json!({"tally_sheet_import": {"id": "import-1"}}),
            |graphql| {
                review_tally_sheet_import(
                    graphql,
                    EVENT,
                    "import-1",
                    TallySheetImportDecisionArg::Approve,
                )
            },
        ),
        (
            "import-list",
            "sequent_backend_tally_sheet_import",
            json!([import_row(Value::Null)]),
            |graphql| list_tally_sheet_imports(graphql, EVENT_UUID, 50),
        ),
        (
            "import-show",
            "sequent_backend_tally_sheet_import",
            json!([import_with_items(Value::Null)]),
            |graphql| get_tally_sheet_import(graphql, EVENT_UUID, IMPORT_UUID),
        ),
        (
            "import-download-source",
            "sequent_backend_tally_sheet_import",
            json!([import_with_items(Value::Null)]),
            |graphql| {
                download_tally_sheet_import_source(
                    graphql,
                    &MemoryDocuments::default(),
                    EVENT_UUID,
                    IMPORT_UUID,
                    Path::new("downloads"),
                )
                .map(|path| json!(path))
            },
        ),
        (
            "recount",
            "recount_tally_session",
            json!({"tally_session_id": TALLY_UUID}),
            |graphql| recount_tally_session(graphql, EVENT_UUID, TALLY_UUID).map(Value::from),
        ),
    ]
}

#[test]
fn creating_a_tally_sheet_posts_its_content_for_the_area_and_contest() {
    let sheet = json!({"id": "sheet-1", "status": "PENDING", "version": 1});
    let graphql = answering("create_new_tally_sheet", sheet.clone());
    let created = create_tally_sheet(
        &graphql,
        EVENT,
        "area-1",
        "contest-1",
        VotingChannelArg::Postal,
        json!({"total_votes": 7}),
    )
    .unwrap();
    assert_eq!(created, sheet);
    assert_eq!(
        only_request(&graphql),
        (
            "CreateNewTallySheet".into(),
            json!({
                "election_event_id": EVENT,
                "channel": "POSTAL",
                "content": {"total_votes": 7},
                "contest_id": "contest-1",
                "area_id": "area-1"
            })
        )
    );
}

#[test]
fn reviewing_a_tally_sheet_posts_the_new_status() {
    for (status, sent) in [
        (TallySheetStatusArg::Approved, "APPROVED"),
        (TallySheetStatusArg::Disapproved, "DISAPPROVED"),
    ] {
        let sheet = json!({"id": "sheet-1", "status": sent, "version": 2});
        let graphql = answering("review_tally_sheet", sheet.clone());
        assert_eq!(
            review_tally_sheet(&graphql, EVENT, "sheet-1", status).unwrap(),
            sheet
        );
        assert_eq!(
            only_request(&graphql),
            (
                "ReviewTallySheet".into(),
                json!({"election_event_id": EVENT, "tally_sheet_id": "sheet-1", "new_status": sent})
            )
        );
    }
}

#[test]
fn previewing_an_import_posts_the_document_digest_format_and_channel() {
    let graphql = answering(
        "preview_tally_sheet_import",
        json!({"preview": {"sheets": 3}}),
    );
    let preview = preview_tally_sheet_import(
        &graphql,
        EVENT,
        "document-1",
        Some(ABC_DIGEST),
        TallySheetImportSourceFormatArg::CanonicalCsv,
        VotingChannelArg::InPerson,
    )
    .unwrap();
    assert_eq!(preview, json!({"sheets": 3}));
    assert_eq!(
        only_request(&graphql),
        (
            "PreviewTallySheetImport".into(),
            json!({
                "election_event_id": EVENT,
                "document_id": "document-1",
                "sha256": ABC_DIGEST,
                "source_format": "CANONICAL_CSV",
                "selected_channel": "IN_PERSON"
            })
        )
    );
}

#[test]
fn creating_an_import_without_a_digest_sends_a_null_sha256() {
    let graphql = answering(
        "create_tally_sheet_import",
        json!({"tally_sheet_import": {"id": "import-1"}}),
    );
    let import = create_tally_sheet_import(
        &graphql,
        EVENT,
        "document-1",
        None,
        TallySheetImportSourceFormatArg::EssEnhancedXml,
        VotingChannelArg::Paper,
    )
    .unwrap();
    assert_eq!(import, json!({"id": "import-1"}));
    assert_eq!(
        only_request(&graphql),
        (
            "CreateTallySheetImport".into(),
            json!({
                "election_event_id": EVENT,
                "document_id": "document-1",
                "sha256": null,
                "source_format": "ESS_ENHANCED_XML",
                "selected_channel": "PAPER"
            })
        )
    );
}

#[test]
fn reviewing_an_import_posts_the_decision() {
    for (decision, sent) in [
        (TallySheetImportDecisionArg::Approve, "APPROVE"),
        (TallySheetImportDecisionArg::Disapprove, "DISAPPROVE"),
    ] {
        let graphql = answering(
            "review_tally_sheet_import",
            json!({"tally_sheet_import": {"id": "import-1", "status": sent}}),
        );
        assert_eq!(
            review_tally_sheet_import(&graphql, EVENT, "import-1", decision).unwrap(),
            json!({"id": "import-1", "status": sent})
        );
        assert_eq!(
            only_request(&graphql),
            (
                "ReviewTallySheetImport".into(),
                json!({"election_event_id": EVENT, "import_id": "import-1", "decision": sent})
            )
        );
    }
}

#[test]
fn listing_imports_sends_the_normalized_event_uuid_and_the_limit() {
    let rows = json!([import_row(json!("results.xml")), import_row(Value::Null)]);
    let graphql = answering("sequent_backend_tally_sheet_import", rows.clone());
    assert_eq!(
        list_tally_sheet_imports(&graphql, &EVENT_UUID.to_uppercase(), 5).unwrap(),
        rows
    );
    assert_eq!(
        only_request(&graphql),
        (
            "ListTallySheetImports".into(),
            json!({"election_event_id": EVENT_UUID, "limit": 5})
        )
    );
}

#[test]
fn an_empty_import_list_is_an_empty_result() {
    let graphql = answering("sequent_backend_tally_sheet_import", json!([]));
    assert_eq!(
        list_tally_sheet_imports(&graphql, EVENT_UUID, 50).unwrap(),
        json!([])
    );
}

#[test]
fn showing_an_import_returns_the_first_row_with_its_items() {
    let row = import_with_items(Value::Null);
    let graphql = answering(
        "sequent_backend_tally_sheet_import",
        json!([row.clone(), import_with_items(json!("other.xml"))]),
    );
    assert_eq!(
        get_tally_sheet_import(
            &graphql,
            &EVENT_UUID.to_uppercase(),
            &IMPORT_UUID.to_uppercase()
        )
        .unwrap(),
        row
    );
    assert_eq!(
        only_request(&graphql),
        (
            "GetTallySheetImport".into(),
            json!({"election_event_id": EVENT_UUID, "import_id": IMPORT_UUID})
        )
    );
}

#[test]
fn recounting_sends_normalized_uuids_and_returns_the_new_session() {
    let graphql = answering(
        "recount_tally_session",
        json!({"tally_session_id": "ffffffff-0000-4000-8000-000000000009"}),
    );
    assert_eq!(
        recount_tally_session(
            &graphql,
            &EVENT_UUID.to_uppercase(),
            &TALLY_UUID.to_uppercase()
        )
        .unwrap(),
        "ffffffff-0000-4000-8000-000000000009"
    );
    assert_eq!(
        only_request(&graphql),
        (
            "RecountTallySession".into(),
            json!({"election_event_id": EVENT_UUID, "tally_session_id": TALLY_UUID})
        )
    );
}

#[test]
fn list_show_download_and_recount_reject_invalid_uuids_without_a_request() {
    // The other commands send their ids as given; see the tests above.
    let invalid: [(&str, Run); 6] = [
        ("import-list", |graphql| {
            list_tally_sheet_imports(graphql, EVENT, 50)
        }),
        ("import-show event", |graphql| {
            get_tally_sheet_import(graphql, EVENT, IMPORT_UUID)
        }),
        ("import-show import", |graphql| {
            get_tally_sheet_import(graphql, EVENT_UUID, "import-1")
        }),
        ("import-download-source", |graphql| {
            download_tally_sheet_import_source(
                graphql,
                &MemoryDocuments::default(),
                EVENT_UUID,
                "import-1",
                Path::new("downloads"),
            )
            .map(|path| json!(path))
        }),
        ("recount event", |graphql| {
            recount_tally_session(graphql, EVENT, TALLY_UUID).map(Value::from)
        }),
        ("recount tally", |graphql| {
            recount_tally_session(graphql, EVENT_UUID, "tally-1").map(Value::from)
        }),
    ];
    for (name, run) in invalid {
        let graphql = QueuedGraphql::default();
        let error = run(&graphql).unwrap_err();
        assert!(
            error.downcast_ref::<::uuid::Error>().is_some(),
            "{name}: {error}"
        );
        assert!(graphql.requests().is_empty(), "{name}");
    }
}

#[test]
fn every_command_succeeds_with_complete_data() {
    for (name, field, payload, run) in commands() {
        assert!(run(&answering(field, payload)).is_ok(), "{name}");
    }
}

#[test]
fn graphql_errors_win_over_returned_data_in_every_command() {
    for (name, field, payload, run) in commands() {
        let graphql = QueuedGraphql::default().respond(json!({
            "data": {field: payload},
            "errors": [{"message": "first denial"}, {"message": "second denial"}]
        }));
        assert_eq!(
            run(&graphql).unwrap_err().to_string(),
            "first denial, second denial",
            "{name}"
        );
    }
}

#[test]
fn transport_failures_are_returned_unchanged_by_every_command() {
    for (name, _, _, run) in commands() {
        let graphql = QueuedGraphql::default().fail(OUTAGE);
        assert_eq!(run(&graphql).unwrap_err().to_string(), OUTAGE, "{name}");
    }
}

#[test]
fn a_missing_result_fails_with_the_command_message() {
    for (name, message) in [
        ("create", "failed creating tally sheet"),
        ("review", "failed reviewing tally sheet"),
        ("import-preview", "failed previewing tally sheet import"),
        ("import-create", "failed creating tally sheet import"),
        ("import-review", "failed reviewing tally sheet import"),
        ("import-show", "tally sheet import not found"),
        ("import-download-source", "tally sheet import not found"),
        ("recount", "failed recounting tally session"),
    ] {
        let (_, field, payload, run) = commands()
            .into_iter()
            .find(|command| command.0 == name)
            .unwrap();
        let missing = if payload.is_array() {
            json!([])
        } else {
            Value::Null
        };
        assert_eq!(
            run(&answering(field, missing)).unwrap_err().to_string(),
            message,
            "{name}"
        );
    }
}

fn download(
    source_file_name: Value,
    import_id: &str,
    documents: &MemoryDocuments,
) -> Result<PathBuf, Box<dyn Error>> {
    let graphql = answering(
        "sequent_backend_tally_sheet_import",
        json!([import_with_items(source_file_name)]),
    );
    download_tally_sheet_import_source(
        &graphql,
        documents,
        &EVENT_UUID.to_uppercase(),
        import_id,
        Path::new("downloads"),
    )
}

#[test]
fn a_source_is_downloaded_under_the_base_name_of_its_file() {
    for (source_file_name, file_name) in [
        ("results.xml", "results.xml"),
        ("uploads/2026/results.xml", "results.xml"),
        ("../../etc/passwd", "passwd"),
        ("reports/", "reports"),
    ] {
        let documents = MemoryDocuments::default();
        let path = download(json!(source_file_name), IMPORT_UUID, &documents).unwrap();
        let expected = Path::new("downloads").join(file_name);
        assert_eq!(path, expected, "{source_file_name}");
        // The event id reaches the document lookup as given, unlike the
        // normalized uuid variables of the import query.
        assert_eq!(
            documents.downloads(),
            [Download {
                election_event_id: EVENT_UUID.to_uppercase(),
                document_id: SOURCE_DOCUMENT_UUID.into(),
                output_path: expected,
            }]
        );
    }
}

#[test]
fn a_source_without_a_file_name_is_downloaded_under_the_import_id() {
    for source_file_name in [Value::Null, json!(""), json!(".."), json!("/")] {
        let documents = MemoryDocuments::default();
        let path = download(
            source_file_name.clone(),
            &IMPORT_UUID.to_uppercase(),
            &documents,
        )
        .unwrap();
        assert_eq!(
            path,
            Path::new("downloads/tally-sheet-import-BBBBBBBB-0000-4000-8000-000000000002"),
            "{source_file_name}"
        );
        assert_eq!(documents.downloads().len(), 1);
    }
}

#[test]
fn an_import_that_is_not_found_downloads_nothing() {
    let documents = MemoryDocuments::default();
    let graphql = answering("sequent_backend_tally_sheet_import", json!([]));
    download_tally_sheet_import_source(
        &graphql,
        &documents,
        EVENT_UUID,
        IMPORT_UUID,
        Path::new("downloads"),
    )
    .unwrap_err();
    assert!(documents.downloads().is_empty());
}

#[test]
fn download_failures_are_returned_unchanged() {
    let documents = MemoryDocuments::failing("synthetic storage failure");
    let error = download(json!("results.xml"), IMPORT_UUID, &documents).unwrap_err();
    assert_eq!(error.to_string(), "synthetic storage failure");
    assert_eq!(documents.downloads().len(), 1);
}

fn source_file(bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("results.csv");
    fs::write(&file, bytes).unwrap();
    (directory, file)
}

#[test]
fn a_local_source_is_uploaded_for_the_event_once_its_digest_matches() {
    let (_directory, file) = source_file(b"abc");
    let documents = MemoryDocuments::default();
    let expected = format!(" {}\n", ABC_DIGEST.to_uppercase());
    let document =
        resolve_import_document(&documents, EVENT, Some(&file), None, Some(&expected), true)
            .unwrap();
    assert_eq!(document.document_id, UPLOADED_DOCUMENT_ID);
    assert_eq!(document.sha256.as_deref(), Some(ABC_DIGEST));
    assert_eq!(
        documents.uploads(),
        [Upload {
            file_path: file,
            is_local: true,
            election_event_id: EVENT.into(),
        }]
    );
}

#[test]
fn a_local_source_without_a_digest_is_sent_with_the_computed_one() {
    let (_directory, file) = source_file(b"abc");
    let documents = MemoryDocuments::default();
    let document =
        resolve_import_document(&documents, EVENT, Some(&file), None, None, false).unwrap();
    assert_eq!(document.sha256.as_deref(), Some(ABC_DIGEST));
    assert!(!documents.uploads()[0].is_local);
}

#[test]
fn a_digest_mismatch_names_both_digests() {
    let (_directory, file) = source_file(b"different bytes");
    let documents = MemoryDocuments::default();
    let error = resolve_import_document(
        &documents,
        EVENT,
        Some(&file),
        None,
        Some(ABC_DIGEST),
        false,
    )
    .err()
    .unwrap();
    assert_eq!(
        error.to_string(),
        format!(
            "sha256 mismatch: expected {ABC_DIGEST}, got \
             538a741c622733afe147819e939d82cc071edfbeb3eb49ed701a55975ac288b3"
        )
    );
    assert!(documents.uploads().is_empty());
}

#[test]
fn an_invalid_digest_for_a_local_source_is_rejected_before_uploading() {
    let (_directory, file) = source_file(b"abc");
    let documents = MemoryDocuments::default();
    let error = resolve_import_document(&documents, EVENT, Some(&file), None, Some("abc"), false)
        .err()
        .unwrap();
    assert_eq!(
        error.to_string(),
        "sha256 must be a 64-character hex string"
    );
    assert!(documents.uploads().is_empty());
}

#[test]
fn upload_failures_are_returned_unchanged() {
    let (_directory, file) = source_file(b"abc");
    let documents = MemoryDocuments::failing("synthetic upload failure");
    let error = resolve_import_document(&documents, EVENT, Some(&file), None, None, false)
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "synthetic upload failure");
    assert_eq!(documents.uploads().len(), 1);
}
