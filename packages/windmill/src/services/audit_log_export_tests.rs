// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use electoral_log::messages::message::SigningData;
use electoral_log::messages::newtypes::{AuditLogEntry, EventIdString};
use voting_crypto::signing::SigningKey;

const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
const EVENT: &str = "0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9";

fn request(scope: AuditExportScope, event: Option<&str>) -> AuditExportRequest {
    AuditExportRequest {
        tenant_id: TENANT.into(),
        election_event_id: event.map(str::to_string),
        format: AuditExportFormat::Sql,
        scope,
        username: None,
        election_id: None,
        area_id: None,
        created_from: None,
        created_to: None,
        time_zone: None,
        requested_by: AuditActor {
            user_id: Some("auditor-id".into()),
            username: Some("auditor".into()),
            role: None,
        },
    }
}

fn stored(description: &str) -> ElectoralLogMessage {
    let data = SigningData::new(
        SigningKey::generate().unwrap(),
        "ada",
        SigningKey::generate().unwrap(),
    );
    let message = Message::audit_message(
        EventIdString(TENANT.into()),
        AuditLogEntry {
            kind: AuditStatementKind::AdminChange,
            event_type: StatementEventType::USER,
            log_type: StatementLogType::INFO,
            description: description.into(),
            details_json: "{}".into(),
            entry_id: "entry".into(),
        },
        1_833_000_000,
        &data,
        Some("ada-id".into()),
        Some("ada".into()),
        None,
        None,
    )
    .unwrap();
    let mut stored = ElectoralLogMessage::try_from(&message).unwrap();
    stored.id = 7;
    // 3 February 2028, 16:30:05 UTC.
    stored.created = 1_833_208_205;
    stored
}

#[test]
fn a_scope_must_fit_its_log_and_name_what_it_needs() {
    for (scope, event) in [
        (AuditExportScope::TenantLog, None),
        (AuditExportScope::All, Some(EVENT)),
    ] {
        request(scope, event).validate().unwrap();
    }
    for (scope, event) in [
        (AuditExportScope::TenantLog, Some(EVENT)),
        (AuditExportScope::AccountWithEvents, Some(EVENT)),
        (AuditExportScope::All, None),
        (AuditExportScope::Election, None),
        (AuditExportScope::Area, Some("")),
        // Each of these lacks what it names.
        (AuditExportScope::Account, None),
        (AuditExportScope::AccountWithEvents, None),
        (AuditExportScope::Election, Some(EVENT)),
        (AuditExportScope::Area, Some(EVENT)),
    ] {
        assert!(
            request(scope, event).validate().is_err(),
            "{scope} {event:?}"
        );
    }
    let mut account = request(AuditExportScope::Account, Some(EVENT));
    account.username = Some(" ada ".into());
    account.validate().unwrap();
    account.election_event_id = None;
    account.validate().unwrap();
    let mut election = request(AuditExportScope::Election, Some(EVENT));
    election.election_id = Some("election".into());
    election.validate().unwrap();
    let mut area = request(AuditExportScope::Area, Some(EVENT));
    area.area_id = Some("area".into());
    area.validate().unwrap();

    let mut bad = request(AuditExportScope::TenantLog, None);
    bad.time_zone = Some("Mars/Olympus".into());
    assert!(bad.validate().is_err());
    bad.time_zone = Some("Asia/Manila".into());
    bad.created_from = Some("yesterday".into());
    assert!(bad.validate().is_err());
    bad.created_from = None;
    bad.tenant_id = "tenant".into();
    assert!(bad.validate().is_err());
    bad.tenant_id = TENANT.into();
    bad.scope = AuditExportScope::All;
    bad.election_event_id = Some("event".into());
    assert!(bad.validate().is_err());
}

