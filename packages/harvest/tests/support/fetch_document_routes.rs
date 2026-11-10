// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! fetchDocument on the migrated test database: the document's access
//! policy decides who may read it, and the caller gets the storage's URL or
//! a JSON error with a code.

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use sequent_core::ballot::DecodedBallotsInclusionPolicy;
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

/// A tenant's document needs the download permission, whichever task wrote
/// it: the tenant permissions give none.
#[rocket::async_test]
async fn a_tenant_document_needs_the_download_permission() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let id = document(&services, &event, None).await;
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
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.tasks_execution
            (id, tenant_id, election_event_id, name, type, execution_status,
             annotations, executed_by_user)
         VALUES ($1, $2, NULL, 'Export Tenant Config', 'EXPORT_TENANT_CONFIG',
             'SUCCESS', $3, 'synthetic-tenant-admin')",
        &[
            &uuid::Uuid::new_v4(),
            &tenant_id,
            &json!({"document_id": id}),
        ],
    )
    .await;
    let body = json!({"document_id": id});

    for permissions in [
        &[Permissions::TENANT_READ][..],
        &[Permissions::TENANT_READ, Permissions::TENANT_WRITE][..],
    ] {
        let (status, response) =
            fetch(&services, &reader(&event, permissions), &body).await;
        assert_eq!(status, Status::Unauthorized, "{response}");
        assert_eq!(response["extensions"]["code"], "Unauthorized");
    }
    let (status, response) = fetch(
        &services,
        &reader(&event, &[Permissions::DOCUMENT_DOWNLOAD]),
        &body,
    )
    .await;
    assert_eq!(status, Status::Ok, "{response}");
    assert_eq!(
        response,
        json!({"url": format!("https://documents.test/{id}")})
    );
}

/// The documents of a tally of `event` whose session has `policy`: the
/// archives of the tally's folder, with and without its PDFs, and the
/// results database and its spreadsheet.
async fn tally_documents(
    services: &Services,
    event: &Event,
    policy: DecodedBallotsInclusionPolicy,
) -> (Vec<String>, Vec<String>) {
    let election = event.election(&services.hasura).await;
    let session = event
        .tally_session(&services.hasura, &[election], "SUCCESS")
        .await;
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.tally_session SET configuration = $2
         WHERE id = $1::text::uuid",
        &[
            &session,
            &json!({"decoded_ballots_inclusion_policy": policy}),
        ],
    )
    .await;
    let mut ids = Vec::new();
    for _ in 0..5 {
        ids.push(document(services, event, None).await);
    }
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.results_event
             (tenant_id, election_event_id, documents)
         VALUES ($1::text::uuid, $2::text::uuid, $3)",
        &[
            &event.tenant_id,
            &event.election_event_id,
            &json!({
                "tar_gz": ids[0],
                "tar_gz_original": ids[1],
                "tar_gz_pdfs": ids[2],
            }),
        ],
    )
    .await;
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.tally_session_execution SET documents = $2
         WHERE tally_session_id = $1::text::uuid",
        &[&session, &json!({"sqlite": ids[3], "xlsx": ids[4]})],
    )
    .await;
    let results = ids.split_off(3);
    (ids, results)
}

/// The refusal of a document that waits for post-processing, in `phase`.
fn waits_for_post_processing(phase: &str) -> Value {
    json!({
        "message": format!("This operation requires electoral phase PostProcessing; current phase is {phase}"),
        "extensions": {"code": "InternalServerError"},
    })
}

