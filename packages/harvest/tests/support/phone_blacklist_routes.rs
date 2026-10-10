// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Phone blacklist entries on the migrated test database: each change is
//! committed only with its electoral-log entry.

use crate::adapters::memory::electoral_log::{
    LoggedEntry, MemoryElectoralLogs,
};
use crate::ports::electoral_log::PhoneBlacklistChange;
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::types::permissions::Permissions;
use serde_json::json;

// Entries record their creator as a version 4 UUID.
const USER_ID: &str = "3f2b8c1e-6d4a-4c1b-9e7f-2a5d8b0c9e1a";
const PHONE: &str = "+34600000001";

fn admin(event: &Event, permission: Permissions) -> Claims {
    Claims::new(&event.tenant_id, USER_ID).roles([permission])
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

fn logged(event: &Event, change: PhoneBlacklistChange) -> LoggedEntry {
    LoggedEntry::PhoneBlacklistEntry {
        change,
        election_event_id: event.election_event_id.clone(),
        admin_id: USER_ID.into(),
        phone_e164: PHONE.into(),
    }
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
        services.electoral_log.entries(),
        vec![logged(&event, PhoneBlacklistChange::Created)]
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
    assert_eq!(services.electoral_log.entries().len(), 1);
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
        services.electoral_log.entries(),
        vec![
            logged(&event, PhoneBlacklistChange::Created),
            logged(&event, PhoneBlacklistChange::Deleted)
        ]
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
    assert!(services.electoral_log.entries().is_empty());
}

#[rocket::async_test]
async fn changes_whose_electoral_log_entry_fails_are_rolled_back() {
    let services = Services::on_test_database()
        .await
        .with_electoral_log(MemoryElectoralLogs::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, message) = text(create(&client, &event).await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with("Failed to post the electoral log message"),
        "{message}"
    );
    assert!(stored_numbers(&services, &event).await.is_empty());

    // An entry committed before the log failed stays when its deletion
    // cannot be logged.
    let id = uuid::Uuid::new_v4();
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.phone_blacklist
            (id, tenant_id, election_event_id, phone_e164, created_by)
         VALUES ($1, $2, $3, $4, $5)",
        &[
            &id,
            &uuid::Uuid::parse_str(&event.tenant_id).unwrap(),
            &uuid::Uuid::parse_str(&event.election_event_id).unwrap(),
            &PHONE,
            &uuid::Uuid::parse_str(USER_ID).unwrap(),
        ],
    )
    .await;
    let (status, message) =
        text(delete(&client, &event, &id.to_string()).await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with("Failed to post the electoral log message"),
        "{message}"
    );
    assert_eq!(stored_numbers(&services, &event).await, vec![PHONE]);
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
