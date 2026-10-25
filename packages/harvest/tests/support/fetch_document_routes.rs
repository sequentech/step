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

/// A completed tenant configuration export, bound by the worker's task result.
/// The filename alone is never an authorization grant.
async fn tenant_backup(
    services: &Services,
    event: &Event,
) -> (String, uuid::Uuid) {
    let id = document(services, event, None).await;
    let document_id = uuid::Uuid::parse_str(&id).unwrap();
    let tenant_id = uuid::Uuid::parse_str(&event.tenant_id).unwrap();
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.document SET election_event_id = NULL,
            name = $2, media_type = 'application/zip', is_public = false
         WHERE id = $1",
        &[
            &document_id,
            &format!("export-tenant-config-{}.zip", event.tenant_id),
        ],
    )
    .await;
    let task_id = uuid::Uuid::new_v4();
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.tasks_execution
            (id, tenant_id, election_event_id, name, type, execution_status,
             annotations, executed_by_user)
         VALUES ($1, $2, NULL, 'Export Tenant Config', 'EXPORT_TENANT_CONFIG',
             'SUCCESS', $3, 'synthetic-tenant-admin')",
        &[&task_id, &tenant_id, &json!({"document_id": id})],
    )
    .await;
    (id, task_id)
}

#[rocket::async_test]
async fn tenant_config_exporter_can_download_its_completed_backup_without_document_download(
) {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let (id, _) = tenant_backup(&services, &event).await;

    let (status, body) = fetch(
        &services,
        &reader(&event, &[Permissions::TENANT_READ]),
        &json!({"document_id": id}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body, json!({"url": format!("https://documents.test/{id}")}));
}

#[rocket::async_test]
async fn tenant_backup_download_rejects_every_broken_binding() {
    let services = Services::on_test_database().await;
    // Each case starts with the same valid completed backup and changes one
    // binding. Neither a guessed filename nor mutable document metadata grants
    // access to another object or an election document.
    for change in [
        "missing task",
        "wrong task type",
        "unfinished task",
        "wrong task tenant",
        "event task",
        "different result document",
        "event document",
        "renamed object",
        "traversal filename",
        "wrong media type",
        "public object",
        "missing name",
        "missing media type",
        "missing visibility",
        "request event",
    ] {
        let event = rows::event(&services.hasura).await;
        let (id, task_id) = tenant_backup(&services, &event).await;
        let document_id = uuid::Uuid::parse_str(&id).unwrap();
        let event_id = uuid::Uuid::parse_str(&event.election_event_id).unwrap();
        let mut body = json!({"document_id": id});
        match change {
            "missing task" => {
                rows::execute(
                    &services.hasura,
                    "DELETE FROM sequent_backend.tasks_execution WHERE id=$1",
                    &[&task_id],
                )
                .await;
            }
            "wrong task type" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.tasks_execution SET type='EXPORT_ELECTION_EVENT' WHERE id=$1", &[&task_id]).await;
            }
            "unfinished task" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.tasks_execution SET execution_status='IN_PROGRESS' WHERE id=$1", &[&task_id]).await;
            }
            "wrong task tenant" => {
                let other = rows::event(&services.hasura).await;
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.tasks_execution SET tenant_id=$2 WHERE id=$1",
                    &[&task_id, &uuid::Uuid::parse_str(&other.tenant_id).unwrap()]).await;
            }
            "event task" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.tasks_execution SET election_event_id=$2 WHERE id=$1",
                    &[&task_id, &event_id]).await;
            }
            "different result document" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.tasks_execution SET annotations=$2 WHERE id=$1",
                    &[&task_id, &json!({"document_id": uuid::Uuid::new_v4().to_string()})]).await;
            }
            "event document" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.document SET election_event_id=$2 WHERE id=$1",
                    &[&document_id, &event_id]).await;
            }
            "renamed object" | "traversal filename" => {
                let name = if change == "renamed object" {
                    "other-backup.zip"
                } else {
                    "../../election/voters.zip"
                };
                rows::execute(
                    &services.hasura,
                    "UPDATE sequent_backend.document SET name=$2 WHERE id=$1",
                    &[&document_id, &name],
                )
                .await;
            }
            "wrong media type" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.document SET media_type='text/csv' WHERE id=$1", &[&document_id]).await;
            }
            "public object" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.document SET is_public=true WHERE id=$1", &[&document_id]).await;
            }
            "missing name" => {
                rows::execute(
                    &services.hasura,
                    "UPDATE sequent_backend.document SET name=NULL WHERE id=$1",
                    &[&document_id],
                )
                .await;
            }
            "missing media type" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.document SET media_type=NULL WHERE id=$1", &[&document_id]).await;
            }
            "missing visibility" => {
                rows::execute(&services.hasura,
                    "UPDATE sequent_backend.document SET is_public=NULL WHERE id=$1", &[&document_id]).await;
            }
            "request event" => {
                body["election_event_id"] = json!(event.election_event_id)
            }
            _ => unreachable!(),
        }
        let (status, response) = fetch(
            &services,
            &reader(&event, &[Permissions::TENANT_READ]),
            &body,
        )
        .await;
        assert_eq!(status, Status::Unauthorized, "{change}: {response}");
        assert_eq!(response["extensions"]["code"], "Unauthorized", "{change}");
    }
}

