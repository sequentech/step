// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tenant log's routes: who may list and export it, and the exports the
//! requests ask for.

use super::*;
use crate::route_services::{post, text, Services};
use crate::routes::export_election_event_logs::ExportElectionEventInput;
use crate::services::authorization::test_claims::{
    admin_claims, CALLER_TENANT_ID,
};
use crate::test_claims::Claims;
use serde_json::json;

const EVENT_ID: &str = "0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9";

fn input(value: serde_json::Value) -> ExportTenantLogsInput {
    serde_json::from_value(value).unwrap()
}

#[test]
fn the_export_is_of_the_callers_tenant_log() {
    let claims = admin_claims(CALLER_TENANT_ID, &["tenant-logs-export"]);
    let request = tenant_export_request(
        &claims,
        &input(json!({
            "format": "SQL",
            "scope": "ACCOUNT_WITH_EVENTS",
            "username": "ada",
            "created_from": "2028-02-01T00:00:00+08:00",
            "time_zone": "Asia/Manila",
        })),
    )
    .unwrap();
    assert_eq!(request.tenant_id, CALLER_TENANT_ID);
    assert_eq!(request.election_event_id, None);
    assert_eq!(request.format, AuditExportFormat::Sql);
    assert_eq!(request.scope, AuditExportScope::AccountWithEvents);
    assert_eq!(request.username.as_deref(), Some("ada"));
    assert_eq!(request.requested_by.user_id.as_deref(), Some("admin"));
}

#[test]
fn a_request_the_export_cannot_serve_is_a_bad_request() {
    let claims = admin_claims(CALLER_TENANT_ID, &["tenant-logs-export"]);
    for body in [
        json!({"format": "XML", "scope": "TENANT_LOG"}),
        json!({"format": "CSV", "scope": "EVERYTHING"}),
        // A scope of an election event's log.
        json!({"format": "CSV", "scope": "ALL"}),
        // One account, without saying which.
        json!({"format": "CSV", "scope": "ACCOUNT"}),
        json!({"format": "CSV", "scope": "TENANT_LOG", "time_zone": "Mars/Olympus"}),
        json!({"format": "CSV", "scope": "TENANT_LOG", "created_to": "tomorrow"}),
    ] {
        let (status, _) =
            tenant_export_request(&claims, &input(body.clone())).unwrap_err();
        assert_eq!(status, Status::BadRequest, "{body}");
    }
}

#[rocket::async_test]
async fn only_tenant_log_exporters_export_the_tenant_log() {
    let services = Services::without_database();
    let client = services.client().await;
    let body = json!({"format": "CSV", "scope": "TENANT_LOG"});
    // Reading the tenant log, or exporting an event's, is not enough.
    for roles in [
        vec![],
        vec![Permissions::TENANT_LOGS_READ, Permissions::LOGS_EXPORT],
    ] {
        let claims = Claims::new(CALLER_TENANT_ID, "auditor").roles(&roles);
        let (status, _) =
            text(post(&client, "/export-tenant-logs", &claims, &body).await)
                .await;
        assert_eq!(status, Status::Forbidden, "{roles:?}");
    }
    // An exporter's bad request is refused before anything is queued.
    let exporter = Claims::new(CALLER_TENANT_ID, "auditor")
        .roles(&[Permissions::TENANT_LOGS_EXPORT]);
    let bad = json!({"format": "CSV", "scope": "ACCOUNT"});
    let (status, reason) =
        text(post(&client, "/export-tenant-logs", &exporter, &bad).await).await;
    assert_eq!(status, Status::BadRequest);
    assert!(reason.contains("username"), "{reason}");
}

#[rocket::async_test]
async fn only_tenant_log_readers_list_the_tenant_log() {
    let services = Services::without_database();
    let client = services.client().await;
    // No election event: the tenant log.
    for body in [
        json!({"tenant_id": CALLER_TENANT_ID}),
        json!({"tenant_id": CALLER_TENANT_ID, "election_event_id": null}),
        json!({"tenant_id": CALLER_TENANT_ID, "election_event_id": ""}),
    ] {
        let reader = Claims::new(CALLER_TENANT_ID, "sbei")
            .roles(&[Permissions::LOGS_READ]);
        let (status, _) =
            text(post(&client, "/electoral-log", &reader, &body).await).await;
        assert_eq!(status, Status::Forbidden, "{body}");
    }
}

fn event_input(value: serde_json::Value) -> ExportElectionEventInput {
    serde_json::from_value(value).unwrap()
}

#[test]
fn a_scoped_or_sql_export_of_an_event_is_an_audit_export() {
    let claims = admin_claims(CALLER_TENANT_ID, &["logs-export"]);
    // The whole log as CSV or PDF stays the activity logs report.
    for body in [
        json!({"election_event_id": EVENT_ID, "format": "CSV"}),
        json!({"election_event_id": EVENT_ID, "format": "PDF", "scope": "ALL"}),
        json!({"election_event_id": EVENT_ID, "format": "CSV", "scope": " "}),
    ] {
        assert_eq!(event_input(body).audit_export(&claims).unwrap(), None);
    }
    let sql =
        event_input(json!({"election_event_id": EVENT_ID, "format": "sql"}))
            .audit_export(&claims)
            .unwrap()
            .unwrap();
    assert_eq!(sql.format, AuditExportFormat::Sql);
    assert_eq!(sql.scope, AuditExportScope::All);
    assert_eq!(sql.election_event_id.as_deref(), Some(EVENT_ID));
    assert_eq!(sql.tenant_id, CALLER_TENANT_ID);

    let account = event_input(json!({
        "election_event_id": EVENT_ID, "format": "CSV", "scope": "ACCOUNT", "username": "ada",
    }))
    .audit_export(&claims)
    .unwrap()
    .unwrap();
    assert_eq!(account.scope, AuditExportScope::Account);
    assert_eq!(account.username.as_deref(), Some("ada"));

    for body in [
        json!({"election_event_id": EVENT_ID, "format": "CSV", "scope": "POST"}),
        json!({"election_event_id": EVENT_ID, "format": "CSV", "scope": "ELECTION"}),
        json!({"election_event_id": EVENT_ID, "format": "CSV", "scope": "TENANT_LOG"}),
        json!({"election_event_id": EVENT_ID, "format": "XML", "scope": "AREA", "area_id": "a"}),
        json!({"election_event_id": "event", "format": "SQL"}),
    ] {
        let (status, _) =
            event_input(body.clone()).audit_export(&claims).unwrap_err();
        assert_eq!(status, Status::BadRequest, "{body}");
    }
}
