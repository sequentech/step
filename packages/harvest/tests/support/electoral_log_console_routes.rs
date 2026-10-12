// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The electoral-log console on the migrated test database and the event's
//! own electoral-log database. The console reads the event's whole database,
//! so it answers only admins whose permission labels allow every election of
//! the event.

use crate::route_services::rows::{self, Event};
use crate::route_services::{
    configure_super_admin_tenant, json, post, Services, SUPER_ADMIN_TENANT_ID,
};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

/// An event with an election labelled `north` and one labelled `south`, a
/// ballot in each, and the record of the northern vote on its board.
struct LabelledEvent {
    event: Event,
    record: i64,
}

async fn labelled_event(services: &Services, event: Event) -> LabelledEvent {
    let north = event.labelled_election(&services.hasura, "north").await;
    let south = event.labelled_election(&services.hasura, "south").await;
    let area_id = event.area(&services.hasura, "Area").await;
    let ballot_box = event.ballot_box(&services.hasura).await;
    event
        .cast_vote(&ballot_box, &north, &area_id, "voter-north")
        .await;
    event
        .cast_vote(&ballot_box, &south, &area_id, "voter-south")
        .await;
    let record = event
        .cast_vote_record(&ballot_box, &event.board(), &north, "voter-north")
        .await;
    LabelledEvent { event, record }
}

fn reader(tenant_id: &str) -> Claims {
    Claims::new(tenant_id, "console-reader")
        .roles([Permissions::ELECTORAL_LOG_CONSOLE_READ])
}

fn querier(labels: &[&str]) -> Claims {
    Claims::new(SUPER_ADMIN_TENANT_ID, "console-querier")
        .roles([
            Permissions::ELECTORAL_LOG_CONSOLE_QUERY,
            Permissions::ELECTORAL_LOG_PERSONAL_DATA_READ,
        ])
        .permission_labels(labels)
}

async fn page(
    client: &Client,
    claims: &Claims,
    event: &Event,
    table: &str,
) -> (Status, Value) {
    json(
        post(
            client,
            "/electoral-log-console/page",
            claims,
            &json!({
                "tenant_id": event.tenant_id,
                "election_event_id": event.election_event_id,
                "table": table,
            }),
        )
        .await,
    )
    .await
}

async fn record(
    client: &Client,
    claims: &Claims,
    fixture: &LabelledEvent,
) -> (Status, Value) {
    json(
        post(
            client,
            "/electoral-log-console/record",
            claims,
            &json!({
                "election_event_id": fixture.event.election_event_id,
                "position": fixture.record,
            }),
        )
        .await,
    )
    .await
}

async fn query(
    client: &Client,
    claims: &Claims,
    event: &Event,
) -> (Status, Value) {
    json(
        post(
            client,
            "/electoral-log-console/query",
            claims,
            &json!({
                "tenant_id": event.tenant_id,
                "election_event_id": event.election_event_id,
                "sql": "SELECT election_id::text FROM ballot_box_ballot",
            }),
        )
        .await,
    )
    .await
}

fn assert_refused(status: Status, body: &Value) {
    assert_eq!(status, Status::Forbidden, "{body}");
    assert_eq!(body["extensions"]["code"], "Unauthorized", "{body}");
}

fn row_count(body: &Value) -> usize {
    body["rows"].as_array().map(Vec::len).unwrap_or_default()
}

#[rocket::async_test]
async fn the_console_refuses_admins_whose_labels_leave_out_an_election() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let fixture =
        labelled_event(&services, rows::event(&services.hasura).await).await;
    let event = &fixture.event;
    let north = reader(&event.tenant_id).permission_labels(&["north"]);

    for table in ["records", "ballots", "voters", "queue"] {
        let (status, body) = page(&client, &north, event, table).await;
        assert_refused(status, &body);
    }
    let (status, body) = record(&client, &north, &fixture).await;
    assert_refused(status, &body);
}

#[rocket::async_test]
async fn the_console_answers_admins_whose_labels_allow_every_election() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let fixture =
        labelled_event(&services, rows::event(&services.hasura).await).await;
    let event = &fixture.event;
    let both = reader(&event.tenant_id).permission_labels(&["north", "south"]);

    let (status, body) = page(&client, &both, event, "ballots").await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(row_count(&body), 2, "{body}");
    let (status, body) = page(&client, &both, event, "records").await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(row_count(&body), 1, "{body}");
    let (status, body) = record(&client, &both, &fixture).await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["position"], fixture.record, "{body}");
    assert_eq!(body["kind"], "CastVote", "{body}");
}

/// Labels name elections of the user's own tenant, so users of the
/// super-admin tenant browse other tenants' events whatever their labels.
#[rocket::async_test]
async fn super_admins_browse_other_tenants_events_whatever_their_labels() {
    configure_super_admin_tenant();
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let fixture =
        labelled_event(&services, rows::event(&services.hasura).await).await;
    let super_admin = reader(SUPER_ADMIN_TENANT_ID);

    let (status, body) =
        page(&client, &super_admin, &fixture.event, "ballots").await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(row_count(&body), 2, "{body}");
    let (status, body) = query(&client, &querier(&[]), &fixture.event).await;
    assert_eq!(status, Status::Ok, "{body}");
}

#[rocket::async_test]
async fn queries_need_labels_that_allow_every_election_of_the_event() {
    configure_super_admin_tenant();
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let fixture = labelled_event(
        &services,
        rows::event_of(&services.hasura, SUPER_ADMIN_TENANT_ID).await,
    )
    .await;
    let event = &fixture.event;

    let (status, body) = query(&client, &querier(&["north"]), event).await;
    assert_refused(status, &body);
    let (status, body) =
        query(&client, &querier(&["north", "south"]), event).await;
    assert_eq!(status, Status::Ok, "{body}");
    // Without ELECTORAL_LOG_PG_READER_USER the console has no role to query as.
    match body.get("error") {
        Some(error) => assert_eq!(
            error, "Queries are not available in this environment",
            "{body}"
        ),
        None => assert_eq!(row_count(&body), 2, "{body}"),
    }
}
