// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The boards a tally session's public verification reads, and each board's
//! transcript, from the request to the rows of the migrated test database
//! and a local stand-in for b4 and its storage.

use crate::route_services::http::{Exchange, HttpServer};
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use base64::{engine::general_purpose::STANDARD, Engine};
use deadpool_postgres::Pool;
use rocket::http::{ContentType, Status};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use uuid::Uuid;

const PATH: &str = "/tally-public-verification-boards";
const TRANSCRIPT: &str = "/tally-public-verification-transcript";
const UNREADABLE: &str = "the tally session's boards could not be read";

fn admin(tenant_id: &str, permissions: &[Permissions]) -> Claims {
    Claims::new(tenant_id, "tally-admin").roles(permissions)
}

fn reader(tenant_id: &str) -> Claims {
    admin(tenant_id, &[Permissions::TALLY_RESULTS_READ])
}

fn body(election_event_id: &str, tally_session_id: &str) -> Value {
    json!({
        "election_event_id": election_event_id,
        "tally_session_id": tally_session_id,
    })
}

fn uuid(id: &str) -> Uuid {
    Uuid::parse_str(id).expect("fixture ids are UUIDs")
}

/// A finished tally session of `event` whose keys ceremony has a DKG board
/// with one tally board; returns the session and the two board names.
async fn session_with_boards(
    pool: &Pool,
    event: &Event,
) -> (String, String, String) {
    let session = event.tally_session(pool, &[], "SUCCESS").await;
    let ceremony: Uuid = rows::query(
        pool,
        "SELECT keys_ceremony_id FROM sequent_backend.tally_session
         WHERE id = $1",
        &[&uuid(&session)],
    )
    .await[0]
        .get(0);
    let (tenant_id, event_id) =
        (uuid(&event.tenant_id), uuid(&event.election_event_id));
    let dkg_id = Uuid::new_v4();
    let dkg = format!("dkg_{}", ceremony.simple());
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.protocol_board
            (id, tenant_id, election_event_id, keys_ceremony_id, name,
             manager_message)
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &dkg_id,
            &tenant_id,
            &event_id,
            &ceremony,
            &dkg,
            &b"signed configuration".to_vec(),
        ],
    )
    .await;
    let tally = format!("tally_{}_0", uuid(&session).simple());
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.protocol_board
            (tenant_id, election_event_id, parent_id, keys_ceremony_id,
             tally_session_id, batch, name, manager_message)
         VALUES ($1, $2, $3, $4, $5, 0, $6, $7)",
        &[
            &tenant_id,
            &event_id,
            &dkg_id,
            &ceremony,
            &uuid(&session),
            &tally,
            &b"ballots".to_vec(),
        ],
    )
    .await;
    (session, dkg, tally)
}

#[rocket::async_test]
async fn a_reader_gets_the_boards_of_a_tally_session_of_its_own_tenant() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (session, dkg, tally) =
        session_with_boards(&services.hasura, &event).await;
    let request = body(&event.election_event_id, &session);

    let (status, boards) =
        json(post(&client, PATH, &reader(&event.tenant_id), &request).await)
            .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(
        boards,
        json!({"boards": [{
            "dkg_board": dkg,
            "tally_board": tally,
            "batch": 0,
            "election_presentation": null,
            "contest_presentation": null,
            "area_name": null,
            "configuration_message": "c2lnbmVkIGNvbmZpZ3VyYXRpb24=",
            "ballots_message": "YmFsbG90cw==",
        }]})
    );

    // Another tenant's reader asking for the same ids gets none.
    let other = rows::event(&services.hasura).await;
    let (status, boards) =
        json(post(&client, PATH, &reader(&other.tenant_id), &request).await)
            .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(boards, json!({"boards": []}));
}

