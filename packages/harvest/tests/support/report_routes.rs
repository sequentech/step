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
async fn a_render_the_task_queues_cannot_take_writes_no_task_row() {
    let services = Services::on_test_database()
        .await
        .with_tasks(MemoryTaskQueue::unreachable());
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
            "The task queues are not available: Timed out connecting to the task-queue database"
                .into()
        )
    );
    assert!(services.ledger.tasks().is_empty());
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
            &reader(&event, &[Permissions::REPORT_GENERATE]),
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
        &reader(&event, &[Permissions::REPORT_GENERATE]),
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
            Permissions::REPORT_GENERATE,
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
        (
            "/generate-transmission-report",
            Permissions::TRANSMISSION_REPORT_GENERATE,
            transmission_request(
                &event,
                Some(&uuid::Uuid::new_v4().to_string()),
                Some(&uuid::Uuid::new_v4().to_string()),
            ),
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

/// Saves a `Required` rule of two signatures for `action` in `event`.
async fn signing_rule(
    services: &Services,
    event: &Event,
    action: sequent_core::signing::SigningAction,
) {
    use sequent_core::signing::{
        RequesterSigning, SigningRequirement, SigningRule,
    };
    let mut client = services.hasura.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    windmill::postgres::signing::upsert_signing_rule(
        &tx,
        uuid::Uuid::parse_str(&event.tenant_id).unwrap(),
        uuid::Uuid::parse_str(&event.election_event_id).unwrap(),
        &SigningRule {
            action,
            requirement: SigningRequirement::Required,
            signatures: 2,
            requester_signing: RequesterSigning::NotAllowed,
            expires_minutes: Some(60),
            revision: 0,
        },
        0,
        "manager",
        Some("Configuration Manager"),
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
}

/// While its action needs signatures, a real report is not generated here,
/// and nothing is queued: the election returns are the tally's, produced
/// and signed per Post (409), and a participation report must be a Post's
/// (400). Without the rule the same reports are generated as before.
#[rocket::async_test]
async fn a_report_whose_action_needs_signatures_is_not_generated_here() {
    use sequent_core::signing::SigningAction;
    for (report_type, action, status, message) in [
        (
            "ELECTORAL_RESULTS",
            SigningAction::GenerateElectionReturns,
            Status::Conflict,
            "This report needs signatures: the tally produces it for each Post, to be signed there.",
        ),
        (
            "PARTICIPATION_REPORT",
            SigningAction::GenerateReports,
            Status::BadRequest,
            "This report needs signatures: generate it for a Post.",
        ),
    ] {
        let services = Services::on_test_database().await;
        let client = services.client().await;
        let event = rows::event(&services.hasura).await;
        let report_id = event
            .report(&services.hasura, report_type, TEMPLATE_ALIAS)
            .await;
        let claims = reader(&event, &[Permissions::REPORT_READ]);
        let request = report_request(&event.tenant_id, &report_id);

        let (generated, body) =
            json(post(&client, "/generate-report", &claims, &request).await)
                .await;
        assert_eq!(generated, Status::Ok, "{report_type}: {body}");
        assert_eq!(services.ledger.tasks().len(), 1, "{report_type}");

        signing_rule(&services, &event, action).await;
        let response =
            post(&client, "/generate-report", &claims, &request).await;
        assert_eq!(
            text(response).await,
            (status, message.to_string()),
            "{report_type}"
        );
        assert_eq!(services.ledger.tasks().len(), 1, "{report_type}");
    }
}

// The Transmission Report of a Post (an election), downloaded from the tally.

fn transmission_request(
    event: &Event,
    election_id: Option<&str>,
    tally_session_id: Option<&str>,
) -> Value {
    json!({
        "tenant_id": event.tenant_id,
        "election_event_id": event.election_event_id,
        "election_id": election_id,
        "tally_session_id": tally_session_id,
    })
}

fn transmission_officer(event: &Event) -> Claims {
    reader(event, &[Permissions::TRANSMISSION_REPORT_GENERATE])
}

#[rocket::async_test]
async fn a_transmission_report_is_generated_for_a_tally_sessions_election() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    let session_id = event
        .tally_session(
            &services.hasura,
            std::slice::from_ref(&election_id),
            "SUCCESS",
        )
        .await;

    let (status, body) = json(
        post(
            &client,
            "/generate-transmission-report",
            &transmission_officer(&event),
            &transmission_request(
                &event,
                Some(&election_id),
                Some(&session_id),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");

    let tasks = services.ledger.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_type, "GENERATE_TRANSMISSION_REPORT");
    assert_eq!(tasks[0].tenant_id, event.tenant_id);
    assert_eq!(
        tasks[0].election_event_id.as_deref(),
        Some(event.election_event_id.as_str())
    );
    assert_eq!(body["task_execution"]["id"], tasks[0].id.as_str());
    assert_eq!(body["encryption_policy"], "unencrypted");

    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "generate_report");
    let kwargs = &sent[0].kwargs;
    assert_eq!(kwargs["report"]["report_type"], "TRANSMISSION_REPORT");
    assert_eq!(kwargs["report"]["tenant_id"], event.tenant_id);
    assert_eq!(
        kwargs["report"]["election_event_id"],
        event.election_event_id
    );
    assert_eq!(kwargs["report"]["election_id"], election_id.as_str());
    assert_eq!(kwargs["report"]["template_alias"], Value::Null);
    assert_eq!(kwargs["report_mode"], "REAL");
    assert_eq!(kwargs["document_id"], body["document_id"]);
    assert_eq!(kwargs["tally_session_id"], session_id.as_str());
    assert_eq!(kwargs["executer_username"], USERNAME);
    assert_eq!(kwargs["may_read_secret_attributes"], false);
}

#[rocket::async_test]
async fn a_transmission_report_uses_the_template_its_election_saved() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    let session_id = event
        .tally_session(
            &services.hasura,
            std::slice::from_ref(&election_id),
            "SUCCESS",
        )
        .await;
    let report_id = event
        .report(&services.hasura, "TRANSMISSION_REPORT", "post-transmission")
        .await;
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.report SET election_id = $2 WHERE id = $1",
        &[
            &uuid::Uuid::parse_str(&report_id).unwrap(),
            &uuid::Uuid::parse_str(&election_id).unwrap(),
        ],
    )
    .await;

    let (status, body) = json(
        post(
            &client,
            "/generate-transmission-report",
            &transmission_officer(&event),
            &transmission_request(
                &event,
                Some(&election_id),
                Some(&session_id),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let report = &services.tasks.sent()[0].kwargs["report"];
    assert_eq!(report["id"], report_id.as_str());
    assert_eq!(report["template_alias"], "post-transmission");
}

#[rocket::async_test]
async fn a_transmission_report_needs_an_election_and_a_tally_session_of_it() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    let other_election = event.election(&services.hasura).await;
    let session_id = event
        .tally_session(
            &services.hasura,
            std::slice::from_ref(&other_election),
            "SUCCESS",
        )
        .await;
    let unknown = uuid::Uuid::new_v4().to_string();
    let claims = transmission_officer(&event);

    for (request, expected) in [
        (
            transmission_request(&event, None, Some(&session_id)),
            (
                Status::BadRequest,
                "The transmission report is generated for one election".to_string(),
            ),
        ),
        (
            transmission_request(&event, Some(&election_id), None),
            (
                Status::BadRequest,
                "Tally session id is required to generate transmission report"
                    .to_string(),
            ),
        ),
        (
            transmission_request(&event, Some("not-a-uuid"), Some(&session_id)),
            (Status::BadRequest, "Invalid election id".to_string()),
        ),
        (
            transmission_request(&event, Some(&election_id), Some("not-a-uuid")),
            (Status::BadRequest, "Invalid tally session id".to_string()),
        ),
        (
            transmission_request(&event, Some(&election_id), Some(&unknown)),
            (Status::NotFound, format!("Tally session {unknown} not found")),
        ),
        (
            transmission_request(&event, Some(&election_id), Some(&session_id)),
            (
                Status::BadRequest,
                format!(
                    "Tally session {session_id} does not count election {election_id}"
                ),
            ),
        ),
    ] {
        let response = post(
            &client,
            "/generate-transmission-report",
            &claims,
            &request,
        )
        .await;
        assert_eq!(text(response).await, expected, "{request}");
    }
    assert!(services.ledger.tasks().is_empty());
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_transmission_report_of_another_tenant_or_without_the_role_is_refused(
) {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let other = rows::event(&services.hasura).await;
    let election_id = other.election(&services.hasura).await;
    let session_id = other
        .tally_session(
            &services.hasura,
            std::slice::from_ref(&election_id),
            "SUCCESS",
        )
        .await;
    let request =
        transmission_request(&other, Some(&election_id), Some(&session_id));

    for claims in [
        transmission_officer(&event),
        reader(&other, &[Permissions::REPORT_READ]),
    ] {
        let response =
            post(&client, "/generate-transmission-report", &claims, &request)
                .await;
        assert_eq!(response.status(), Status::Unauthorized);
    }
    assert!(services.ledger.tasks().is_empty());
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_transmission_report_of_a_labelled_election_needs_the_label() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election_id = event.labelled_election(&services.hasura, "madrid").await;
    let session_id = event
        .tally_session(
            &services.hasura,
            std::slice::from_ref(&election_id),
            "SUCCESS",
        )
        .await;

    let response = post(
        &client,
        "/generate-transmission-report",
        &transmission_officer(&event),
        &transmission_request(&event, Some(&election_id), Some(&session_id)),
    )
    .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(services.ledger.tasks().is_empty());
}

#[rocket::async_test]
async fn a_transmission_report_the_task_queues_cannot_take_is_a_server_error() {
    let services = Services::on_test_database()
        .await
        .with_tasks(MemoryTaskQueue::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    let session_id = event
        .tally_session(
            &services.hasura,
            std::slice::from_ref(&election_id),
            "SUCCESS",
        )
        .await;

    let response = post(
        &client,
        "/generate-transmission-report",
        &transmission_officer(&event),
        &transmission_request(&event, Some(&election_id), Some(&session_id)),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::InternalServerError,
            "Error generating report: ForcedShutdown".into()
        )
    );
}

/// An event with an election and a finished tally session that counts it.
async fn tallied_election(services: &Services) -> (Event, String, String) {
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    let session_id = event
        .tally_session(
            &services.hasura,
            std::slice::from_ref(&election_id),
            "SUCCESS",
        )
        .await;
    (event, election_id, session_id)
}

#[rocket::async_test]
async fn a_transmission_report_without_task_queues_writes_no_task_row() {
    let services = Services::on_test_database()
        .await
        .with_tasks(MemoryTaskQueue::unreachable());
    let client = services.client().await;
    let (event, election_id, session_id) = tallied_election(&services).await;

    let response = post(
        &client,
        "/generate-transmission-report",
        &transmission_officer(&event),
        &transmission_request(&event, Some(&election_id), Some(&session_id)),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::InternalServerError,
            "The task queues are not available: Timed out connecting to the task-queue database"
                .into()
        )
    );
    assert!(services.ledger.tasks().is_empty());
}

#[rocket::async_test]
async fn a_transmission_report_whose_task_row_cannot_be_written_is_not_queued()
{
    let services = Services::on_test_database()
        .await
        .with_ledger(MemoryTaskLedger::refusing());
    let client = services.client().await;
    let (event, election_id, session_id) = tallied_election(&services).await;

    let (status, message) = text(
        post(
            &client,
            "/generate-transmission-report",
            &transmission_officer(&event),
            &transmission_request(
                &event,
                Some(&election_id),
                Some(&session_id),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with(
            "Failed to insert task execution record: tasks_execution is unavailable"
        ),
        "{message}"
    );
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_transmission_report_names_a_caller_without_a_username_by_their_id() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, election_id, session_id) = tallied_election(&services).await;

    let (status, body) = json(
        post(
            &client,
            "/generate-transmission-report",
            &Claims::new(&event.tenant_id, USER_ID)
                .roles([Permissions::TRANSMISSION_REPORT_GENERATE]),
            &transmission_request(
                &event,
                Some(&election_id),
                Some(&session_id),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        services.tasks.sent()[0].kwargs["executer_username"],
        USER_ID
    );
    assert_eq!(services.ledger.tasks()[0].executed_by_user, USER_ID);
}

#[rocket::async_test]
async fn a_tally_session_id_the_database_cannot_read_is_a_server_error() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, election_id, _) = tallied_election(&services).await;
    // A UUID, but not a version 4 one: tally sessions are looked up by those.
    let not_v4 = "00000000-0000-0000-0000-000000000001";

    let (status, message) = text(
        post(
            &client,
            "/generate-transmission-report",
            &transmission_officer(&event),
            &transmission_request(&event, Some(&election_id), Some(not_v4)),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError, "{message}");
    assert!(services.ledger.tasks().is_empty());
}