#[test]
fn a_scope_and_a_range_become_the_filters_of_the_list() {
    let texts = |request: &AuditExportRequest| -> Vec<String> {
        request
            .filters()
            .unwrap()
            .iter()
            .map(|filter| match filter {
                Filter::Text(column, operator, value) => format!("{column} {operator} {value}"),
                Filter::Number(column, operator, value) => format!("{column} {operator} {value}"),
                Filter::ElectionScope(_) => "scope".to_string(),
            })
            .collect()
    };
    assert!(texts(&request(AuditExportScope::TenantLog, None)).is_empty());
    let mut account = request(AuditExportScope::AccountWithEvents, None);
    account.username = Some(" ada ".into());
    account.created_from = Some("2028-02-03T00:00:00+08:00".into());
    account.created_to = Some("2028-02-03T23:59:00+08:00".into());
    assert_eq!(
        texts(&account),
        [
            // Both ends are included, to the minute.
            "created >= 1833120000",
            "created < 1833206400",
            "username = ada",
        ]
    );
    let mut election = request(AuditExportScope::Election, Some(EVENT));
    election.election_id = Some("election".into());
    assert_eq!(texts(&election), ["election_id = election"]);
    let mut area = request(AuditExportScope::Area, Some(EVENT));
    area.area_id = Some("area".into());
    assert_eq!(texts(&area), ["area_id = area"]);
    assert_eq!(
        account.target().unwrap(),
        AuditLogTarget {
            tenant_id: Uuid::parse_str(TENANT).unwrap(),
            election_event_id: None,
        }
    );
    assert_eq!(
        election.target().unwrap().election_event_id,
        Some(Uuid::parse_str(EVENT).unwrap())
    );
}

#[test]
fn a_row_shows_its_time_in_utc_and_in_the_exports_zone() {
    let entry = stored("Updated tenant \"acme\"");
    let row = AuditExportRow::new("tenant", &entry, "Asia/Manila".parse().unwrap()).unwrap();
    assert_eq!(row.log, "tenant");
    assert_eq!(row.id, 7);
    assert_eq!(row.created_utc, "2028-02-03T16:30:05Z");
    assert_eq!(row.created_local, "2028-02-04T00:30:05+08:00");
    assert_eq!(row.time_zone, "Asia/Manila");
    assert_eq!(
        (row.event_type.as_str(), row.log_type.as_str()),
        ("USER", "INFO")
    );
    assert_eq!(row.username.as_deref(), Some("ada"));
    assert_eq!(row.sender_name, "ada");
    assert_eq!(row.description, "Updated tenant \"acme\"");
    assert!(!row.message.contains('\n'));

    // What the row carries verifies the entry.
    let message = Message::record_deserialize(&entry.message).unwrap();
    assert_eq!(
        row.sender_public_key,
        message.sender.pk.to_b64_string().unwrap()
    );
    assert_eq!(
        row.statement,
        base64_text(&message.statement.record_serialize().unwrap())
    );
    let mut unreadable = entry;
    unreadable.message = vec![1, 2, 3];
    assert!(AuditExportRow::new("tenant", &unreadable, Tz::UTC).is_err());
    unreadable = stored("x");
    unreadable.created = i64::MAX;
    assert!(AuditExportRow::new("tenant", &unreadable, Tz::UTC).is_err());
}

#[test]
fn the_tenant_log_is_shown_in_the_tenants_zone() {
    assert_eq!(tenant_log_zone(None), Tz::UTC);
    assert_eq!(tenant_log_zone(Some(&json!({}))), Tz::UTC);
    assert_eq!(
        tenant_log_zone(Some(&json!({"logs_timezone": "Nowhere/None"}))),
        Tz::UTC
    );
    assert_eq!(
        tenant_log_zone(Some(&json!({"logs_timezone": "Europe/Madrid"}))).name(),
        "Europe/Madrid"
    );
}

fn manifest(request: &AuditExportRequest, entries: u64) -> ExportManifest {
    ExportManifest::new(
        request,
        "Asia/Manila".parse().unwrap(),
        vec![ExportedLog {
            board: "board".into(),
            retired: None,
            log: "tenant".into(),
            system_public_key: Some("key".into()),
            entries,
        }],
    )
}

#[test]
fn the_sql_export_is_one_statement_per_row_that_cannot_end_early() {
    let mut entry = stored("It's \"quoted\";\nDROP TABLE audit_log; --");
    entry.user_id = None;
    let row = AuditExportRow::new("tenant", &entry, Tz::UTC).unwrap();
    let insert = sql_insert(&row);
    assert_eq!(insert.matches('\n').count(), 1);
    assert!(insert.starts_with("INSERT INTO audit_log VALUES ('tenant', 7, 1833208205, "));
    assert!(insert.contains("'It''s \"quoted\"; DROP TABLE audit_log; --'"));
    assert!(insert.contains(", NULL, 'ada', "));
    assert!(insert.contains("decode('"));
    assert!(insert.ends_with(");\n"));
    let header = sql_header();
    assert!(header.starts_with("BEGIN;\nCREATE TABLE audit_log ("));
    assert!(header.contains("statement BYTEA NOT NULL"));
    assert!(header.contains("PRIMARY KEY (log, id)"));

    let mut request = request(AuditExportScope::Account, None);
    request.username = Some("o'brien".into());
    let footer = sql_footer(&manifest(&request, 1)).unwrap();
    assert!(footer.starts_with("CREATE TABLE audit_log_export (manifest JSONB NOT NULL);\n"));
    assert!(footer.contains("\"username\":\"o''brien\""));
    assert!(footer.contains("\"system_public_key\":\"key\""));
    assert!(footer.contains("\"entries\":1"));
    assert!(!footer.contains("\"board\""));
    assert!(footer.ends_with("COMMIT;\n"));
}

