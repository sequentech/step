// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! fetchDocument on the migrated test database: the document's access
//! policy decides who may read it, and the caller gets the storage's URL or
//! a JSON error with a code.

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

fn reader(event: &Event, permissions: &[Permissions]) -> Claims {
    Claims::new(&event.tenant_id, "document-reader").roles(permissions)
}

fn request(event: &Event, document_id: &str) -> Value {
    json!({
        "election_event_id": event.election_event_id,
        "document_id": document_id,
    })
}

/// A document row of `event` with these annotations.
async fn document(
    services: &Services,
    event: &Event,
    annotations: Option<Value>,
) -> String {
    let id = event
        .document(&services.hasura, "record.json", "application/json", None)
        .await;
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.document SET annotations = $2 WHERE id = $1",
        &[&uuid::Uuid::parse_str(&id).unwrap(), &annotations],
    )
    .await;
    id
}

async fn fetch(
    services: &Services,
    claims: &Claims,
    body: &Value,
) -> (Status, Value) {
    let client = services.client().await;
    json(post(&client, "/fetch-document", claims, body).await).await
}

#[rocket::async_test]
async fn a_document_is_answered_with_its_storage_url() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let id = document(&services, &event, None).await;

    let (status, body) = fetch(
        &services,
        &reader(&event, &[Permissions::DOCUMENT_DOWNLOAD]),
        &request(&event, &id),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body, json!({"url": format!("https://documents.test/{id}")}));
}

#[rocket::async_test]
async fn a_missing_document_is_not_found_with_its_code() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;

    let (status, body) = fetch(
        &services,
        &reader(&event, &[Permissions::DOCUMENT_DOWNLOAD]),
        &request(&event, &uuid::Uuid::new_v4().to_string()),
    )
    .await;
    assert_eq!(status, Status::NotFound);
    assert_eq!(
        body,
        json!({"message": "Document not found", "extensions": {"code": "DocumentNotFound"}})
    );
}

#[rocket::async_test]
async fn a_document_with_voter_secrets_needs_the_secret_read_permission() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let id = document(
        &services,
        &event,
        Some(json!({"access": {"voter_secret_attributes": true}})),
    )
    .await;

    let (status, body) = fetch(
        &services,
        &reader(&event, &[Permissions::DOCUMENT_DOWNLOAD]),
        &request(&event, &id),
    )
    .await;
    assert_eq!(status, Status::Unauthorized, "{body}");
    assert_eq!(body["extensions"]["code"], "Unauthorized");

    let (status, body) = fetch(
        &services,
        &reader(
            &event,
            &[
                Permissions::DOCUMENT_DOWNLOAD,
                Permissions::VOTER_SECRET_ATTRIBUTE_READ,
            ],
        ),
        &request(&event, &id),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["url"], format!("https://documents.test/{id}"));
}

#[rocket::async_test]
async fn an_unreadable_access_policy_is_an_error_without_its_details() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let id = document(&services, &event, Some(json!({"access": "unreadable"})))
        .await;

    let (status, body) = fetch(
        &services,
        &reader(&event, &[Permissions::DOCUMENT_DOWNLOAD]),
        &request(&event, &id),
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(
        body,
        json!({"message": "Could not fetch the document.", "extensions": {"code": "InternalServerError"}})
    );
}

#[rocket::async_test]
async fn an_unreachable_database_is_an_error_without_its_details() {
    let services = Services::without_database();
    let tenant_id = uuid::Uuid::new_v4().to_string();
    let claims = Claims::new(&tenant_id, "document-reader")
        .roles([Permissions::DOCUMENT_DOWNLOAD]);

    let (status, body) = fetch(
        &services,
        &claims,
        &json!({"document_id": uuid::Uuid::new_v4().to_string()}),
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(body["extensions"]["code"], "InternalServerError");
    assert_eq!(body["message"], "Could not fetch the document.");
}