#[rocket::async_test]
async fn callers_without_a_token_or_the_permission_are_refused() {
    let client = Services::without_database().client().await;
    let tenant_id = Uuid::new_v4().to_string();
    let id = Uuid::new_v4().to_string();
    let request = body(&id, &id);

    let response = client
        .post(PATH)
        .header(ContentType::JSON)
        .body(request.to_string())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let (status, _) = text(
        post(
            &client,
            PATH,
            &admin(&tenant_id, &[Permissions::TALLY_READ]),
            &request,
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Unauthorized);
}

#[rocket::async_test]
async fn malformed_ids_are_refused_and_failures_are_not_detailed() {
    let tenant_id = Uuid::new_v4().to_string();
    let id = Uuid::new_v4().to_string();
    let refused = Services::without_database().client().await;
    for request in [
        body("not-a-uuid", &id),
        body(&id, "not-a-uuid"),
        json!({ "election_event_id": id }),
    ] {
        let response =
            post(&refused, PATH, &reader(&tenant_id), &request).await;
        assert_eq!(response.status(), Status::UnprocessableEntity, "{request}");
    }

    // The database refuses the connection.
    assert_eq!(
        text(post(&refused, PATH, &reader(&tenant_id), &body(&id, &id)).await)
            .await,
        (Status::InternalServerError, UNREADABLE.to_string())
    );

    // The query fails: the tenant claim is not a UUID.
    let services = Services::on_test_database().await;
    let client = services.client().await;
    assert_eq!(
        text(post(&client, PATH, &reader("not-a-uuid"), &body(&id, &id)).await)
            .await,
        (Status::InternalServerError, UNREADABLE.to_string())
    );
}

fn transcript_body(event: &Event, session: &str, tally_board: &str) -> Value {
    json!({
        "election_event_id": event.election_event_id,
        "tally_session_id": session,
        "tally_board": tally_board,
    })
}

/// A message b4 keeps inline, as its message list serves it.
fn inline(id: &str, bytes: &[u8]) -> Value {
    json!({
        "id": id,
        "content_type": {"message": STANDARD.encode(bytes)},
        "version": "2",
        "download_url": null,
    })
}

/// A message whose body is in storage, at `download_url`.
fn stored(id: &str, download_url: &str) -> Value {
    json!({
        "id": id,
        "content_type": {"key": format!("messages/{id}")},
        "version": "2",
        "download_url": download_url,
    })
}

fn messages(path: &str, messages: Vec<Value>) -> Exchange {
    Exchange::json("GET", path, 200, json!({ "messages": messages }))
}

#[rocket::async_test]
async fn a_reader_gets_a_boards_transcript_read_from_b4_and_its_storage() {
    let storage = HttpServer::start(vec![Exchange::json(
        "GET",
        "/wbraid-messages/mix",
        200,
        Value::Null,
    )
    .body("mixed ballots")]);
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let (session, dkg, tally) =
        session_with_boards(&services.hasura, &event).await;
    let b4 = HttpServer::start(vec![
        messages(
            &format!("/boards/{dkg}/messages"),
            vec![inline("c", b"configuration")],
        ),
        messages(
            &format!("/boards/{tally}/messages"),
            vec![
                inline("b", b"ballots"),
                stored("m", &format!("{}/wbraid-messages/mix", storage.url)),
            ],
        ),
    ]);
    let services = services.with_b4_url(&b4.url);
    let client = services.client().await;
    let caller = reader(&event.tenant_id);

    let (status, transcript) = json(
        post(
            &client,
            TRANSCRIPT,
            &caller,
            &transcript_body(&event, &session, &tally),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(
        transcript,
        json!({
            "dkg_messages": [STANDARD.encode("configuration")],
            "tally_messages": [
                STANDARD.encode("ballots"),
                STANDARD.encode("mixed ballots"),
            ],
        })
    );

    // A board of another tally session is not read.
    let other = format!("tally_{}_0", Uuid::new_v4().simple());
    assert_eq!(
        text(
            post(
                &client,
                TRANSCRIPT,
                &caller,
                &transcript_body(&event, &session, &other),
            )
            .await
        )
        .await,
        (
            Status::NotFound,
            "not a tally board of this tally session".to_string()
        )
    );
    assert_eq!(b4.finish().len(), 2);
    assert_eq!(storage.finish().len(), 1);
}

#[rocket::async_test]
async fn a_board_service_that_is_unset_or_fails_is_a_bad_gateway() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let (session, _, tally) =
        session_with_boards(&services.hasura, &event).await;
    let caller = reader(&event.tenant_id);
    let request = transcript_body(&event, &session, &tally);

    // WBRAID_B4_URL is unset.
    let unset = services.client().await;
    assert_eq!(
        text(post(&unset, TRANSCRIPT, &caller, &request).await).await,
        (
            Status::BadGateway,
            "the board service is not configured".to_string()
        )
    );

    // b4 answers nothing it was asked for.
    let b4 = HttpServer::start(vec![]);
    let failing = services.with_b4_url(&b4.url).client().await;
    assert_eq!(
        text(post(&failing, TRANSCRIPT, &caller, &request).await).await,
        (
            Status::BadGateway,
            "the board service could not be read".to_string()
        )
    );
    assert_eq!(b4.finish().len(), 1);
}

#[rocket::async_test]
async fn a_stored_board_name_that_does_not_parse_is_an_internal_error() {
    let b4 = HttpServer::start(vec![]);
    let services = Services::on_test_database().await.with_b4_url(&b4.url);
    let event = rows::event(&services.hasura).await;
    let (session, dkg, tally) =
        session_with_boards(&services.hasura, &event).await;
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.protocol_board SET name = 'not-a-board'
         WHERE name = $1",
        &[&dkg],
    )
    .await;
    let client = services.client().await;
    assert_eq!(
        text(
            post(
                &client,
                TRANSCRIPT,
                &reader(&event.tenant_id),
                &transcript_body(&event, &session, &tally),
            )
            .await
        )
        .await,
        (Status::InternalServerError, UNREADABLE.to_string())
    );
    assert!(b4.finish().is_empty());
}
