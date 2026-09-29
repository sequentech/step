// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The monitoring dashboards as a viewer sees them: the event's
//! configuration and elections on the migrated test database, the snapshot
//! rows and the renderer as fakes.

use crate::adapters::memory::monitoring_snapshots::MemorySnapshots;
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::services::monitoring::snapshot::ScopeRead;

pub fn viewer(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, "monitor")
        .username("monitor")
        .roles([Permissions::MONITORING_VIEW])
}

pub fn configurator(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, "configurator")
        .username("configurator")
        .roles([
            Permissions::MONITORING_VIEW,
            Permissions::MONITORING_CONFIGURE,
            Permissions::ELECTION_EVENT_WRITE,
        ])
}

/// Resets the event to the COMELEC preset through the route.
pub async fn configure(client: &Client, event: &Event) {
    let (status, body) = json(
        post(
            client,
            "/monitoring/reset-to-preset",
            &configurator(event),
            &json!({
                "election_event_id": event.election_event_id,
                "preset_id": "comelec",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
}

async fn labelled_election(
    services: &Services,
    event: &Event,
    label: &str,
) -> String {
    let id = event.election(&services.hasura).await;
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.election SET permission_label = $2 WHERE id = $1",
        &[&Uuid::parse_str(&id).unwrap(), &label],
    )
    .await;
    id
}

async fn render(
    client: &Client,
    claims: &Claims,
    event: &Event,
    widget_id: &str,
    extra: Value,
) -> (Status, Value) {
    let mut body = json!({
        "election_event_id": event.election_event_id,
        "dashboard_id": "overview",
        "widget_id": widget_id,
    });
    for (key, value) in extra.as_object().cloned().unwrap_or_default() {
        body[key] = value;
    }
    json(post(client, "/monitoring/render-widget", claims, &body).await).await
}

#[rocket::async_test]
async fn an_event_never_configured_shows_the_legacy_dashboard() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, body) = json(
        post(
            &client,
            "/monitoring/list-dashboards",
            &viewer(&event),
            &json!({"election_event_id": event.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        body,
        json!({"mode": "LEGACY", "dashboards": [], "snapshot": null})
    );

    let (status, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-summary",
        json!({}),
    )
    .await;
    assert_eq!(status, Status::NotFound, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_NOT_FOUND");
}

#[rocket::async_test]
async fn a_configured_event_lists_its_dashboards_and_draws_a_widget_once() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;

    let (status, body) = json(
        post(
            &client,
            "/monitoring/list-dashboards",
            &viewer(&event),
            &json!({"election_event_id": event.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["mode"], "CONFIGURED");
    assert_eq!(body["dashboards"][0]["id"], "overview", "{body}");
    assert_eq!(body["snapshot"]["revision"], 7);

    let (status, body) = json(
        post(
            &client,
            "/monitoring/get-dashboard",
            &viewer(&event),
            &json!({
                "election_event_id": event.election_event_id,
                "dashboard_id": "overview",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        body["widgets"].as_object().map(|widgets| widgets.len()),
        Some(7),
        "{body}"
    );

    for _ in 0..2 {
        let (status, body) = render(
            &client,
            &viewer(&event),
            &event,
            "turnout-summary",
            json!({}),
        )
        .await;
        assert_eq!(status, Status::Ok, "{body}");
        assert_eq!(body["state"], "RENDERED", "{body}");
        assert!(body["svg"].as_str().unwrap().starts_with("<svg"), "{body}");
        assert!(body["table"]["columns"].is_array(), "{body}");
        assert_eq!(body["snapshot_revision"], 7);
    }
    assert_eq!(services.monitoring_renderer.renders(), 1);
}

#[rocket::async_test]
async fn a_post_outside_the_viewers_labels_is_refused_before_any_snapshot_is_read(
) {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let north = labelled_election(&services, &event, "north").await;
    let south = labelled_election(&services, &event, "south").await;
    configure(&client, &event).await;
    let northern = viewer(&event).permission_labels(&["north"]);

    let (status, body) = render(
        &client,
        &northern,
        &event,
        "turnout-summary",
        json!({"scope": {"post": south}}),
    )
    .await;
    assert_eq!(status, Status::Forbidden, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_FORBIDDEN_SCOPE");

    let (status, body) = json(
        post(
            &client,
            "/monitoring/list-dashboards",
            &northern,
            &json!({
                "election_event_id": event.election_event_id,
                "election_id": south,
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Forbidden, "{body}");
    assert_eq!(services.monitoring_snapshots.calls(), Vec::<String>::new());

    let (status, body) = render(
        &client,
        &northern,
        &event,
        "turnout-summary",
        json!({"scope": {"post": north}}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "RENDERED", "{body}");
}

#[rocket::async_test]
async fn a_widget_whose_source_has_no_producer_is_not_connected_and_not_drawn()
{
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    configure(&client, &event).await;

    let (status, body) = render(
        &client,
        &viewer(&event),
        &event,
        "helpdesk-issues",
        json!({}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "NOT_CONNECTED", "{body}");
    assert!(body.get("svg").is_none(), "{body}");
    assert_eq!(services.monitoring_renderer.renders(), 0);
    assert!(
        !services
            .monitoring_snapshots
            .calls()
            .iter()
            .any(|call| call.starts_with("read_scope")),
        "{:?}",
        services.monitoring_snapshots.calls()
    );
}

#[rocket::async_test]
async fn without_a_snapshot_a_widget_says_so() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    configure(&client, &event).await;

    let (status, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-summary",
        json!({}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "NO_SNAPSHOT", "{body}");
    assert_eq!(services.monitoring_renderer.renders(), 0);
}

#[rocket::async_test]
async fn an_uncounted_set_of_elections_is_pending_and_requested() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(
            7,
            ScopeRead::NotCounted,
        ));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let north = labelled_election(&services, &event, "north").await;
    labelled_election(&services, &event, "south").await;
    configure(&client, &event).await;

    let (status, body) = render(
        &client,
        &viewer(&event).permission_labels(&["north"]),
        &event,
        "turnout-summary",
        json!({}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "SCOPE_PENDING", "{body}");
    assert_eq!(
        services.monitoring_snapshots.requested(),
        vec![vec![Uuid::parse_str(&north).unwrap()]]
    );
    assert_eq!(services.monitoring_renderer.renders(), 0);
}
