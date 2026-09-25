// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Voter information letters: the checks before the task row, and the
//! document password and task each letter needs.

use crate::adapters::memory::electoral_log::{
    LoggedEntry, MemoryElectoralLogs,
};
use crate::adapters::memory::identity::LocalIdentityAdmin;
use crate::adapters::memory::task_ledger::MemoryTaskLedger;
use crate::adapters::memory::task_queue::MemoryTaskQueue;
use crate::adapters::memory::vault::MemoryVault;
use crate::route_services::http::{Exchange, HttpServer};
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::services::keycloak::ParsedRealmPasswordPolicy;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use windmill::services::electoral_log::VoterSecretAttributeAction;

const VOTER_ID: &str = "voter-1";
const ADMIN_ID: &str = "letter-admin";

fn letter_admin(event: &Event, extra: &[Permissions]) -> Claims {
    let mut permissions = vec![
        Permissions::VOTER_INFORMATION_LETTER,
        Permissions::DOCUMENT_PASSWORD_READ,
    ];
    permissions.extend_from_slice(extra);
    Claims::new(&event.tenant_id, ADMIN_ID).roles(permissions)
}

fn generation_policy() -> ParsedRealmPasswordPolicy {
    ParsedRealmPasswordPolicy {
        managed_rules_present: true,
        minimum_length: Some(12),
        required_lowercase: Some(1),
        ..Default::default()
    }
}

/// Keycloak knows the voter, and the realm's policy can generate passwords.
fn keycloak(peer: &HttpServer) -> LocalIdentityAdmin {
    LocalIdentityAdmin {
        url: Some(peer.url.clone()),
        password_policy: Some(generation_policy()),
        ..Default::default()
    }
}

fn voter_lookup(event: &Event, status: u16) -> Exchange {
    Exchange::json(
        "GET",
        &format!("/admin/realms/{}/users/{VOTER_ID}", event.realm()),
        status,
        json!({"id": VOTER_ID, "username": "voter.one"}),
    )
}

async fn letter<'c>(
    client: &'c Client,
    event: &Event,
    claims: &Claims,
) -> LocalResponse<'c> {
    post(
        client,
        "/generate-voter-information-letter",
        claims,
        &json!({"election_event_id": event.election_event_id, "voter_id": VOTER_ID}),
    )
    .await
}

fn failure(status: Status, message: &str, code: &str) -> (Status, Value) {
    (
        status,
        json!({"message": message, "extensions": {"code": code}}),
    )
}

#[rocket::async_test]
async fn a_letter_is_generated_by_a_task_with_its_document_password_saved() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let peer = HttpServer::start(vec![voter_lookup(&event, 200)]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let (status, body) =
        json(letter(&client, &event, &letter_admin(&event, &[])).await).await;
    assert_eq!(status, Status::Ok, "{body}");
    peer.finish();

    // The response names the password the vault keeps for its document.
    let saved = services.vault.document_passwords.lock().unwrap().clone();
    assert_eq!(
        saved,
        vec![(
            body["document_id"].as_str().unwrap().to_string(),
            body["pdf_password"].as_str().unwrap().to_string()
        )]
    );
    let tasks = services.ledger.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_type, "VOTER_INFORMATION_LETTER");
    assert_eq!(body["task_execution"]["id"], tasks[0].id.as_str());
    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "generate_voter_information_letter");
    let kwargs = &sent[0].kwargs;
    assert_eq!(kwargs["voter_id"], VOTER_ID);
    assert_eq!(kwargs["document_id"], body["document_id"]);
    assert_eq!(kwargs["password_secret_id"], "secret-1");
    assert_eq!(kwargs["password_change_initiator"]["user_id"], ADMIN_ID);
    assert_eq!(kwargs["may_read_secret_attributes"], false);
}

#[rocket::async_test]
async fn a_letter_for_a_voter_keycloak_does_not_know_is_not_generated() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let peer = HttpServer::start(vec![voter_lookup(&event, 404)]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    assert_eq!(
        json(letter(&client, &event, &letter_admin(&event, &[])).await).await,
        failure(
            Status::NotFound,
            "Voter not found",
            "VoterInformationLetterUnavailable"
        )
    );
    peer.finish();
    assert!(services.ledger.tasks().is_empty());
}

#[rocket::async_test]
async fn a_letter_needs_a_password_policy_it_can_generate_passwords_for() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    let services = base.with_identity(LocalIdentityAdmin {
        password_policy: Some(ParsedRealmPasswordPolicy::default()),
        ..Default::default()
    });
    let client = services.client().await;
    assert_eq!(
        json(letter(&client, &event, &letter_admin(&event, &[])).await).await,
        failure(
            Status::BadRequest,
            "Password Policy is not configured. Set it under Election Event Data before generating a letter.",
            "PasswordPolicyNotConfigured"
        )
    );

    // A policy Keycloak cannot return is a server error.
    let services = Services::on_test_database()
        .await
        .with_identity(LocalIdentityAdmin::default());
    let client = services.client().await;
    assert_eq!(
        json(letter(&client, &event, &letter_admin(&event, &[])).await).await,
        failure(
            Status::InternalServerError,
            "Failed to read the election event password policy",
            "InternalServerError"
        )
    );
}

