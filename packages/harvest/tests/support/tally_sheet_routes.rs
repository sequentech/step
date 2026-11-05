// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tally sheet routes on the migrated test database, from the request
//! to the rows they commit and the recount tasks they send.

use crate::adapters::memory::task_queue::MemoryTaskQueue;
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use windmill::services::tally_sheet_import::hash::hash_bytes;

const USER_ID: &str = "tally-sheet-admin";
const PLURALITY: &str = "plurality-at-large";

fn admin(event: &Event, permissions: &[Permissions]) -> Claims {
    Claims::new(&event.tenant_id, USER_ID).roles(permissions)
}

/// An election with one plurality contest in one area.
struct Ballot {
    event: Event,
    election_id: String,
    contest_id: String,
    area_id: String,
}

async fn ballot(services: &Services) -> Ballot {
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    let contest_id = event
        .contest(&services.hasura, &election_id, PLURALITY)
        .await;
    let area_id = event.area(&services.hasura, "Precinct 1").await;
    Ballot {
        event,
        election_id,
        contest_id,
        area_id,
    }
}

fn sheet(ballot: &Ballot, content: Value) -> Value {
    json!({
        "election_event_id": ballot.event.election_event_id,
        "channel": "PAPER",
        "contest_id": ballot.contest_id,
        "area_id": ballot.area_id,
        "content": content,
    })
}

/// Ten valid ballots for one candidate.
fn consistent_content(ballot: &Ballot) -> Value {
    json!({
        "area_id": ballot.area_id, "contest_id": ballot.contest_id,
        "total_votes": 10, "total_valid_votes": 10, "total_blank_votes": 0,
        "invalid_votes": {"total_invalid": 0, "implicit_invalid": 0, "explicit_invalid": 0},
        "census": 20,
        "candidate_results": {"candidate": {"candidate_id": "candidate", "total_votes": 10}}
    })
}

async fn create_sheet(
    client: &Client,
    ballot: &Ballot,
    content: Value,
) -> (Status, Value) {
    let claims = admin(&ballot.event, &[Permissions::TALLY_SHEET_CREATE]);
    json(
        post(
            client,
            "/create-new-tally-sheet",
            &claims,
            &sheet(ballot, content),
        )
        .await,
    )
    .await
}

async fn review<'c>(
    client: &'c Client,
    event: &Event,
    tally_sheet_id: &str,
    new_status: &str,
) -> rocket::local::asynchronous::LocalResponse<'c> {
    post(
        client,
        "/review-tally-sheet",
        &admin(event, &[Permissions::TALLY_SHEET_REVIEW]),
        &json!({
            "election_event_id": event.election_event_id,
            "tally_sheet_id": tally_sheet_id,
            "new_status": new_status,
        }),
    )
    .await
}

#[rocket::async_test]
async fn each_new_tally_sheet_for_a_ballot_box_is_its_next_pending_version() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;

    let (status, first) =
        create_sheet(&client, &ballot, consistent_content(&ballot)).await;
    assert_eq!(status, Status::Ok, "{first}");
    assert_eq!(first["status"], "PENDING");
    assert_eq!(first["version"], 1);
    assert_eq!(first["created_by_user_id"], USER_ID);
    assert_eq!(first["election_id"], ballot.election_id.as_str());
    assert_eq!(first["channel"], "PAPER");
    let (_, second) =
        create_sheet(&client, &ballot, consistent_content(&ballot)).await;
    assert_eq!(second["version"], 2);

    // Both versions were committed.
    let stored = rows::query(
        &services.hasura,
        "SELECT version FROM sequent_backend.tally_sheet
         WHERE contest_id = $1 ORDER BY version",
        &[&uuid::Uuid::parse_str(&ballot.contest_id).unwrap()],
    )
    .await;
    let versions: Vec<i32> = stored.iter().map(|row| row.get(0)).collect();
    assert_eq!(versions, vec![1, 2]);
}

