// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tally ceremony routes on the migrated test database: their answers,
//! the executions a recount records and the tasks it sends.

use crate::adapters::memory::task_queue::MemoryTaskQueue;
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

const USER_ID: &str = "tally-admin";
const HIDDEN: &str = "Could not complete the tally operation.";

fn admin(event: &Event, permission: Permissions) -> Claims {
    Claims::new(&event.tenant_id, USER_ID).roles([permission])
}

async fn recount<'c>(
    client: &'c Client,
    event: &Event,
    tally_session_id: &str,
) -> LocalResponse<'c> {
    post(
        client,
        "/recount-tally-session",
        &admin(event, Permissions::TALLY_RECOUNT_EXECUTE),
        &json!({
            "election_event_id": event.election_event_id,
            "tally_session_id": tally_session_id,
        }),
    )
    .await
}

/// An event with one election whose tally finished with `status`.
async fn tallied(services: &Services, status: &str) -> (Event, String, String) {
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    let tally_session_id = event
        .tally_session(&services.hasura, &[election_id.clone()], status)
        .await;
    (event, election_id, tally_session_id)
}

#[rocket::async_test]
async fn recounting_a_completed_tally_records_an_execution_and_sends_the_tally_task(
) {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, election_id, tally_session_id) =
        tallied(&services, "SUCCESS").await;

    let (status, body) =
        json(recount(&client, &event, &tally_session_id).await).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(body, json!({ "tally_session_id": tally_session_id }));
    assert_eq!(
        event
            .tally_session_executions(&services.hasura, &tally_session_id)
            .await,
        2
    );
    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "execute_tally_session");
    assert_eq!(
        sent[0].kwargs,
        json!({
            "tenant_id": event.tenant_id,
            "election_event_id": event.election_event_id,
            "tally_session_id": tally_session_id,
            "tally_type": "ELECTORAL_RESULTS",
            "election_ids": [election_id],
            "force_new_results_id": true,
        })
    );
}

#[rocket::async_test]
async fn a_recount_whose_task_cannot_be_sent_is_accepted_for_later_delivery() {
    let services = Services::on_test_database()
        .await
        .with_tasks(MemoryTaskQueue::refusing());
    let client = services.client().await;
    let (event, _, tally_session_id) = tallied(&services, "SUCCESS").await;

    let (status, _) =
        json(recount(&client, &event, &tally_session_id).await).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(
        event
            .tally_session_executions(&services.hasura, &tally_session_id)
            .await,
        2
    );
}

#[rocket::async_test]
async fn only_a_completed_tally_can_be_recounted() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, _, tally_session_id) = tallied(&services, "IN_PROGRESS").await;

    assert_eq!(
        text(recount(&client, &event, &tally_session_id).await).await,
        (
            Status::BadRequest,
            "Only completed tally sessions can be recounted".into()
        )
    );
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_completed_tally_without_execution_history_cannot_be_recounted() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, _, tally_session_id) = tallied(&services, "SUCCESS").await;
    rows::execute(
        &services.hasura,
        "DELETE FROM sequent_backend.tally_session_execution
         WHERE tally_session_id = $1",
        &[&uuid::Uuid::parse_str(&tally_session_id).unwrap()],
    )
    .await;

    assert_eq!(
        text(recount(&client, &event, &tally_session_id).await).await,
        (
            Status::Conflict,
            "Only a completed tally session with execution history can be recounted".into()
        )
    );
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn recounting_an_unknown_tally_session_is_not_found() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let missing = uuid::Uuid::new_v4().to_string();

    assert_eq!(
        text(recount(&client, &event, &missing).await).await,
        (
            Status::NotFound,
            format!("Could not find tally session by id {missing}")
        )
    );
}

async fn create<'c>(
    client: &'c Client,
    event: &Event,
    election_ids: &[&str],
    tally_type: &str,
) -> LocalResponse<'c> {
    post(
        client,
        "/create-tally-ceremony",
        &admin(event, Permissions::ADMIN_CEREMONY),
        &json!({
            "election_event_id": event.election_event_id,
            "election_ids": election_ids,
            "tally_type": tally_type,
        }),
    )
    .await
}

fn tally_validation(message: &str) -> (Status, Value) {
    (
        Status::BadRequest,
        json!({"message": message, "extensions": {"code": "TallyValidation"}}),
    )
}

#[rocket::async_test]
async fn a_tally_ceremony_of_an_unknown_type_is_a_validation_error() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;

    assert_eq!(
        json(create(&client, &event, &[&election_id], "RECOUNT_ALL").await)
            .await,
        tally_validation("Invalid tally type")
    );
}

#[rocket::async_test]
async fn a_tally_ceremony_needs_at_least_one_election() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    assert_eq!(
        json(create(&client, &event, &[], "ELECTORAL_RESULTS").await).await,
        tally_validation("Select at least one election to tally.")
    );
    let sessions = rows::query(
        &services.hasura,
        "SELECT id FROM sequent_backend.tally_session
         WHERE election_event_id = $1",
        &[&uuid::Uuid::parse_str(&event.election_event_id).unwrap()],
    )
    .await;
    assert!(sessions.is_empty());
}