/// Gives the tenant a signing setting that allows unsigned files, past the
/// settings guard, which accepts only the required policy.
async fn allow_unsigned_imports(services: &Services, tenant_id: &str) {
    let mut client = services.hasura.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL session_replication_role = replica")
        .await
        .unwrap();
    tx.execute(
        "UPDATE sequent_backend.tenant SET settings = $2
         WHERE id = $1::text::uuid",
        &[
            &tenant_id,
            &json!({"configuration_signing": {"policy": "optional"}}),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// Where only signed packages import, the tally's documents that hold
/// decoded ballots are served from post-processing on; the event's other
/// documents are served in any phase.
#[rocket::async_test]
async fn tally_documents_with_decoded_ballots_wait_for_post_processing() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let (archives, results) = tally_documents(
        &services,
        &event,
        DecodedBallotsInclusionPolicy::INCLUDED,
    )
    .await;
    let other = document(&services, &event, None).await;
    let claims = reader(&event, &[Permissions::DOCUMENT_DOWNLOAD]);

    for (phase, name) in
        [("preparation", "Preparation"), ("evaluation", "Evaluation")]
    {
        event.enter_phase(&services.hasura, phase).await;
        for id in archives.iter().chain(&results) {
            let (status, body) =
                fetch(&services, &claims, &request(&event, id)).await;
            assert_eq!(status, Status::Conflict, "{body}");
            assert_eq!(body, waits_for_post_processing(name));
        }
        let (status, body) =
            fetch(&services, &claims, &request(&event, &other)).await;
        assert_eq!(status, Status::Ok, "{body}");
    }

    event.enter_phase(&services.hasura, "post_processing").await;
    for id in archives.iter().chain(&results).chain([&other]) {
        let (status, body) =
            fetch(&services, &claims, &request(&event, id)).await;
        assert_eq!(status, Status::Ok, "{body}");
        assert_eq!(body["url"], format!("https://documents.test/{id}"));
    }
}

/// A results database whose tally session leaves the decoded ballots out,
/// and its spreadsheet, are served before post-processing; the archives of
/// the tally's folder are not.
#[rocket::async_test]
async fn results_without_decoded_ballots_are_served_before_post_processing() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let (archives, results) = tally_documents(
        &services,
        &event,
        DecodedBallotsInclusionPolicy::NOT_INCLUDED,
    )
    .await;
    let claims = reader(&event, &[Permissions::DOCUMENT_DOWNLOAD]);
    event.enter_phase(&services.hasura, "evaluation").await;

    for id in &archives {
        let (status, body) =
            fetch(&services, &claims, &request(&event, id)).await;
        assert_eq!(status, Status::Conflict, "{body}");
        assert_eq!(body, waits_for_post_processing("Evaluation"));
    }
    for id in &results {
        let (status, body) =
            fetch(&services, &claims, &request(&event, id)).await;
        assert_eq!(status, Status::Ok, "{body}");
        assert_eq!(body["url"], format!("https://documents.test/{id}"));
    }
}

/// Where unsigned packages import, the tally's documents are served in any
/// phase.
#[rocket::async_test]
async fn tally_documents_are_served_in_evaluation_where_unsigned_packages_import(
) {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let (archives, results) = tally_documents(
        &services,
        &event,
        DecodedBallotsInclusionPolicy::INCLUDED,
    )
    .await;
    allow_unsigned_imports(&services, &event.tenant_id).await;
    event.enter_phase(&services.hasura, "evaluation").await;

    for id in archives.iter().chain(&results) {
        let (status, body) = fetch(
            &services,
            &reader(&event, &[Permissions::DOCUMENT_DOWNLOAD]),
            &request(&event, id),
        )
        .await;
        assert_eq!(status, Status::Ok, "{body}");
        assert_eq!(body["url"], format!("https://documents.test/{id}"));
    }
}

#[rocket::async_test]
async fn transcript_export_rejects_callers_without_ceremony_permission_before_io(
) {
    let services = Services::without_database();
    let claims = Claims::new(&uuid::Uuid::new_v4().to_string(), "not-board")
        .roles([Permissions::DOCUMENT_DOWNLOAD]);
    let client = services.client().await;
    let response = post(
        &client,
        "/export-verification-transcript",
        &claims,
        &json!({"election_event_id": uuid::Uuid::new_v4().to_string(),
                "protocol_board_id": uuid::Uuid::new_v4().to_string()}),
    )
    .await;
    assert_eq!(response.status(), Status::Unauthorized);
}