#[rocket::async_test]
async fn a_tally_sheet_for_an_unknown_contest_is_not_found() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let mut ballot = ballot(&services).await;
    ballot.contest_id = uuid::Uuid::new_v4().to_string();

    let response = post(
        &client,
        "/create-new-tally-sheet",
        &admin(&ballot.event, &[Permissions::TALLY_SHEET_CREATE]),
        &sheet(&ballot, consistent_content(&ballot)),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::NotFound,
            format!("Contest {} not found ", ballot.contest_id)
        )
    );
}

#[rocket::async_test]
async fn acclaimed_contests_take_no_tally_sheets() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.contest SET is_acclaimed = true WHERE id = $1",
        &[&uuid::Uuid::parse_str(&ballot.contest_id).unwrap()],
    )
    .await;

    let response = post(
        &client,
        "/create-new-tally-sheet",
        &admin(&ballot.event, &[Permissions::TALLY_SHEET_CREATE]),
        &sheet(&ballot, consistent_content(&ballot)),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::BadRequest,
            "Tally sheets cannot be created for acclaimed contests".into()
        )
    );
}

#[rocket::async_test]
async fn inconsistent_counts_are_refused_with_their_validation_messages() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    let mut content = consistent_content(&ballot);
    content["invalid_votes"] = json!({"total_invalid": 3, "implicit_invalid": 1, "explicit_invalid": 1});

    let response = post(
        &client,
        "/create-new-tally-sheet",
        &admin(&ballot.event, &[Permissions::TALLY_SHEET_CREATE]),
        &sheet(&ballot, content),
    )
    .await;
    let (status, message) = text(response).await;
    assert_eq!(status, Status::BadRequest);
    assert!(
        message.starts_with(
            "Invalid tally sheet content: invalid_total_invalid: total_invalid (3) must equal implicit_invalid (1) + explicit_invalid (1)"
        ),
        "{message}"
    );
    let stored = rows::query(
        &services.hasura,
        "SELECT id FROM sequent_backend.tally_sheet WHERE contest_id = $1",
        &[&uuid::Uuid::parse_str(&ballot.contest_id).unwrap()],
    )
    .await;
    assert!(stored.is_empty(), "a refused sheet is not stored");
}

#[rocket::async_test]
async fn an_unrecognised_counting_algorithm_is_reported_instead_of_validating_counts(
) {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    let contest_id = event
        .contest(&services.hasura, &election_id, "not-an-algorithm")
        .await;
    let area_id = event.area(&services.hasura, "Precinct 1").await;
    let ballot = Ballot {
        event,
        election_id,
        contest_id,
        area_id,
    };

    let response = post(
        &client,
        "/create-new-tally-sheet",
        &admin(&ballot.event, &[Permissions::TALLY_SHEET_CREATE]),
        &sheet(&ballot, consistent_content(&ballot)),
    )
    .await;
    assert_eq!(
        text(response).await,
        (
            Status::BadRequest,
            "Invalid tally sheet content: unknown_counting_algorithm: counting_algorithm (not-an-algorithm) is not a counting algorithm this version recognises".into()
        )
    );
}

#[rocket::async_test]
async fn tally_sheet_routes_answer_500_when_the_database_is_unreachable() {
    let services = Services::without_database();
    let client = services.client().await;
    let event = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    for (path, permission, body) in [
        (
            "/create-new-tally-sheet",
            Permissions::TALLY_SHEET_CREATE,
            json!({"election_event_id": event.election_event_id, "channel": "PAPER",
                "contest_id": "contest", "area_id": "area",
                "content": {"area_id": "area", "contest_id": "contest", "candidate_results": {}}}),
        ),
        (
            "/review-tally-sheet",
            Permissions::TALLY_SHEET_REVIEW,
            json!({"election_event_id": event.election_event_id,
                "tally_sheet_id": "sheet", "new_status": "APPROVED"}),
        ),
        (
            "/preview-tally-sheet-import",
            Permissions::TALLY_SHEET_IMPORT_CREATE,
            json!({"election_event_id": event.election_event_id, "document_id": "document",
                "source_format": "CANONICAL_CSV", "selected_channel": "PAPER"}),
        ),
        (
            "/create-tally-sheet-import",
            Permissions::TALLY_SHEET_IMPORT_CREATE,
            json!({"election_event_id": event.election_event_id, "document_id": "document",
                "source_format": "CANONICAL_CSV", "selected_channel": "PAPER"}),
        ),
        (
            "/review-tally-sheet-import",
            Permissions::TALLY_SHEET_IMPORT_REVIEW,
            json!({"election_event_id": event.election_event_id,
                "import_id": "import", "decision": "APPROVE"}),
        ),
    ] {
        let response =
            post(&client, path, &admin(&event, &[permission]), &body).await;
        let (status, message) = text(response).await;
        assert_eq!(status, Status::InternalServerError, "{path}");
        assert!(message.contains("Connection refused"), "{path}: {message}");
    }
}

