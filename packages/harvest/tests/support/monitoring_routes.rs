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

/// Counts the event the way the Windmill job does, from voter rows put
/// straight into its projection.
async fn count(services: &Services, event: &Event) {
    use sequent_core::monitoring::config::ConfigKind;
    use windmill::services::monitoring::config_store::{
        get_live_config, EventRef,
    };
    use windmill::services::monitoring::snapshot::count_event;
    let event = EventRef {
        tenant_id: Uuid::parse_str(&event.tenant_id).unwrap(),
        election_event_id: Uuid::parse_str(&event.election_event_id).unwrap(),
    };
    let mut client = services.hasura.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    let live = get_live_config(&transaction, event).await.unwrap().unwrap();
    transaction.commit().await.unwrap();
    let settings_revision = live
        .documents
        .iter()
        .find(|document| document.kind == ConfigKind::Settings)
        .unwrap()
        .revision;
    count_event(
        &mut client,
        event,
        live.assembled.set.settings.as_ref().unwrap(),
        settings_revision,
        live.generation,
    )
    .await
    .unwrap();
}

async fn post_in(
    services: &Services,
    event: &Event,
    name: &str,
    region: &str,
    label: &str,
) -> String {
    let id = labelled_election(services, event, label).await;
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.election
         SET presentation = $2, annotations = $3 WHERE id = $1",
        &[
            &Uuid::parse_str(&id).unwrap(),
            &json!({"i18n": {"en": {"name": name}}}),
            &json!({"miru:geographical-region": region}),
        ],
    )
    .await;
    id
}

async fn voter_of(
    services: &Services,
    event: &Event,
    election: &str,
    voter: &str,
    region: &str,
    voted: bool,
) {
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.monitoring_voter
             (tenant_id, election_event_id, election_id, voter_id, region,
              country, dims, first_voted_at, attributes_hash, settings_revision)
         VALUES ($1, $2, $3, $4, $5, 'Philippines', '{}',
                 CASE WHEN $6 THEN now() - interval '1 hour' END, 'h', 1)",
        &[
            &Uuid::parse_str(&event.tenant_id).unwrap(),
            &Uuid::parse_str(&event.election_event_id).unwrap(),
            &Uuid::parse_str(election).unwrap(),
            &voter,
            &region,
            &voted,
        ],
    )
    .await;
}

/// The value of `column` in the first row of a drawn widget's table.
fn first(body: &Value, column: &str) -> Value {
    let columns = body["table"]["columns"].as_array().unwrap();
    let at = columns
        .iter()
        .position(|c| c["name"] == column || c == column)
        .unwrap_or_else(|| panic!("no {column} in {body}"));
    body["table"]["rows"][0][at].clone()
}

#[rocket::async_test]
async fn the_figures_windmill_counted_are_drawn_for_exactly_the_viewers_elections(
) {
    let services = Services::on_test_database().await.with_live_snapshots();
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let luzon = post_in(&services, &event, "Manila", "Luzon", "north").await;
    let mindanao =
        post_in(&services, &event, "Davao", "Mindanao", "south").await;
    configure(&client, &event).await;
    voter_of(&services, &event, &luzon, "v1", "Luzon", true).await;
    voter_of(&services, &event, &luzon, "v2", "Luzon", false).await;
    voter_of(&services, &event, &mindanao, "v3", "Mindanao", true).await;
    count(&services, &event).await;

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
    assert_eq!(first(&body, "registered"), 3, "{body}");
    assert_eq!(first(&body, "voted"), 2, "{body}");

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
    let regions: Vec<_> = body["scope_options"]["regions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|region| region["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(regions, ["Luzon", "Mindanao"], "{body}");
    // The Day selector's options: the days with votes, in Manila time.
    let days = body["event_days"].as_array().expect("event_days");
    assert_eq!(days.len(), 1, "{body}");
    assert_eq!(days[0].as_str().unwrap().len(), "2026-05-04".len());

    let (status, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-summary",
        json!({"scope": {"region": "Visayas"}}),
    )
    .await;
    assert_eq!(status, Status::Forbidden, "{body}");

    // A viewer of the north only: their set is not counted yet, so it is
    // asked for, and the next pass counts it.
    let northern = viewer(&event).permission_labels(&["north"]);
    let (_, body) =
        render(&client, &northern, &event, "turnout-summary", json!({})).await;
    assert_eq!(body["state"], "SCOPE_PENDING", "{body}");
    count(&services, &event).await;
    let (status, body) =
        render(&client, &northern, &event, "turnout-summary", json!({})).await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "RENDERED", "{body}");
    assert_eq!(first(&body, "registered"), 2, "{body}");
    assert_eq!(first(&body, "voted"), 1, "{body}");
}
