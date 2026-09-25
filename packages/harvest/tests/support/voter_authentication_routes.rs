// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Voter enrollment and OTP settings: only changes against the election
//! event's presentation reach Keycloak.

use crate::adapters::memory::identity::{
    AuthenticationUpdate, LocalIdentityAdmin,
};
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::{Client, LocalResponse};
use serde_json::json;

/// The route checks an empty permission list: any admin of the tenant.
fn admin(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, "any-admin")
}

async fn set<'c>(
    client: &'c Client,
    event: &Event,
    enrollment: &str,
    otp: &str,
) -> LocalResponse<'c> {
    post(
        client,
        "/set-voter-authentication",
        &admin(event),
        &json!({
            "election_event_id": event.election_event_id,
            "enrollment": enrollment,
            "otp": otp,
        }),
    )
    .await
}

fn updated() -> (Status, serde_json::Value) {
    (
        Status::Ok,
        json!({"success": true, "message": "Authentication updated successfully"}),
    )
}

#[rocket::async_test]
async fn settings_that_match_the_event_are_not_sent_to_keycloak() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    // Without a presentation, both are enabled.
    let event = rows::event(&services.hasura).await;

    assert_eq!(
        json(set(&client, &event, "enabled", "enabled").await).await,
        updated()
    );
    // Blank settings are left as they are.
    assert_eq!(json(set(&client, &event, " ", "").await).await, updated());
    assert!(services.identity.updates().is_empty());
}

#[rocket::async_test]
async fn changed_settings_are_sent_to_keycloak() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    assert_eq!(
        json(set(&client, &event, "disabled", "disabled").await).await,
        updated()
    );
    assert_eq!(
        services.identity.updates(),
        vec![
            AuthenticationUpdate::Enrollment(false),
            AuthenticationUpdate::Otp("DISABLED".into()),
        ]
    );
}

#[rocket::async_test]
async fn enabling_otp_makes_it_required() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event
        .present(
            &services.hasura,
            json!({"enrollment": "disabled", "otp": "disabled"}),
        )
        .await;

    assert_eq!(
        json(set(&client, &event, "enabled", "enabled").await).await,
        updated()
    );
    assert_eq!(
        services.identity.updates(),
        vec![
            AuthenticationUpdate::Enrollment(true),
            AuthenticationUpdate::Otp("REQUIRED".into()),
        ]
    );
}

#[rocket::async_test]
async fn a_setting_keycloak_refuses_is_a_server_error() {
    let services =
        Services::on_test_database()
            .await
            .with_identity(LocalIdentityAdmin {
                refuses_updates: true,
                ..Default::default()
            });
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, message) =
        text(set(&client, &event, "disabled", "enabled").await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(
        message.starts_with("Error updating enrollment: "),
        "{message}"
    );
    let (status, message) =
        text(set(&client, &event, "enabled", "disabled").await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(message.starts_with("Error updating OTP: "), "{message}");
}

#[rocket::async_test]
async fn the_settings_of_an_unknown_election_event_are_a_server_error() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let unknown = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };

    let (status, _) =
        text(set(&client, &unknown, "disabled", "disabled").await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(services.identity.updates().is_empty());

    let services = Services::without_database();
    let client = services.client().await;
    let (status, message) =
        text(set(&client, &unknown, "disabled", "disabled").await).await;
    assert_eq!(status, Status::InternalServerError);
    assert!(message.contains("Connection refused"), "{message}");
}