async fn declare_secret_attributes(services: &Services, event: &Event) {
    event
        .template(
            &services.hasura,
            "credentials",
            json!({"secret_attribute_names": ["national-id"]}),
        )
        .await;
    event
        .report(&services.hasura, "CREDENTIALS", "credentials")
        .await;
}

#[rocket::async_test]
async fn a_letter_revealing_secret_attributes_is_logged_for_its_voter() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    declare_secret_attributes(&base, &event).await;
    let peer = HttpServer::start(vec![voter_lookup(&event, 200)]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let claims =
        letter_admin(&event, &[Permissions::VOTER_SECRET_ATTRIBUTE_READ]);
    let (status, _) = json(letter(&client, &event, &claims).await).await;
    assert_eq!(status, Status::Ok);
    peer.finish();
    assert_eq!(
        services.electoral_log.entries(),
        vec![LoggedEntry::VoterSecretAttributes {
            election_event_id: event.election_event_id.clone(),
            admin_id: ADMIN_ID.into(),
            action: VoterSecretAttributeAction::Report,
            voter_id: Some(VOTER_ID.into()),
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
async fn secret_attributes_in_a_letter_need_their_permission_and_a_log_entry() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    declare_secret_attributes(&services, &event).await;
    let client = services.client().await;
    assert_eq!(
        json(letter(&client, &event, &letter_admin(&event, &[])).await).await,
        failure(Status::Forbidden, "Authorization failed", "Unauthorized")
    );

    let services = Services::on_test_database()
        .await
        .with_electoral_log(MemoryElectoralLogs::refusing());
    let client = services.client().await;
    let claims =
        letter_admin(&event, &[Permissions::VOTER_SECRET_ATTRIBUTE_READ]);
    assert_eq!(
        json(letter(&client, &event, &claims).await).await,
        failure(
            Status::InternalServerError,
            "Failed to record the secret-attribute electoral-log entry",
            "InternalServerError"
        )
    );
    assert!(services.ledger.tasks().is_empty());
}

#[rocket::async_test]
async fn a_letter_whose_task_cannot_proceed_fails_that_task() {
    for (services, message, logged) in [
        (
            Services::on_test_database()
                .await
                .with_vault(MemoryVault::refusing()),
            "Failed to prepare Voter Information Letter generation",
            "Error: Failed to prepare Voter Information Letter generation",
        ),
        (
            Services::on_test_database()
                .await
                .with_tasks(MemoryTaskQueue::refusing()),
            "Failed to enqueue Voter Information Letter task",
            "Error: Failed to enqueue Voter Information Letter generation",
        ),
    ] {
        let event = rows::event(&services.hasura).await;
        let peer = HttpServer::start(vec![voter_lookup(&event, 200)]);
        let services = services.with_identity(keycloak(&peer));
        let client = services.client().await;

        assert_eq!(
            json(letter(&client, &event, &letter_admin(&event, &[])).await)
                .await,
            failure(
                Status::InternalServerError,
                message,
                "InternalServerError"
            )
        );
        peer.finish();
        let tasks = services.ledger.tasks();
        assert_eq!(tasks[0].execution_status, "FAILED", "{message}");
        assert_eq!(tasks[0].logs, Some(json!([logged])), "{message}");
    }
}

#[rocket::async_test]
async fn a_letter_stops_at_the_first_unavailable_backend() {
    let event = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    let services = Services::without_database();
    let client = services.client().await;
    assert_eq!(
        json(letter(&client, &event, &letter_admin(&event, &[])).await).await,
        failure(
            Status::InternalServerError,
            "Failed to read Voter Information Letter template",
            "InternalServerError"
        )
    );

    // Keycloak refuses the admin client.
    let services =
        Services::on_test_database()
            .await
            .with_identity(LocalIdentityAdmin {
                password_policy: Some(generation_policy()),
                ..Default::default()
            });
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    assert_eq!(
        json(letter(&client, &event, &letter_admin(&event, &[])).await).await,
        failure(
            Status::InternalServerError,
            "Failed to initialize Keycloak client",
            "InternalServerError"
        )
    );

    let base = Services::on_test_database()
        .await
        .with_ledger(MemoryTaskLedger::refusing());
    let peer = HttpServer::start(vec![voter_lookup(&event, 200)]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;
    assert_eq!(
        json(letter(&client, &event, &letter_admin(&event, &[])).await).await,
        failure(
            Status::InternalServerError,
            "Failed to create Voter Information Letter task",
            "InternalServerError"
        )
    );
    peer.finish();
    assert!(services.tasks.sent().is_empty());
}