#[rocket::async_test]
async fn approving_a_tally_sheet_retires_its_older_versions() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    let (_, first) =
        create_sheet(&client, &ballot, consistent_content(&ballot)).await;
    let (_, second) =
        create_sheet(&client, &ballot, consistent_content(&ballot)).await;

    let second_id = second["id"].as_str().unwrap();
    let (status, reviewed) =
        json(review(&client, &ballot.event, second_id, "APPROVED").await).await;
    assert_eq!(status, Status::Ok, "{reviewed}");
    assert_eq!(reviewed["status"], "APPROVED");
    assert_eq!(reviewed["reviewed_by_user_id"], USER_ID);

    let deleted = rows::query(
        &services.hasura,
        "SELECT id::text FROM sequent_backend.tally_sheet
         WHERE contest_id = $1 AND deleted_at IS NOT NULL",
        &[&uuid::Uuid::parse_str(&ballot.contest_id).unwrap()],
    )
    .await;
    let deleted: Vec<String> = deleted.iter().map(|row| row.get(0)).collect();
    assert_eq!(deleted, vec![first["id"].as_str().unwrap().to_string()]);
}

#[rocket::async_test]
async fn disapproving_a_tally_sheet_keeps_its_older_versions() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    create_sheet(&client, &ballot, consistent_content(&ballot)).await;
    let (_, second) =
        create_sheet(&client, &ballot, consistent_content(&ballot)).await;

    let (status, reviewed) = json(
        review(
            &client,
            &ballot.event,
            second["id"].as_str().unwrap(),
            "DISAPPROVED",
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(reviewed["status"], "DISAPPROVED");
    let deleted = rows::query(
        &services.hasura,
        "SELECT id FROM sequent_backend.tally_sheet
         WHERE contest_id = $1 AND deleted_at IS NOT NULL",
        &[&uuid::Uuid::parse_str(&ballot.contest_id).unwrap()],
    )
    .await;
    assert!(deleted.is_empty());
}

#[rocket::async_test]
async fn a_reviewed_tally_sheet_cannot_be_reviewed_again() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    let (_, created) =
        create_sheet(&client, &ballot, consistent_content(&ballot)).await;
    let id = created["id"].as_str().unwrap();
    review(&client, &ballot.event, id, "APPROVED").await;

    let response = review(&client, &ballot.event, id, "DISAPPROVED").await;
    assert_eq!(
        text(response).await,
        (
            Status::Conflict,
            format!("Tally sheet {id} cannot be reviewed from status APPROVED")
        )
    );
}

#[rocket::async_test]
async fn reviewing_an_unknown_tally_sheet_is_not_found() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let response = review(
        &client,
        &event,
        &uuid::Uuid::new_v4().to_string(),
        "APPROVED",
    )
    .await;
    assert_eq!(
        text(response).await,
        (Status::NotFound, "Tally sheet not found".into())
    );
}

const CANONICAL_CSV: &str = "canonical.csv";

fn csv(ballot: &Ballot) -> String {
    format!(
        "channel,area_name,contest_external_id,field,candidate_external_id,value\n\
         PAPER,Precinct 1,{contest},total_votes,,10\n\
         PAPER,Precinct 1,{contest},total_valid_votes,,10\n\
         PAPER,Precinct 1,{contest},total_blank_votes,,0\n\
         PAPER,Precinct 1,{contest},census,,20\n",
        contest = ballot.contest_id
    )
}