#[test]
fn the_pdf_page_escapes_what_it_shows() {
    let entry = stored("<script>alert(1)</script> & more");
    let row = AuditExportRow::new("tenant", &entry, Tz::UTC).unwrap();
    let mut request = request(AuditExportScope::Account, None);
    request.username = Some("ada".into());
    request.created_from = Some("2028-02-01T00:00:00Z".into());
    let page = pdf_html(&manifest(&request, 1), &[row]);
    assert!(page.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; more"));
    assert!(!page.contains("<script>"));
    assert!(page.contains("<th>Account</th><td>ada</td>"));
    assert!(page.contains("<th>Scope</th><td>ACCOUNT</td>"));
    assert!(page.contains("<th>Created to</th><td>-</td>"));
    assert!(page.contains("2028-02-03T16:30:05Z"));
    let whole = pdf_html(
        &manifest(&self::request(AuditExportScope::TenantLog, None), 0),
        &[],
    );
    assert!(!whole.contains("<th>Account</th>"));
}

#[test]
fn an_export_is_recorded_with_its_scope_count_and_hash() {
    let mut account = request(AuditExportScope::AccountWithEvents, None);
    account.username = Some("ada".into());
    account.format = AuditExportFormat::Csv;
    let entry = logs_exported_entry(&account, &manifest(&account, 12), "ab12", "document").unwrap();
    assert_eq!(entry.statement_kind, AuditStatementKind::LogsExported);
    assert_eq!(entry.event_type, StatementEventType::USER);
    assert_eq!(entry.username.as_deref(), Some("auditor"));
    assert_eq!(entry.target.election_event_id, None);
    assert_eq!(
        entry.description,
        "Exported the entries of ada in the tenant log and every election event as CSV: \
         12 entries"
    );
    assert_eq!(entry.details["format"], "CSV");
    assert_eq!(entry.details["scope"], "ACCOUNT_WITH_EVENTS");
    assert_eq!(entry.details["entries"], 12);
    assert_eq!(entry.details["sha256"], "ab12");
    assert_eq!(entry.details["document_id"], "document");
    assert_eq!(entry.details["time_zone"], "Asia/Manila");

    let descriptions: Vec<String> = [
        (AuditExportScope::TenantLog, None),
        (AuditExportScope::All, Some(EVENT)),
        (AuditExportScope::Account, None),
        (AuditExportScope::Election, Some(EVENT)),
        (AuditExportScope::Area, Some(EVENT)),
    ]
    .into_iter()
    .map(|(scope, event)| {
        let mut request = request(scope, event);
        request.username = Some("ada".into());
        request.requested_by = AuditActor::default();
        let entry = logs_exported_entry(&request, &manifest(&request, 0), "", "").unwrap();
        assert_eq!(entry.event_type, StatementEventType::SYSTEM);
        entry.description
    })
    .collect();
    assert_eq!(
        descriptions,
        [
            "Exported the tenant log as SQL: 0 entries",
            "Exported the election event's log as SQL: 0 entries",
            "Exported the entries of ada as SQL: 0 entries",
            "Exported the entries of an election as SQL: 0 entries",
            "Exported the entries of an area as SQL: 0 entries",
        ]
    );
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        (
            AuditExportFormat::Csv.extension(),
            AuditExportFormat::Csv.media_type()
        ),
        ("csv", "text/csv")
    );
    assert_eq!(
        (
            AuditExportFormat::Pdf.extension(),
            AuditExportFormat::Pdf.media_type()
        ),
        ("pdf", "application/pdf")
    );
    assert_eq!(AuditExportFormat::Sql.media_type(), "application/sql");
    assert_eq!(
        "PDF".parse::<AuditExportFormat>().unwrap(),
        AuditExportFormat::Pdf
    );
    assert_eq!(
        "ACCOUNT_WITH_EVENTS".parse::<AuditExportScope>().unwrap(),
        AuditExportScope::AccountWithEvents
    );
}
