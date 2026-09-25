// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot publication routes on the migrated test database: gold and
//! lockdown checks, and the task row a publish attempt completes or fails.

use crate::adapters::memory::task_ledger::MemoryTaskLedger;
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use chrono::Utc;
use rocket::http::Status;
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

const USER_ID: &str = "publisher";

fn publisher(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, USER_ID).roles([Permissions::PUBLISH_WRITE])
}

fn gold_publisher(event: &Event) -> Claims {
    publisher(event)
        .acr(&Permissions::GOLD.to_string())
        .auth_time(Utc::now().timestamp())
}

fn failure(message: &str) -> (Status, Value) {
    (
        Status::InternalServerError,
        json!({"message": message, "extensions": {"code": "InternalServerError"}}),
    )
}

async fn generate<'c>(client: &'c Client, event: &Event) -> LocalResponse<'c> {
    post(
        client,
        "/generate-ballot-publication",
        &gold_publisher(event),
        &json!({"election_event_id": event.election_event_id}),
    )
    .await
}

#[rocket::async_test]
async fn a_locked_down_election_event_takes_no_new_ballot_publication() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event
        .present(&services.hasura, json!({"locked_down": "locked-down"}))
        .await;

    assert_eq!(
        json(generate(&client, &event).await).await,
        (
            Status::Forbidden,
            json!({"message": "Election event is locked down",
                "extensions": {"code": "Unauthorized"}})
        )
    );
}

#[rocket::async_test]
async fn ballot_publication_generation_hides_server_side_failures() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let unknown = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    let unreadable = rows::event(&services.hasura).await;
    unreadable
        .present(&services.hasura, json!({"locked_down": 7}))
        .await;

    for event in [&unknown, &unreadable] {
        assert_eq!(
            json(generate(&client, event).await).await,
            failure("Could not generate the ballot publication.")
        );
    }
}

async fn publish<'c>(
    client: &'c Client,
    event: &Event,
    ballot_publication_id: &str,
) -> LocalResponse<'c> {
    post(
        client,
        "/publish-ballot",
        &publisher(event),
        &json!({
            "election_event_id": event.election_event_id,
            "ballot_publication_id": ballot_publication_id,
        }),
    )
    .await
}

#[rocket::async_test]
async fn publishing_an_already_published_ballot_completes_its_task() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let publication =
        event.ballot_publication(&services.hasura, true, true).await;

    assert_eq!(
        json(publish(&client, &event, &publication).await).await,
        (Status::Ok, json!({ "ballot_publication_id": publication }))
    );
    let tasks = services.ledger.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_type, "PUBLISH_BALLOT");
    assert_eq!(tasks[0].execution_status, "SUCCESS");
}

#[rocket::async_test]
async fn a_publication_that_cannot_be_published_fails_its_task_with_the_reason()
{
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let ungenerated = event
        .ballot_publication(&services.hasura, false, false)
        .await;
    let unknown = uuid::Uuid::new_v4().to_string();

    for (publication, reason) in [
        (unknown, "Can't find ballot publication"),
        (
            ungenerated,
            "Ballot publication not generated yet, can't publish.",
        ),
    ] {
        let tasks_before = services.ledger.tasks().len();
        let response = publish(&client, &event, &publication).await;
        let task = services.ledger.tasks()[tasks_before].clone();
        assert_eq!(
            json(response).await,
            failure(&format!("Publish task {} failed: {reason}", task.id))
        );
        assert_eq!(task.execution_status, "FAILED");
        assert_eq!(task.logs, Some(json!([format!("Error: {reason}")])));
    }
}

#[rocket::async_test]
async fn a_failed_publish_whose_task_cannot_be_failed_says_so() {
    let services = Services::on_test_database()
        .await
        .with_ledger(MemoryTaskLedger::refusing_updates());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let unknown = uuid::Uuid::new_v4().to_string();

    let (status, body) = json(publish(&client, &event, &unknown).await).await;
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(
        body["message"],
        "Publish task task-1 failed: Can't find ballot publication. The task failure could not be recorded: tasks_execution is unavailable"
    );
}

#[rocket::async_test]
async fn a_publish_whose_task_cannot_be_completed_is_reported_as_published() {
    let services = Services::on_test_database()
        .await
        .with_ledger(MemoryTaskLedger::refusing_updates());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let publication =
        event.ballot_publication(&services.hasura, true, true).await;

    assert_eq!(
        json(publish(&client, &event, &publication).await).await,
        failure("Ballot was published, but task task-1 could not be marked complete: tasks_execution is unavailable")
    );
}

#[rocket::async_test]
async fn a_publish_without_a_task_row_is_not_attempted() {
    let services = Services::on_test_database()
        .await
        .with_ledger(MemoryTaskLedger::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let publication = event
        .ballot_publication(&services.hasura, true, false)
        .await;

    let (status, body) =
        json(publish(&client, &event, &publication).await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .starts_with("tasks_execution is unavailable"),
        "{body}"
    );
    let published = rows::query(
        &services.hasura,
        "SELECT published_at FROM sequent_backend.ballot_publication
         WHERE id = $1 AND published_at IS NOT NULL",
        &[&uuid::Uuid::parse_str(&publication).unwrap()],
    )
    .await;
    assert!(published.is_empty());
}

#[rocket::async_test]
async fn the_changes_of_a_first_publication_have_nothing_to_compare_with() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let publication = event
        .ballot_publication(&services.hasura, true, false)
        .await;

    let (status, body) = json(
        post(
            &client,
            "/get-ballot-publication-changes",
            &Claims::new(&event.tenant_id, USER_ID)
                .roles([Permissions::PUBLISH_READ]),
            &json!({
                "election_event_id": event.election_event_id,
                "ballot_publication_id": publication,
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        body["current"]["ballot_publication_id"],
        publication.as_str()
    );
    assert_eq!(body["previous"], Value::Null);
}

#[rocket::async_test]
async fn the_changes_of_an_unknown_publication_are_a_server_error() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let response = post(
        &client,
        "/get-ballot-publication-changes",
        &Claims::new(&event.tenant_id, USER_ID)
            .roles([Permissions::PUBLISH_READ]),
        &json!({
            "election_event_id": event.election_event_id,
            "ballot_publication_id": uuid::Uuid::new_v4().to_string(),
        }),
    )
    .await;
    let (status, message) = text(response).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.contains("Can't find ballot publication"),
        "{message}"
    );
}

#[rocket::async_test]
async fn ballot_publication_routes_answer_500_when_the_database_is_unreachable()
{
    let services = Services::without_database();
    let client = services.client().await;
    let event = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    assert_eq!(
        json(generate(&client, &event).await).await,
        failure("Could not generate the ballot publication.")
    );
    let (status, body) =
        json(publish(&client, &event, "publication").await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("Connection refused"),
        "{body}"
    );
    assert!(services.ledger.tasks().is_empty());
    let response = post(
        &client,
        "/get-ballot-publication-changes",
        &Claims::new(&event.tenant_id, USER_ID)
            .roles([Permissions::PUBLISH_READ]),
        &json!({"election_event_id": event.election_event_id,
            "ballot_publication_id": "publication"}),
    )
    .await;
    let (status, message) = text(response).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(message.contains("Connection refused"), "{message}");
}