fn import_request(event: &Event, document_id: &str, sha256: Value) -> Value {
    json!({
        "election_event_id": event.election_event_id,
        "document_id": document_id,
        "sha256": sha256,
        "source_format": "CANONICAL_CSV",
        "selected_channel": "PAPER",
    })
}

async fn upload(services: &Services, event: &Event, contents: &str) -> String {
    let document_id = event
        .document(
            &services.hasura,
            CANONICAL_CSV,
            "text/csv",
            Some(contents.len() as i64),
        )
        .await;
    services.documents.store(&document_id, contents.as_bytes());
    document_id
}

#[rocket::async_test]
async fn an_import_names_a_document_that_must_exist() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let missing = uuid::Uuid::new_v4().to_string();

    for path in ["/preview-tally-sheet-import", "/create-tally-sheet-import"] {
        let response = post(
            &client,
            path,
            &admin(&event, &[Permissions::TALLY_SHEET_IMPORT_CREATE]),
            &import_request(&event, &missing, Value::Null),
        )
        .await;
        let (status, message) = text(response).await;
        assert_eq!(status, Status::NotFound, "{path}");
        assert!(message.contains(&missing), "{path}: {message}");
    }
}

fn import_claims(event: &Event) -> Claims {
    admin(event, &[Permissions::TALLY_SHEET_IMPORT_CREATE])
}

#[rocket::async_test]
async fn a_document_larger_than_the_import_limit_is_refused_before_download() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let document_id = event
        .document(
            &services.hasura,
            CANONICAL_CSV,
            "text/csv",
            Some(50 * 1024 * 1024 + 1),
        )
        .await;

    let response = post(
        &client,
        "/preview-tally-sheet-import",
        &import_claims(&event),
        &import_request(&event, &document_id, Value::Null),
    )
    .await;
    let (status, message) = text(response).await;
    assert_eq!(status, Status::PayloadTooLarge, "{message}");
}

#[rocket::async_test]
async fn an_unreadable_import_document_is_a_server_error() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    // The row exists, but object storage has nothing under its id.
    let document_id = event
        .document(&services.hasura, CANONICAL_CSV, "text/csv", Some(10))
        .await;

    let response = post(
        &client,
        "/create-tally-sheet-import",
        &import_claims(&event),
        &import_request(&event, &document_id, Value::Null),
    )
    .await;
    let (status, message) = text(response).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(message.contains("No stored object"), "{message}");
}

#[rocket::async_test]
async fn a_source_whose_digest_does_not_match_is_refused() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    let contents = csv(&ballot);
    let document_id = upload(&services, &ballot.event, &contents).await;
    let wrong = hash_bytes(b"another file");

    for path in ["/preview-tally-sheet-import", "/create-tally-sheet-import"] {
        let response = post(
            &client,
            path,
            &import_claims(&ballot.event),
            &import_request(&ballot.event, &document_id, json!(wrong)),
        )
        .await;
        let (status, message) = text(response).await;
        assert_eq!(status, Status::BadRequest, "{path}");
        assert!(
            message.starts_with(&format!(
                "Uploaded source SHA-256 mismatch: expected {wrong}, got {}",
                hash_bytes(contents.as_bytes())
            )),
            "{path}: {message}"
        );
    }
}

