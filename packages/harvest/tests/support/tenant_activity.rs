// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Requests that belong to no election event are queued for the tenant's log
//! with who made them and how they ended, whether or not they were allowed.

use super::*;
use crate::adapters::memory::task_queue::{MemoryTaskQueue, SentTask};
use crate::route_services::{bearer, post, Services};
use crate::test_claims::Claims;
use rocket::http::ContentType;
use rocket::local::asynchronous::Client;
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use std::collections::HashSet;
use strum::IntoEnumIterator;

const TENANT_ID: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
const EVENT_ID: &str = "fdd21db2-dd68-4974-90eb-7f2750b2b5df";
const ADMIN_ID: &str = "admin-id";
const ADMIN: &str = "ada.admin";

fn admin() -> Claims {
    Claims::new(TENANT_ID, ADMIN_ID).username(ADMIN)
}

/// Answers with the status asked for, after recording a deletion.
#[post("/probe/<status>")]
fn probe(
    status: u16,
    claims: JwtClaims,
    activity: TenantActivityRecorder<'_>,
) -> Status {
    activity.record(
        &claims,
        TenantAction::CreateRole,
        json!({"role": "overwritten"}),
    );
    activity.record(
        &claims,
        TenantAction::DeleteElectionEvent,
        json!({"election_event_id": EVENT_ID}),
    );
    Status::new(status)
}

/// Records nothing.
#[post("/quiet")]
fn quiet(_claims: JwtClaims, _activity: TenantActivityRecorder<'_>) -> Status {
    Status::Ok
}

async fn probe_client(services: &Services) -> Client {
    let rocket = rocket::build()
        .attach(fairing())
        .manage(services.harvest_services())
        .manage(crate::test_claims::signing::issuers())
        .mount("/", routes![probe, quiet]);
    Client::untracked(rocket).await.expect("a rocket")
}

/// The one event a request queued: its task's input and its activity.
fn queued(tasks: &MemoryTaskQueue) -> (Value, Value) {
    let sent: Vec<SentTask> = tasks.sent();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].name, "enqueue_electoral_log_event");
    let input = sent[0].kwargs["input"].clone();
    let activity = serde_json::from_str(
        input["body"]
            .as_str()
            .expect("the activity as a JSON string"),
    )
    .expect("an activity");
    (input, activity)
}

#[test]
fn every_action_has_its_own_name_and_sentences() {
    let mut names = HashSet::new();
    let mut sentences = HashSet::new();
    for action in TenantAction::iter() {
        let name = action.to_string();
        assert!(
            name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "{name}"
        );
        assert_eq!(name.parse::<TenantAction>().unwrap(), action);
        assert!(names.insert(name));
        for outcome in [
            TenantActivityOutcome::Succeeded,
            TenantActivityOutcome::Denied,
            TenantActivityOutcome::Failed,
        ] {
            let sentence = action.description(outcome);
            assert!(sentence.starts_with("Request to "), "{sentence}");
            assert!(sentence.ends_with('.'), "{sentence}");
            assert!(sentences.insert(sentence));
        }
    }
    assert_eq!(
        TenantAction::DeleteElectionEvent.to_string(),
        "delete-election-event"
    );
    assert_eq!(
        TenantAction::DeleteElectionEvent
            .description(TenantActivityOutcome::Denied),
        "Request to delete an election event denied."
    );
    assert!("delete-everything".parse::<TenantAction>().is_err());
}

#[test]
fn the_status_says_how_a_request_ended() {
    for (code, outcome) in [
        (200, TenantActivityOutcome::Succeeded),
        (204, TenantActivityOutcome::Succeeded),
        (401, TenantActivityOutcome::Denied),
        (403, TenantActivityOutcome::Denied),
        (400, TenantActivityOutcome::Failed),
        (404, TenantActivityOutcome::Failed),
        (500, TenantActivityOutcome::Failed),
        (302, TenantActivityOutcome::Failed),
    ] {
        assert_eq!(outcome_of(Status::new(code)), outcome, "{code}");
    }
}

