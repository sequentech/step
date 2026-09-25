// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The report routes: the documents and reports they read on the migrated
//! test database, the task rows they write, the tasks they send and the
//! secret-attribute reads they log.

use crate::adapters::memory::electoral_log::{
    LoggedEntry, MemoryElectoralLogs,
};
use crate::adapters::memory::task_ledger::MemoryTaskLedger;
use crate::adapters::memory::task_queue::MemoryTaskQueue;
use crate::adapters::memory::vault::MemoryVault;
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use windmill::services::electoral_log::VoterSecretAttributeAction;

const USER_ID: &str = "report-admin";
const USERNAME: &str = "report.admin";
const TEMPLATE_ALIAS: &str = "voter-credentials";

fn reader(event: &Event, permissions: &[Permissions]) -> Claims {
    Claims::new(&event.tenant_id, USER_ID)
        .username(USERNAME)
        .roles(permissions)
}

async fn html_document(services: &Services, event: &Event) -> String {
    event
        .document(&services.hasura, "report.html", "text/html", Some(10))
        .await
}

fn render_request(event: &Event, document_id: &str) -> Value {
    json!({
        "document_id": document_id,
        "election_event_id": event.election_event_id,
        "tally_session_id": "tally-session",
    })
}

#[rocket::async_test]
async fn rendering_an_html_document_queues_its_pdf_behind_a_task_row() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let document_id = html_document(&services, &event).await;

    let (status, body) = json(
        post(
            &client,
            "/render-document-pdf",
            &reader(&event, &[Permissions::REPORT_READ]),
            &render_request(&event, &document_id),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");

    let tasks = services.ledger.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_type, "RENDER_DOCUMENT_PDF");
    assert_eq!(tasks[0].tenant_id, event.tenant_id);
    assert_eq!(
        tasks[0].election_event_id.as_deref(),
        Some(event.election_event_id.as_str())
    );
    assert_eq!(tasks[0].executed_by_user, USER_ID);
    assert_eq!(body["task_execution"]["id"], tasks[0].id.as_str());

    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "render_document_pdf");
    let kwargs = &sent[0].kwargs;
    // The PDF is written to a new document named in the response.
    assert_eq!(kwargs["document_id"], document_id.as_str());
    assert_eq!(kwargs["output_document_id"], body["document_id"]);
    assert_ne!(body["document_id"], document_id.as_str());
    assert_eq!(kwargs["task_execution"]["id"], tasks[0].id.as_str());
    assert_eq!(kwargs["executer_username"], USERNAME);
    assert_eq!(kwargs["tally_session_id"], "tally-session");
}

#[rocket::async_test]
async fn only_html_documents_with_a_media_type_are_rendered() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let csv = event
        .document(&services.hasura, "report.csv", "text/csv", Some(10))
        .await;
    let untyped = uuid::Uuid::new_v4();
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.document (id, tenant_id, election_event_id, name)
         VALUES ($1, $2, $3, 'untyped')",
        &[
            &untyped,
            &uuid::Uuid::parse_str(&event.tenant_id).unwrap(),
            &uuid::Uuid::parse_str(&event.election_event_id).unwrap(),
        ],
    )
    .await;
    let missing = uuid::Uuid::new_v4().to_string();

    for (document_id, expected) in [
        (
            csv.clone(),
            (
                Status::InternalServerError,
                "Invalid document type: text/csv".to_string(),
            ),
        ),
        (
            untyped.to_string(),
            (
                Status::InternalServerError,
                format!("Document {untyped}: missing media type"),
            ),
        ),
        (
            missing.clone(),
            (Status::NotFound, format!("Document not found: {missing}")),
        ),
    ] {
        let response = post(
            &client,
            "/render-document-pdf",
            &reader(&event, &[Permissions::REPORT_READ]),
            &render_request(&event, &document_id),
        )
        .await;
        assert_eq!(text(response).await, expected);
    }
    assert!(services.ledger.tasks().is_empty());
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_render_that_cannot_be_enqueued_leaves_its_task_row_in_progress() {
    // Unlike the user tasks, this route does not fail the row it wrote.
    let services = Services::on_test_database()
        .await
        .with_tasks(MemoryTaskQueue::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let document_id = html_document(&services, &event).await;

    let response = post(
        &client,
        "/render-document-pdf",
        &reader(&event, &[Permissions::REPORT_READ]),
        &render_request(&event, &document_id),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::InternalServerError,
            "Error publishing task render_document_pdf ForcedShutdown".into()
        )
    );
    let tasks = services.ledger.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].execution_status, "IN_PROGRESS");
}

