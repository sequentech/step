// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The electoral-log proof routes on the migrated test database and the
//! event's own electoral-log database. Checkpoints and proofs cover the whole
//! log, so they answer only admins whose permission labels allow every
//! election of the event.

use crate::route_services::rows::{self, Event};
use crate::route_services::{
    configure_super_admin_tenant, json, post, text, Services,
    SUPER_ADMIN_TENANT_ID,
};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::sync::Once;
use windmill::services::protocol_manager::get_event_board;

/// The environment slug boards are named with, unless the environment sets
/// `ENV_SLUG`.
const TEST_SLUG: &str = "harvesttest";

fn env_slug() -> String {
    static SLUG: Once = Once::new();
    SLUG.call_once(|| {
        if std::env::var("ENV_SLUG").is_err() {
            std::env::set_var("ENV_SLUG", TEST_SLUG);
        }
    });
    std::env::var("ENV_SLUG").expect("ENV_SLUG")
}

/// An event with an election labelled `north` and one labelled `south`, and
/// a record of a vote in the north on the event's board.
async fn labelled_event(services: &Services) -> (Event, i64) {
    let event = rows::event(&services.hasura).await;
    let north = event.labelled_election(&services.hasura, "north").await;
    event.labelled_election(&services.hasura, "south").await;
    let store = event.ballot_box(&services.hasura).await;
    let board = get_event_board(
        &event.tenant_id,
        &event.election_event_id,
        &env_slug(),
    );
    let record = event
        .cast_vote_record(&store, &board, &north, "voter-north")
        .await;
    (event, record)
}

fn logs_reader(tenant_id: &str) -> Claims {
    Claims::new(tenant_id, "logs-reader").roles([Permissions::LOGS_READ])
}

fn request(event: &Event) -> Value {
    json!({
        "tenant_id": event.tenant_id,
        "election_event_id": event.election_event_id,
    })
}

fn with(mut body: Value, field: &str, value: Value) -> Value {
    body[field] = value;
    body
}

async fn checkpoint(
    client: &Client,
    claims: &Claims,
    event: &Event,
) -> (Status, Value) {
    json(
        post(client, "/electoral-log/checkpoint", claims, &request(event))
            .await,
    )
    .await
}

#[rocket::async_test]
async fn proofs_refuse_admins_whose_labels_leave_out_an_election() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, record) = labelled_event(&services).await;
    let north = logs_reader(&event.tenant_id).permission_labels(&["north"]);
    let both =
        logs_reader(&event.tenant_id).permission_labels(&["north", "south"]);
    let (status, body) = checkpoint(&client, &both, &event).await;
    assert_eq!(status, Status::Ok, "{body}");
    let trusted = body["checkpoint"].clone();

    let requests = [
        ("/electoral-log/checkpoint", request(&event)),
        (
            "/electoral-log/inclusion",
            with(request(&event), "record_id", json!(record)),
        ),
        (
            "/electoral-log/consistency",
            with(request(&event), "checkpoint", trusted),
        ),
    ];
    for (path, body) in &requests {
        let (status, message) =
            text(post(&client, path, &north, body).await).await;
        assert_eq!(status, Status::Forbidden, "{path}: {message}");
    }
}

#[rocket::async_test]
async fn proofs_answer_admins_whose_labels_allow_every_election() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, record) = labelled_event(&services).await;
    let both =
        logs_reader(&event.tenant_id).permission_labels(&["north", "south"]);

    let (status, body) = checkpoint(&client, &both, &event).await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["checkpoint"]["tree_size"], 1, "{body}");
    let trusted = body["checkpoint"].clone();
    let (status, body) = json(
        post(
            &client,
            "/electoral-log/inclusion",
            &both,
            &with(request(&event), "record_id", json!(record)),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["entry"]["message"]["id"], record, "{body}");
    let (status, body) = json(
        post(
            &client,
            "/electoral-log/consistency",
            &both,
            &with(request(&event), "checkpoint", trusted),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
}

/// Labels name elections of the user's own tenant, so users of the
/// super-admin tenant read other tenants' logs whatever their labels.
#[rocket::async_test]
async fn super_admins_read_other_tenants_logs_whatever_their_labels() {
    configure_super_admin_tenant();
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let (event, _) = labelled_event(&services).await;

    let (status, body) =
        checkpoint(&client, &logs_reader(SUPER_ADMIN_TENANT_ID), &event).await;
    assert_eq!(status, Status::Ok, "{body}");
}
