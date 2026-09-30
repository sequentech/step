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
use windmill::services::monitoring::snapshot::{
    LiveSnapshot as SnapshotHead, ScopeRead,
};

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

/// Saves the `kind` document `key` with `from` replaced by `to`, as a
/// configurator; the generation the save made.
async fn resave(
    client: &Client,
    event: &Event,
    kind: &str,
    key: &str,
    from: &str,
    to: &str,
) -> i64 {
    let (status, body) = json(
        post(
            client,
            "/monitoring/get-config",
            &configurator(event),
            &json!({
                "election_event_id": event.election_event_id,
                "kind": kind,
                "key": key,
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let yaml = body["yaml"].as_str().unwrap();
    let changed = yaml.replacen(from, to, 1);
    assert_ne!(changed, yaml);
    let (status, body) = json(
        post(
            client,
            "/monitoring/save-config",
            &configurator(event),
            &json!({
                "election_event_id": event.election_event_id,
                "kind": kind,
                "key": key,
                "yaml": changed,
                "expected_revision": body["revision"],
                "change": "UPSERT",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    body["generation"].as_i64().unwrap()
}

/// Saves the widget `key` with its title and its first chart's label
/// changed: what it shows, not what it reads.
async fn retitle(client: &Client, event: &Event, key: &str) -> i64 {
    resave(client, event, "widget", key, "title: ", "title: Renamed ").await;
    resave(client, event, "widget", key, "label: ", "label: Renamed ").await
}

async fn reset_to(client: &Client, event: &Event, preset_id: &str) {
    let (status, body) = json(
        post(
            client,
            "/monitoring/reset-to-preset",
            &configurator(event),
            &json!({
                "election_event_id": event.election_event_id,
                "preset_id": preset_id,
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
}

async fn settings_revision(client: &Client, event: &Event) -> i32 {
    let (status, body) = json(
        post(
            client,
            "/monitoring/get-config",
            &configurator(event),
            &json!({
                "election_event_id": event.election_event_id,
                "kind": "settings",
                "key": "settings",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    body["revision"].as_i64().unwrap() as i32
}

/// The last board the renderer drew.
fn last_drawn(services: &Services) -> String {
    let boards = services.monitoring_renderer.boards();
    serde_json::to_string(&boards.last().expect("a drawing").board).unwrap()
}

fn set_head(services: &Services, change: impl FnOnce(&mut SnapshotHead)) {
    let mut head = services.monitoring_snapshots.head.lock().unwrap();
    change(head.as_mut().expect("a live run"));
}

#[rocket::async_test]
async fn a_saved_presentation_change_is_drawn_at_once_from_the_figures_already_counted(
) {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;

    let (status, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-summary",
        json!({}),
    )
    .await;
    assert_eq!(body["state"], "RENDERED", "{status} {body}");
    assert_eq!(services.monitoring_renderer.renders(), 1);
    assert!(!last_drawn(&services).contains("Renamed"));

    // Saved after the run was counted, under the same settings: what the
    // run counted is what the widget reads, so it is drawn as saved now,
    // not after the next run.
    retitle(&client, &event, "turnout-summary").await;
    let (_, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-summary",
        json!({}),
    )
    .await;
    assert_eq!(body["state"], "RENDERED", "{body}");
    assert_eq!(body["notices"], json!([]), "{body}");
    assert_eq!(body["snapshot_revision"], 7, "{body}");
    assert_eq!(services.monitoring_renderer.renders(), 2);
    assert!(last_drawn(&services).contains("Renamed"), "{body}");
}

#[rocket::async_test]
async fn after_the_settings_change_a_widget_is_drawn_as_its_run_was_counted_until_the_next_run(
) {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;
    let (_, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-summary",
        json!({}),
    )
    .await;
    assert_eq!(body["state"], "RENDERED", "{body}");
    assert_eq!(services.monitoring_renderer.renders(), 1);

    // The settings decide what a run counts, and only a reset changes
    // them: the run in hand was counted under the old ones, so the widget
    // is drawn as it was then.
    reset_to(&client, &event, "campus").await;
    reset_to(&client, &event, "comelec").await;
    let generation = retitle(&client, &event, "turnout-summary").await;
    let settings_revision = settings_revision(&client, &event).await;
    assert!(settings_revision > 1, "{settings_revision}");
    let (_, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-summary",
        json!({}),
    )
    .await;
    assert_eq!(body["state"], "RENDERED", "{body}");
    assert_eq!(body["notices"], json!([]), "{body}");
    assert_eq!(services.monitoring_renderer.renders(), 1, "{body}");

    // The next run is counted under them.
    set_head(&services, |head| {
        head.config_generation = generation;
        head.settings_revision = settings_revision;
    });
    let (_, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-summary",
        json!({}),
    )
    .await;
    assert_eq!(body["state"], "RENDERED", "{body}");
    assert_eq!(services.monitoring_renderer.renders(), 2);
    assert!(last_drawn(&services).contains("Renamed"), "{body}");
}

#[rocket::async_test]
async fn a_run_whose_configuration_is_gone_is_drawn_with_the_live_one_and_says_so(
) {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;
    // Counted under other settings, at a generation no longer kept.
    set_head(&services, |head| {
        head.config_generation = 99;
        head.settings_revision = 0;
    });

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
    assert_eq!(
        body["notices"],
        json!(["CONFIG_AT_SNAPSHOT_UNAVAILABLE"]),
        "{body}"
    );
}

/// Figures counted before the event had the campus preset's settings: no
/// voter has a faculty.
fn counted_without_faculties() -> ScopeRead {
    use sequent_core::monitoring::sources::DataSourceId;
    use windmill::services::monitoring::snapshot::empty_payload;
    let comelec = sequent_core::monitoring::presets::load("comelec")
        .unwrap()
        .unwrap();
    let payload = empty_payload(
        DataSourceId::VoterTurnout,
        comelec.set.settings.as_ref().unwrap(),
    );
    ScopeRead::Payload {
        sha256_hex: "0".repeat(64),
        text: serde_json::to_string(&payload).unwrap(),
    }
}

#[rocket::async_test]
async fn a_count_the_settings_changed_is_pending_until_the_next_run() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(
            7,
            counted_without_faculties(),
        ));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    let (status, body) = json(
        post(
            &client,
            "/monitoring/reset-to-preset",
            &configurator(&event),
            &json!({
                "election_event_id": event.election_event_id,
                "preset_id": "campus",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let by_faculty = json!({"dashboard_id": "participation"});

    // The run was counted with the settings before these.
    set_head(&services, |head| head.settings_revision = 0);
    let (status, body) = render(
        &client,
        &viewer(&event),
        &event,
        "participation-by-faculty",
        by_faculty.clone(),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "SCOPE_PENDING", "{body}");
    assert_eq!(body["reason"], "SETTINGS_PENDING", "{body}");
    assert!(body.get("table").is_none(), "{body}");
    assert_eq!(services.monitoring_renderer.renders(), 0);

    // Counted with these settings, a count they lack is a configuration
    // problem.
    set_head(&services, |head| head.settings_revision = 1);
    let (status, body) = render(
        &client,
        &viewer(&event),
        &event,
        "participation-by-faculty",
        by_faculty,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "INVALID", "{body}");
    assert_eq!(body["diagnostics"][0]["code"], "not_counted", "{body}");
}

#[rocket::async_test]
async fn a_pinned_run_never_issued_is_not_found_and_one_pruned_is_gone() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;

    for (revision, status, code) in [
        (3, Status::Gone, "MONITORING_SNAPSHOT_PRUNED"),
        (999999, Status::NotFound, "MONITORING_NOT_FOUND"),
    ] {
        let (answered, body) = render(
            &client,
            &viewer(&event),
            &event,
            "turnout-summary",
            json!({"snapshot_revision": revision}),
        )
        .await;
        assert_eq!(answered, status, "{revision}: {body}");
        assert_eq!(body["extensions"]["code"], code, "{revision}: {body}");
    }
}

#[rocket::async_test]
async fn a_widget_outside_the_dashboard_is_drawn_only_for_a_configurator() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;

    // turnout-by-post is a widget of the event, not of the overview.
    let (status, body) = render(
        &client,
        &viewer(&event),
        &event,
        "turnout-by-post",
        json!({}),
    )
    .await;
    assert_eq!(status, Status::NotFound, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_NOT_FOUND");

    let (status, body) = render(
        &client,
        &configurator(&event),
        &event,
        "turnout-by-post",
        json!({}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "RENDERED", "{body}");
}

#[rocket::async_test]
async fn a_drawn_widget_returns_every_querys_table_in_its_order() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
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
    let queries: Vec<&str> = body["tables"]
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| table["query"].as_str().unwrap())
        .collect();
    assert_eq!(queries, ["totals", "voted_reg", "voted_pre", "pre_reg"]);
    // `table` stays the first query's.
    assert_eq!(body["tables"][0]["table"], body["table"], "{body}");
    assert!(body["tables"][1]["table"]["columns"].is_array(), "{body}");

    // The configuration check's preview has them too.
    let (status, body) = json(
        post(
            &client,
            "/monitoring/get-config",
            &configurator(&event),
            &json!({
                "election_event_id": event.election_event_id,
                "kind": "widget",
                "key": "turnout-summary",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let (status, body) = json(
        post(
            &client,
            "/monitoring/validate-config",
            &configurator(&event),
            &json!({
                "election_event_id": event.election_event_id,
                "kind": "widget",
                "key": "turnout-summary",
                "yaml": body["yaml"],
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        body["preview"]["tables"].as_array().map(Vec::len),
        Some(4),
        "{body}"
    );
}

const MONITORING_ROUTES: [&str; 11] = [
    "/monitoring/list-dashboards",
    "/monitoring/get-dashboard",
    "/monitoring/render-widget",
    "/monitoring/validate-config",
    "/monitoring/save-config",
    "/monitoring/reset-to-preset",
    "/monitoring/list-presets",
    "/monitoring/set-mode",
    "/monitoring/list-config",
    "/monitoring/get-config",
    "/monitoring/export",
];

/// POSTs `body` as it is, JSON or not.
async fn post_raw(
    client: &Client,
    path: &'static str,
    claims: Option<&Claims>,
    body: &str,
) -> (Status, Value) {
    use crate::route_services::bearer;
    use rocket::http::ContentType;
    let mut request = client
        .post(path)
        .header(ContentType::JSON)
        .body(body.to_string());
    if let Some(claims) = claims {
        request = request.header(bearer(claims));
    }
    json(request.dispatch().await).await
}

#[rocket::async_test]
async fn a_malformed_request_is_refused_as_every_monitoring_refusal_is() {
    let client = Services::without_database().client().await;
    let claims = Claims::new("00000000-0000-0000-0000-00000000000a", "admin")
        .username("admin")
        .roles([
            Permissions::MONITORING_VIEW,
            Permissions::MONITORING_CONFIGURE,
            Permissions::ELECTION_EVENT_WRITE,
        ]);
    for path in MONITORING_ROUTES {
        let mut malformed = vec![
            ("{not json", Status::BadRequest),
            (r#"{"election_event_id": 7}"#, Status::UnprocessableEntity),
        ];
        // The presets are the same for every event, so it may be left out.
        if path != "/monitoring/list-presets" {
            malformed.push(("{}", Status::UnprocessableEntity));
        }
        for (body, expected) in malformed {
            let (status, answer) =
                post_raw(&client, path, Some(&claims), body).await;
            assert_eq!(status, expected, "{path} {body}: {answer}");
            assert_eq!(
                answer["extensions"]["code"], "MONITORING_BAD_REQUEST",
                "{path} {body}: {answer}"
            );
            assert!(
                answer["message"].as_str().is_some_and(|m| !m.is_empty()),
                "{path} {body}: {answer}"
            );
        }
    }
    // A missing field is named, so the portal can say what was wrong.
    let (_, answer) =
        post_raw(&client, "/monitoring/get-config", Some(&claims), "{}").await;
    assert!(
        answer["message"]
            .as_str()
            .unwrap()
            .contains("election_event_id"),
        "{answer}"
    );

    // What fails before a route runs keeps the same shape.
    let (status, answer) =
        post_raw(&client, "/monitoring/get-config", None, "{}").await;
    assert_eq!(status, Status::Unauthorized, "{answer}");
    assert_eq!(answer["extensions"]["code"], "Unauthorized", "{answer}");
    let (status, answer) =
        post_raw(&client, "/monitoring/no-such-route", Some(&claims), "{}")
            .await;
    assert_eq!(status, Status::NotFound, "{answer}");
    assert_eq!(answer["extensions"]["code"], "MONITORING_NOT_FOUND");

    // Other routes answer as they did.
    let (status, answer) =
        post_raw(&client, "/get-roles", Some(&claims), "{not json").await;
    assert_eq!(status, Status::BadRequest, "{answer}");
    assert_eq!(answer, json!({"message": "Unknown Error"}));
}