#[rocket::async_test]
async fn a_render_whose_task_row_cannot_be_written_is_not_queued() {
    let services = Services::on_test_database()
        .await
        .with_ledger(MemoryTaskLedger::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let document_id = html_document(&services, &event).await;

    let response = post(
        &client,
        "/render-document-pdf",
        &reader(&event, &[Permissions::REPORT_READ]),
        &render_request(&event, &document_id),
    )
    .await;
    let (status, message) = text(response).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with(
            "Failed to insert task execution record: tasks_execution is unavailable"
        ),
        "{message}"
    );
    assert!(services.tasks.sent().is_empty());
}

fn ballot_images(event: &Event) -> Value {
    json!({
        "type": "BallotImages",
        "election_event_id": event.election_event_id,
        "election_id": "election",
        "tally_session_id": "tally-session",
    })
}

#[rocket::async_test]
async fn ballot_images_are_generated_by_a_task_for_their_event() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, body) = json(
        post(
            &client,
            "/generate-template",
            &reader(&event, &[Permissions::REPORT_READ]),
            &ballot_images(&event),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let tasks = services.ledger.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_type, "GENERATE_REPORT");
    assert_eq!(
        tasks[0].election_event_id.as_deref(),
        Some(event.election_event_id.as_str())
    );
    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "generate_template");
    assert_eq!(sent[0].kwargs["document_id"], body["document_id"]);
    assert_eq!(sent[0].kwargs["input"], ballot_images(&event));
}

#[rocket::async_test]
async fn ballot_images_that_cannot_be_enqueued_are_a_server_error() {
    let services = Services::on_test_database()
        .await
        .with_tasks(MemoryTaskQueue::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let response = post(
        &client,
        "/generate-template",
        &reader(&event, &[Permissions::REPORT_READ]),
        &ballot_images(&event),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::InternalServerError,
            "Error generating template: ForcedShutdown".into()
        )
    );
}

fn report_request(tenant_id: &str, report_id: &str) -> Value {
    json!({"report_id": report_id, "tenant_id": tenant_id, "report_mode": "REAL"})
}

#[rocket::async_test]
async fn a_report_is_generated_under_the_tenant_its_request_names() {
    // Pinned as found: the caller's own tenant is authorized, while the
    // report is read, and its task row written, in the body's tenant.
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let caller = rows::event(&services.hasura).await;
    let other = rows::event(&services.hasura).await;
    let report_id = other
        .report(&services.hasura, "CREDENTIALS", TEMPLATE_ALIAS)
        .await;

    let (status, body) = json(
        post(
            &client,
            "/generate-report",
            &reader(&caller, &[Permissions::REPORT_READ]),
            &report_request(&other.tenant_id, &report_id),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let tasks = services.ledger.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].tenant_id, other.tenant_id);
    let sent = services.tasks.sent();
    assert_eq!(sent[0].name, "generate_report");
    assert_eq!(sent[0].kwargs["report"]["tenant_id"], other.tenant_id);
    assert_eq!(sent[0].kwargs["document_id"], body["document_id"]);
    assert_eq!(sent[0].kwargs["may_read_secret_attributes"], false);
    assert!(services.electoral_log.entries().is_empty());
}

#[rocket::async_test]
async fn unknown_reports_and_report_types_are_refused() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let untyped = event
        .report(&services.hasura, "NOT_A_REPORT_TYPE", TEMPLATE_ALIAS)
        .await;

    let claims = reader(&event, &[Permissions::REPORT_READ]);
    let unknown = uuid::Uuid::new_v4().to_string();
    let response = post(
        &client,
        "/generate-report",
        &claims,
        &report_request(&event.tenant_id, &unknown),
    )
    .await;
    assert_eq!(
        text(response).await,
        (Status::NotFound, "Report not found".into())
    );
    let response = post(
        &client,
        "/generate-report",
        &claims,
        &report_request(&event.tenant_id, &untyped),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::BadRequest,
            "Invalid report type: Matching variant not found".into()
        )
    );
    assert!(services.ledger.tasks().is_empty());
}

async fn secret_report(services: &Services, event: &Event) -> String {
    event
        .template(
            &services.hasura,
            TEMPLATE_ALIAS,
            json!({"secret_attribute_names": ["national-id"]}),
        )
        .await;
    event
        .report(&services.hasura, "CREDENTIALS", TEMPLATE_ALIAS)
        .await
}