#[rocket::async_test]
async fn an_answered_request_is_queued_with_its_actor_and_outcome() {
    let services = Services::without_database();
    let client = probe_client(&services).await;
    let before = chrono::Utc::now().timestamp_millis();

    let response = client
        .post("/probe/200")
        .header(bearer(&admin()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let (input, activity) = queued(&services.tasks);
    assert_eq!(input["tenant_id"], TENANT_ID);
    // No election event: Windmill puts it in the tenant's log.
    assert_eq!(input["election_event_id"], "");
    assert_eq!(input["message_type"], "tenant_activity");
    assert_eq!(input["user_id"], ADMIN_ID);
    assert_eq!(input["username"], ADMIN);
    assert!(input["event_time_ms"].as_i64().unwrap() >= before);
    assert_eq!(
        uuid::Uuid::parse_str(input["delivery_id"].as_str().unwrap())
            .unwrap()
            .get_version_num(),
        4
    );
    // The last record of the request is the one kept.
    assert_eq!(
        activity,
        json!({
            "action": "delete-election-event",
            "description": "Request to delete an election event accepted.",
            "outcome": "succeeded",
            "status": 200,
            "details": {"election_event_id": EVENT_ID},
        })
    );
}

#[rocket::async_test]
async fn refused_and_failed_requests_are_queued_as_such() {
    for (path, outcome, sentence, status) in [
        (
            "/probe/403",
            "denied",
            "Request to delete an election event denied.",
            403,
        ),
        (
            "/probe/500",
            "failed",
            "Request to delete an election event failed.",
            500,
        ),
    ] {
        let services = Services::without_database();
        let client = probe_client(&services).await;
        client.post(path).header(bearer(&admin())).dispatch().await;
        let (_, activity) = queued(&services.tasks);
        assert_eq!(activity["outcome"], outcome);
        assert_eq!(activity["description"], sentence);
        assert_eq!(activity["status"], status);
    }
}

#[rocket::async_test]
async fn nothing_is_queued_without_a_record_or_a_caller() {
    let services = Services::without_database();
    let client = probe_client(&services).await;

    let response = client
        .post("/quiet")
        .header(bearer(&admin()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    // Without a verified caller the handler never runs.
    let response = client.post("/probe/200").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_queue_that_cannot_be_reached_does_not_change_the_answer() {
    for tasks in [MemoryTaskQueue::unreachable(), MemoryTaskQueue::refusing()] {
        let services = Services::without_database().with_tasks(tasks);
        let client = probe_client(&services).await;
        let response = client
            .post("/probe/200")
            .header(bearer(&admin()))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(services.tasks.sent().is_empty());
    }
}

#[rocket::async_test]
async fn a_request_without_managed_services_is_answered_all_the_same() {
    let rocket = rocket::build()
        .attach(fairing())
        .manage(crate::test_claims::signing::issuers())
        .mount("/", routes![probe]);
    let client = Client::untracked(rocket).await.expect("a rocket");
    let response = client
        .post("/probe/200")
        .header(bearer(&admin()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

/// The real routes: a role change is tenant-level whoever asks, and it is
/// recorded before the caller is authorized.
#[rocket::async_test]
async fn a_refused_role_deletion_is_recorded() {
    let services = Services::without_database();
    let client = services.client().await;

    let response = post(
        &client,
        "/delete-role",
        &admin(),
        &json!({"tenant_id": TENANT_ID, "role_id": "group-1"}),
    )
    .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let (input, activity) = queued(&services.tasks);
    assert_eq!(input["username"], ADMIN);
    assert_eq!(activity["action"], "delete-role");
    assert_eq!(activity["outcome"], "denied");
    assert_eq!(activity["status"], 401);
    assert_eq!(
        activity["details"],
        json!({"tenant_id": TENANT_ID, "role_id": "group-1"})
    );
}

/// A voter of an election event is not a staff account: deleting one belongs
/// to the event's log, so the tenant's gets nothing.
#[rocket::async_test]
async fn a_voters_deletion_is_not_tenant_activity() {
    let services = Services::without_database();
    let client = services.client().await;

    let voter = post(
        &client,
        "/delete-user",
        &admin(),
        &json!({
            "tenant_id": TENANT_ID,
            "election_event_id": EVENT_ID,
            "user_id": "voter-1",
        }),
    )
    .await;
    assert_eq!(voter.status(), Status::Unauthorized);
    assert!(services.tasks.sent().is_empty());

    let staff = post(
        &client,
        "/delete-user",
        &admin(),
        &json!({"tenant_id": TENANT_ID, "user_id": "staff-1"}),
    )
    .await;
    assert_eq!(staff.status(), Status::Unauthorized);
    let (_, activity) = queued(&services.tasks);
    assert_eq!(activity["action"], "delete-user");
    assert_eq!(
        activity["details"],
        json!({"tenant_id": TENANT_ID, "user_id": "staff-1"})
    );
}

/// Reading the tenant's log (the electoral log of no election event) needs its
/// own permission, on the caller's tenant.
#[rocket::async_test]
async fn the_tenant_log_needs_its_own_permission() {
    let services = Services::without_database();
    let client = services.client().await;
    let body = json!({"tenant_id": TENANT_ID});

    for claims in [
        admin(),
        admin().roles([Permissions::LOGS_READ, Permissions::TENANT_READ]),
        Claims::new("11111111-2222-4333-8444-555555555555", ADMIN_ID)
            .roles([Permissions::TENANT_LOGS_READ]),
    ] {
        let response = client
            .post("/electoral-log")
            .header(ContentType::JSON)
            .header(bearer(&claims))
            .body(body.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }
    // Reading the log is not itself recorded.
    assert!(services.tasks.sent().is_empty());
}