#[rocket::async_test]
async fn a_preview_reads_the_source_without_storing_an_import() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    let contents = csv(&ballot);
    let document_id = upload(&services, &ballot.event, &contents).await;

    // Digests compare case-insensitively.
    let digest = hash_bytes(contents.as_bytes()).to_uppercase();
    let (status, body) = json(
        post(
            &client,
            "/preview-tally-sheet-import",
            &import_claims(&ballot.event),
            &import_request(&ballot.event, &document_id, json!(digest)),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert!(body["preview"].is_object(), "{body}");
    let imports = rows::query(
        &services.hasura,
        "SELECT id FROM sequent_backend.tally_sheet_import
         WHERE election_event_id = $1",
        &[&uuid::Uuid::parse_str(&ballot.event.election_event_id).unwrap()],
    )
    .await;
    assert!(imports.is_empty());
}

#[rocket::async_test]
async fn creating_an_import_stores_it_for_review() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    let document_id = upload(&services, &ballot.event, &csv(&ballot)).await;

    let (status, body) = json(
        post(
            &client,
            "/create-tally-sheet-import",
            &import_claims(&ballot.event),
            &import_request(&ballot.event, &document_id, Value::Null),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let import_id = body["import"]["id"].as_str().expect("the import id");
    let stored = rows::query(
        &services.hasura,
        "SELECT source_document_id::text, created_by_user_id
         FROM sequent_backend.tally_sheet_import WHERE id = $1",
        &[&uuid::Uuid::parse_str(import_id).unwrap()],
    )
    .await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].get::<_, String>(0), document_id);
    assert_eq!(stored[0].get::<_, String>(1), USER_ID);
}

/// A ballot whose area is assigned its contest, which has one candidate:
/// what a canonical CSV import can resolve.
struct Importable {
    ballot: Ballot,
    candidate_id: String,
}

async fn importable(services: &Services) -> Importable {
    let ballot = ballot(services).await;
    ballot
        .event
        .area_contest(&services.hasura, &ballot.area_id, &ballot.contest_id)
        .await;
    let candidate_id = ballot
        .event
        .candidate(&services.hasura, &ballot.contest_id)
        .await;
    Importable {
        ballot,
        candidate_id,
    }
}

fn complete_csv(importable: &Importable) -> String {
    format!(
        "{}PAPER,Precinct 1,{contest},implicit_invalid,,0\n\
         PAPER,Precinct 1,{contest},explicit_invalid,,0\n\
         PAPER,Precinct 1,{contest},candidate_votes,{candidate},10\n",
        csv(&importable.ballot),
        contest = importable.ballot.contest_id,
        candidate = importable.candidate_id,
    )
}

/// Create an import of `contents` and return its body.
async fn create_import(
    services: &Services,
    client: &Client,
    event: &Event,
    contents: &str,
) -> Value {
    let document_id = upload(services, event, contents).await;
    let (status, body) = json(
        post(
            client,
            "/create-tally-sheet-import",
            &import_claims(event),
            &import_request(event, &document_id, Value::Null),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    body["import"].clone()
}

async fn review_import<'c>(
    client: &'c Client,
    event: &Event,
    import_id: &str,
    decision: &str,
) -> rocket::local::asynchronous::LocalResponse<'c> {
    post(
        client,
        "/review-tally-sheet-import",
        &admin(event, &[Permissions::TALLY_SHEET_IMPORT_REVIEW]),
        &json!({
            "election_event_id": event.election_event_id,
            "import_id": import_id,
            "decision": decision,
        }),
    )
    .await
}

#[rocket::async_test]
async fn reviewing_an_unknown_import_is_not_found() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let missing = uuid::Uuid::new_v4().to_string();

    let (status, message) =
        text(review_import(&client, &event, &missing, "APPROVE").await).await;
    assert_eq!(status, Status::NotFound);
    assert!(message.contains(&missing), "{message}");
}

#[rocket::async_test]
async fn an_import_that_failed_validation_cannot_be_reviewed() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let ballot = ballot(&services).await;
    let import =
        create_import(&services, &client, &ballot.event, &csv(&ballot)).await;
    assert_eq!(import["status"], "FAILED_VALIDATION");

    let (status, message) = text(
        review_import(
            &client,
            &ballot.event,
            import["id"].as_str().unwrap(),
            "APPROVE",
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Conflict, "{message}");
}

/// Completed tallies over the import's election and over another one, and
/// an unfinished tally over the import's election.
struct Tallies {
    recountable: String,
    other_election: String,
    unfinished: String,
}

