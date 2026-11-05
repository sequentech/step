// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Phone blacklist entries on the migrated test database: each change is
//! committed with its entry for the election event's log, queued in the
//! same transaction.

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

// Entries record their creator as a version 4 UUID.
const USER_ID: &str = "3f2b8c1e-6d4a-4c1b-9e7f-2a5d8b0c9e1a";
const USERNAME: &str = "ada.admin";
const PHONE: &str = "+34600000001";

fn admin(event: &Event, permission: Permissions) -> Claims {
    Claims::new(&event.tenant_id, USER_ID)
        .username(USERNAME)
        .roles([permission])
}

async fn create<'c>(client: &'c Client, event: &Event) -> LocalResponse<'c> {
    post(
        client,
        "/create-phone-blacklist-entry",
        &admin(event, Permissions::PHONE_BLACKLIST_CREATE),
        &json!({
            "election_event_id": event.election_event_id,
            "phone_e164": PHONE,
            "reason": "reported abuse",
        }),
    )
    .await
}

async fn delete<'c>(
    client: &'c Client,
    event: &Event,
    id: &str,
) -> LocalResponse<'c> {
    post(
        client,
        "/delete-phone-blacklist-entry",
        &admin(event, Permissions::PHONE_BLACKLIST_DELETE),
        &json!({"election_event_id": event.election_event_id, "id": id}),
    )
    .await
}

async fn stored_numbers(services: &Services, event: &Event) -> Vec<String> {
    rows::query(
        &services.hasura,
        "SELECT phone_e164 FROM sequent_backend.phone_blacklist
         WHERE election_event_id = $1",
        &[&uuid::Uuid::parse_str(&event.election_event_id).unwrap()],
    )
    .await
    .iter()
    .map(|row| row.get(0))
    .collect()
}

/// The entries queued for the event's log: (description, details).
async fn logged(services: &Services, event: &Event) -> Vec<(String, Value)> {
    rows::query(
        &services.hasura,
        "SELECT description, details FROM sequent_backend.audit_log_outbox
         WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid
             AND statement_kind = 'AdminChange' AND user_id = $3
         ORDER BY seq",
        &[&event.tenant_id, &event.election_event_id, &USER_ID],
    )
    .await
    .iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

/// An entry's details for the blacklist entry `id`.
fn details(id: &str, action: &str, changes: Value) -> Value {
    json!({
        "object": {"type": "phone_blacklist", "id": id, "name": PHONE},
        "action": action,
        "actor": {"user_id": USER_ID, "username": USERNAME, "role": "user"},
        "origin": "harvest",
        "changes": changes,
    })
}

fn created(id: &str) -> (String, Value) {
    (
        format!("Created phone blacklist \"{PHONE}\""),
        details(
            id,
            "create",
            json!([
                {"path": "phone_e164", "new": PHONE},
                {"path": "reason", "new": "reported abuse"},
            ]),
        ),
    )
}

fn deleted(id: &str) -> (String, Value) {
    (
        format!("Deleted phone blacklist \"{PHONE}\""),
        details(
            id,
            "delete",
            json!([
                {"path": "phone_e164", "old": PHONE},
                {"path": "reason", "old": "reported abuse"},
            ]),
        ),
    )
}

#[rocket::async_test]
async fn a_blacklisted_number_is_stored_with_its_electoral_log_entry() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, entry) = json(create(&client, &event).await).await;
    assert_eq!(status, Status::Ok, "{entry}");
    assert_eq!(entry["phone_e164"], PHONE);
    assert_eq!(entry["reason"], "reported abuse");
    assert_eq!(entry["created_by"], USER_ID);
    assert_eq!(stored_numbers(&services, &event).await, vec![PHONE]);
    assert_eq!(
        logged(&services, &event).await,
        vec![created(entry["id"].as_str().unwrap())]
    );
}

#[rocket::async_test]
async fn a_number_already_blacklisted_is_refused() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    create(&client, &event).await;

    let (status, message) = text(create(&client, &event).await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with("Failed to insert the entry: "),
        "{message}"
    );
    assert_eq!(logged(&services, &event).await.len(), 1);
}

#[rocket::async_test]
async fn deleting_an_entry_logs_the_number_it_removes() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (_, entry) = json(create(&client, &event).await).await;
    let id = entry["id"].as_str().unwrap();

    assert_eq!(
        json(delete(&client, &event, id).await).await,
        (Status::Ok, json!({ "id": id }))
    );
    assert!(stored_numbers(&services, &event).await.is_empty());
    assert_eq!(
        logged(&services, &event).await,
        vec![created(id), deleted(id)]
    );
}

#[rocket::async_test]
async fn deleting_an_unknown_entry_fails() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, message) =
        text(delete(&client, &event, &uuid::Uuid::new_v4().to_string()).await)
            .await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with("Failed to delete the entry: "),
        "{message}"
    );
    assert!(logged(&services, &event).await.is_empty());
}

/// The entry is queued by the change's own transaction: nothing reaches the
/// bulletin board before the commit, so an unreachable board refuses no
/// change.
#[rocket::async_test]
async fn changes_do_not_wait_for_the_bulletin_board() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, entry) = json(create(&client, &event).await).await;
    assert_eq!(status, Status::Ok, "{entry}");
    let unposted = rows::query(
        &services.hasura,
        "SELECT count(*) FROM sequent_backend.audit_log_outbox
         WHERE election_event_id = $1::text::uuid AND posted_at IS NULL",
        &[&event.election_event_id],
    )
    .await;
    assert_eq!(unposted[0].get::<_, i64>(0), 1);
    assert!(services.electoral_log.entries().is_empty());
}

#[rocket::async_test]
async fn phone_blacklist_routes_answer_500_when_the_database_is_unreachable() {
    let services = Services::without_database();
    let client = services.client().await;
    let event = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    for response in [
        create(&client, &event).await,
        delete(&client, &event, "entry").await,
    ] {
        let (status, message) = text(response).await;
        assert_eq!(status, Status::InternalServerError);
        assert!(
            message.starts_with("Failed to get client from the db pool"),
            "{message}"
        );
    }
}