#[rocket::async_test]
async fn tenant_backup_download_keeps_tenant_permission_and_secret_policy_boundaries(
) {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let other = rows::event(&services.hasura).await;
    let (id, _) = tenant_backup(&services, &event).await;
    let body = json!({"document_id": id});
    for claims in [
        reader(&event, &[]),
        reader(&other, &[Permissions::TENANT_READ]),
    ] {
        let (status, response) = fetch(&services, &claims, &body).await;
        assert_eq!(status, Status::Unauthorized, "{response}");
        assert_eq!(response["extensions"]["code"], "Unauthorized");
    }
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.document SET annotations=$2 WHERE id=$1",
        &[
            &uuid::Uuid::parse_str(&id).unwrap(),
            &json!({"access": {"voter_secret_attributes": true}}),
        ],
    )
    .await;
    let (status, response) = fetch(
        &services,
        &reader(&event, &[Permissions::TENANT_READ]),
        &body,
    )
    .await;
    assert_eq!(status, Status::Unauthorized, "{response}");
    let (status, response) = fetch(
        &services,
        &reader(
            &event,
            &[
                Permissions::TENANT_READ,
                Permissions::VOTER_SECRET_ATTRIBUTE_READ,
            ],
        ),
        &body,
    )
    .await;
    assert_eq!(status, Status::Ok, "{response}");
}

#[rocket::async_test]
async fn tenant_backup_metadata_cannot_change_while_its_url_is_generated() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let (id, _) = tenant_backup(&services, &event).await;
    let mut first = services.hasura.get().await.unwrap();
    let transaction = first.transaction().await.unwrap();
    assert!(super::lock_completed_tenant_backup(
        &transaction,
        &event.tenant_id,
        &id
    )
    .await
    .unwrap());

    let mut second = services.hasura.get().await.unwrap();
    let competing = second.transaction().await.unwrap();
    competing
        .batch_execute("SET LOCAL lock_timeout = '100ms'")
        .await
        .unwrap();
    let error = competing.execute(
        "UPDATE sequent_backend.document SET name='../../other/object.zip' WHERE id=$1",
        &[&uuid::Uuid::parse_str(&id).unwrap()],
    ).await.unwrap_err();
    assert_eq!(
        error.code(),
        Some(&tokio_postgres::error::SqlState::LOCK_NOT_AVAILABLE)
    );
    competing.rollback().await.unwrap();
    transaction.commit().await.unwrap();
    // The row is writable again after URL generation's transaction ends.
    assert_eq!(second.execute(
        "UPDATE sequent_backend.document SET name='changed.zip' WHERE id=$1",
        &[&uuid::Uuid::parse_str(&id).unwrap()],
    ).await.unwrap(), 1);
}