async fn tallies(services: &Services, ballot: &Ballot) -> Tallies {
    let event = &ballot.event;
    let other_election_id = event.election(&services.hasura).await;
    Tallies {
        recountable: event
            .tally_session(
                &services.hasura,
                &[ballot.election_id.clone()],
                "SUCCESS",
            )
            .await,
        other_election: event
            .tally_session(&services.hasura, &[other_election_id], "SUCCESS")
            .await,
        unfinished: event
            .tally_session(
                &services.hasura,
                &[ballot.election_id.clone()],
                "IN_PROGRESS",
            )
            .await,
    }
}

#[rocket::async_test]
async fn approving_an_import_recounts_the_completed_tallies_of_its_elections() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let importable = importable(&services).await;
    let event = &importable.ballot.event;
    event
        .present(
            &services.hasura,
            json!({"automatic_recount_policy": "enabled"}),
        )
        .await;
    let tallies = tallies(&services, &importable.ballot).await;
    let import =
        create_import(&services, &client, event, &complete_csv(&importable))
            .await;
    assert_eq!(import["status"], "PENDING_REVIEW", "{import}");

    let (status, body) = json(
        review_import(
            &client,
            event,
            import["id"].as_str().unwrap(),
            "APPROVE",
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["import"]["status"], "APPROVED");

    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].name, "execute_tally_session");
    assert_eq!(sent[0].kwargs["tally_session_id"], tallies.recountable);
    assert_eq!(sent[0].kwargs["force_new_results_id"], true);
    // The recount is recorded as a new execution of that tally only.
    let executions = |id: String| {
        let pool = services.hasura.clone();
        async move { event.tally_session_executions(&pool, &id).await }
    };
    assert_eq!(executions(tallies.recountable.clone()).await, 2);
    assert_eq!(executions(tallies.other_election.clone()).await, 1);
    assert_eq!(executions(tallies.unfinished.clone()).await, 1);
}

#[rocket::async_test]
async fn an_approved_import_is_not_recounted_while_the_policy_is_disabled() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let importable = importable(&services).await;
    let event = &importable.ballot.event;
    let tallies = tallies(&services, &importable.ballot).await;
    let import =
        create_import(&services, &client, event, &complete_csv(&importable))
            .await;

    let (status, body) = json(
        review_import(
            &client,
            event,
            import["id"].as_str().unwrap(),
            "APPROVE",
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["import"]["status"], "APPROVED");
    assert!(services.tasks.sent().is_empty());
    assert_eq!(
        event
            .tally_session_executions(&services.hasura, &tallies.recountable)
            .await,
        1
    );
}

#[rocket::async_test]
async fn a_disapproved_import_is_never_recounted() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let importable = importable(&services).await;
    let event = &importable.ballot.event;
    event
        .present(
            &services.hasura,
            json!({"automatic_recount_policy": "enabled"}),
        )
        .await;
    tallies(&services, &importable.ballot).await;
    let import =
        create_import(&services, &client, event, &complete_csv(&importable))
            .await;

    let (status, body) = json(
        review_import(
            &client,
            event,
            import["id"].as_str().unwrap(),
            "DISAPPROVE",
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["import"]["status"], "DISAPPROVED");
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_recount_whose_task_cannot_be_sent_stays_requested_and_the_review_succeeds(
) {
    let services = Services::on_test_database()
        .await
        .with_tasks(MemoryTaskQueue::refusing());
    let client = services.client().await;
    let importable = importable(&services).await;
    let event = &importable.ballot.event;
    event
        .present(
            &services.hasura,
            json!({"automatic_recount_policy": "enabled"}),
        )
        .await;
    let tallies = tallies(&services, &importable.ballot).await;
    let import =
        create_import(&services, &client, event, &complete_csv(&importable))
            .await;

    let (status, body) = json(
        review_import(
            &client,
            event,
            import["id"].as_str().unwrap(),
            "APPROVE",
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    // process_board delivers the durable request later.
    assert_eq!(
        event
            .tally_session_executions(&services.hasura, &tallies.recountable)
            .await,
        2
    );
}
