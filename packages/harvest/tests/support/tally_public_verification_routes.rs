// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The boards a tally session's public verification reads, from the request
//! to the rows of the migrated test database.

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use deadpool_postgres::Pool;
use rocket::http::{ContentType, Status};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use uuid::Uuid;

const PATH: &str = "/tally-public-verification-boards";
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