#[rocket::async_test]
async fn updating_an_unknown_tally_session_hides_the_failed_lookup() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let response = post(
        &client,
        "/update-tally-ceremony",
        &admin(&event, Permissions::ADMIN_CEREMONY),
        &json!({
            "election_event_id": event.election_event_id,
            "tally_session_id": uuid::Uuid::new_v4().to_string(),
            "status": "IN_PROGRESS",
        }),
    )
    .await;
    assert_eq!(
        json(response).await,
        (
            Status::InternalServerError,
            json!({"message": HIDDEN, "extensions": {"code": "InternalServerError"}})
        )
    );
}

#[rocket::async_test]
async fn a_tally_status_change_the_ceremony_does_not_allow_is_a_validation_error(
) {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, _, tally_session_id) = tallied(&services, "SUCCESS").await;

    let response = post(
        &client,
        "/update-tally-ceremony",
        &admin(&event, Permissions::ADMIN_CEREMONY),
        &json!({
            "election_event_id": event.election_event_id,
            "tally_session_id": tally_session_id,
            "status": "IN_PROGRESS",
        }),
    )
    .await;
    assert_eq!(
        json(response).await,
        tally_validation(
            "Cannot change tally status from SUCCESS to IN_PROGRESS."
        )
    );
}

#[rocket::async_test]
async fn a_tally_resolution_batch_needs_at_least_one_resolution_before_any_database_work(
) {
    // No database: the empty batch is refused first.
    let services = Services::without_database();
    let client = services.client().await;
    let event = rows::Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };

    let response = post(
        &client,
        "/submit-tally-resolution",
        &admin(&event, Permissions::TALLY_RESOLUTION_SUBMIT),
        &json!({
            "election_event_id": event.election_event_id,
            "tally_session_id": uuid::Uuid::new_v4().to_string(),
            "resolutions": [],
        }),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::BadRequest,
            "At least one resolution required".into()
        )
    );
}

#[rocket::async_test]
async fn resolutions_for_a_tally_that_is_not_paused_fail() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, _, tally_session_id) = tallied(&services, "SUCCESS").await;

    let response = post(
        &client,
        "/submit-tally-resolution",
        &admin(&event, Permissions::TALLY_RESOLUTION_SUBMIT),
        &json!({
            "election_event_id": event.election_event_id,
            "tally_session_id": tally_session_id,
            "resolutions": [{"contest_id": "contest", "selected_candidate_id": "candidate"}],
        }),
    )
    .await;
    let (status, _) = text(response).await;
    assert_eq!(status, Status::InternalServerError);
}

#[rocket::async_test]
async fn restoring_a_key_for_an_unknown_tally_session_is_a_bad_request() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let response = post(
        &client,
        "/restore-private-key",
        &admin(&event, Permissions::TRUSTEE_CEREMONY),
        &json!({
            "election_event_id": event.election_event_id,
            "tally_session_id": uuid::Uuid::new_v4().to_string(),
            "private_key_base64": "not-a-key",
        }),
    )
    .await;
    let (status, _) = text(response).await;
    assert_eq!(status, Status::BadRequest);
}

#[rocket::async_test]
async fn tally_ceremony_routes_answer_500_when_the_database_is_unreachable() {
    let services = Services::without_database();
    let client = services.client().await;
    let event = rows::Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    let session = uuid::Uuid::new_v4().to_string();
    // The ceremony routes answer the tally error contract, which hides
    // server-side detail.
    for (path, permission, body) in [
        (
            "/create-tally-ceremony",
            Permissions::ADMIN_CEREMONY,
            json!({"election_event_id": event.election_event_id,
                "election_ids": [], "tally_type": "ELECTORAL_RESULTS"}),
        ),
        (
            "/update-tally-ceremony",
            Permissions::ADMIN_CEREMONY,
            json!({"election_event_id": event.election_event_id,
                "tally_session_id": session, "status": "IN_PROGRESS"}),
        ),
    ] {
        let response =
            post(&client, path, &admin(&event, permission), &body).await;
        assert_eq!(
            json(response).await,
            (
                Status::InternalServerError,
                json!({"message": HIDDEN, "extensions": {"code": "InternalServerError"}})
            ),
            "{path}"
        );
    }
    for (path, permission, body) in [
        (
            "/recount-tally-session",
            Permissions::TALLY_RECOUNT_EXECUTE,
            json!({"election_event_id": event.election_event_id,
                "tally_session_id": session}),
        ),
        (
            "/restore-private-key",
            Permissions::TRUSTEE_CEREMONY,
            json!({"election_event_id": event.election_event_id,
                "tally_session_id": session, "private_key_base64": "key"}),
        ),
        (
            "/submit-tally-resolution",
            Permissions::TALLY_RESOLUTION_SUBMIT,
            json!({"election_event_id": event.election_event_id,
                "tally_session_id": session, "resolutions": [
                    {"contest_id": "contest", "selected_candidate_id": "candidate"}]}),
        ),
    ] {
        let response =
            post(&client, path, &admin(&event, permission), &body).await;
        let (status, message) = text(response).await;
        assert_eq!(status, Status::InternalServerError, "{path}");
        assert!(
            message.starts_with("Error getting hasura db pool: "),
            "{path}: {message}"
        );
    }
}