#[rocket::async_test]
async fn a_report_declaring_secret_attributes_is_logged_before_it_is_generated()
{
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let report_id = secret_report(&services, &event).await;

    let (status, body) = json(
        post(
            &client,
            "/generate-report",
            &reader(
                &event,
                &[
                    Permissions::REPORT_READ,
                    Permissions::VOTER_SECRET_ATTRIBUTE_READ,
                ],
            ),
            &report_request(&event.tenant_id, &report_id),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        services.electoral_log.entries(),
        vec![LoggedEntry::VoterSecretAttributes {
            election_event_id: event.election_event_id.clone(),
            admin_id: USER_ID.into(),
            action: VoterSecretAttributeAction::Report,
            voter_id: None,
            attribute_names: vec!["national-id".into()],
            document_id: None,
        }]
    );
    assert_eq!(
        services.tasks.sent()[0].kwargs["may_read_secret_attributes"],
        true
    );
}

#[rocket::async_test]
async fn a_report_declaring_secret_attributes_needs_their_read_permission() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let report_id = secret_report(&services, &event).await;

    let response = post(
        &client,
        "/generate-report",
        &reader(&event, &[Permissions::REPORT_READ]),
        &report_request(&event.tenant_id, &report_id),
    )
    .await;
    let (status, _) = text(response).await;
    assert_eq!(status, Status::Unauthorized);
    assert!(services.electoral_log.entries().is_empty());
    assert!(services.ledger.tasks().is_empty());
}

#[rocket::async_test]
async fn a_secret_attribute_report_that_cannot_be_logged_is_not_generated() {
    let services = Services::on_test_database()
        .await
        .with_electoral_log(MemoryElectoralLogs::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let report_id = secret_report(&services, &event).await;

    let response = post(
        &client,
        "/generate-report",
        &reader(
            &event,
            &[
                Permissions::REPORT_READ,
                Permissions::VOTER_SECRET_ATTRIBUTE_READ,
            ],
        ),
        &report_request(&event.tenant_id, &report_id),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::InternalServerError,
            "Failed to record the secret-attribute electoral-log entry: the bulletin board is unreachable".into()
        )
    );
    assert!(services.ledger.tasks().is_empty());
    assert!(services.tasks.sent().is_empty());
}

fn encrypt_request(event: &Event) -> Value {
    json!({"election_event_id": event.election_event_id, "password": "correct horse"})
}

#[rocket::async_test]
async fn a_report_password_the_vault_accepts_answers_a_new_document_id() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, body) = json(
        post(
            &client,
            "/encrypt-report",
            &reader(&event, &[Permissions::REPORT_WRITE]),
            &encrypt_request(&event),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok);
    assert!(
        uuid::Uuid::parse_str(body["document_id"].as_str().unwrap()).is_ok()
    );
    assert_eq!(body["error_msg"], Value::Null);
}

#[rocket::async_test]
async fn a_report_password_the_vault_refuses_is_a_server_error() {
    let services = Services::on_test_database()
        .await
        .with_vault(MemoryVault::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let response = post(
        &client,
        "/encrypt-report",
        &reader(&event, &[Permissions::REPORT_WRITE]),
        &encrypt_request(&event),
    )
    .await;
    assert_eq!(
        text(response).await,
        (Status::InternalServerError, "the vault is sealed".into())
    );
}

#[rocket::async_test]
async fn report_routes_answer_500_when_the_database_is_unreachable() {
    let services = Services::without_database();
    let client = services.client().await;
    let event = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    for (path, permission, body) in [
        (
            "/render-document-pdf",
            Permissions::REPORT_READ,
            render_request(&event, "document"),
        ),
        (
            "/generate-template",
            Permissions::REPORT_READ,
            ballot_images(&event),
        ),
        (
            "/generate-report",
            Permissions::REPORT_READ,
            report_request(&event.tenant_id, "report"),
        ),
        (
            "/encrypt-report",
            Permissions::REPORT_WRITE,
            encrypt_request(&event),
        ),
    ] {
        let response =
            post(&client, path, &reader(&event, &[permission]), &body).await;
        let (status, message) = text(response).await;
        assert_eq!(status, Status::InternalServerError, "{path}");
        assert!(message.contains("Connection refused"), "{path}: {message}");
    }
    assert!(services.ledger.tasks().is_empty());
}
