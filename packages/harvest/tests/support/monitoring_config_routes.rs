// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Changing an event's monitoring configuration through the routes, on the
//! migrated test database with Windmill's configuration store; the renderer
//! and the electoral log as fakes.

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use sequent_core::monitoring::problem::{Code, Problem};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

fn configurator(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, "configurator")
        .username("configurator")
        .roles([
            Permissions::MONITORING_VIEW,
            Permissions::MONITORING_CONFIGURE,
            Permissions::ELECTION_EVENT_WRITE,
        ])
}

async fn call(
    client: &Client,
    path: &'static str,
    event: &Event,
    extra: Value,
) -> (Status, Value) {
    let mut body = json!({"election_event_id": event.election_event_id});
    for (key, value) in extra.as_object().cloned().unwrap_or_default() {
        body[key] = value;
    }
    json(post(client, path, &configurator(event), &body).await).await
}

async fn configure(client: &Client, event: &Event) {
    let (status, body) = call(
        client,
        "/monitoring/reset-to-preset",
        event,
        json!({"preset_id": "comelec"}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
}

/// The widget's live YAML and revision.
async fn widget(client: &Client, event: &Event, key: &str) -> (String, i64) {
    let (status, body) = call(
        client,
        "/monitoring/get-config",
        event,
        json!({"kind": "widget", "key": key}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    (
        body["yaml"].as_str().unwrap().to_string(),
        body["revision"].as_i64().unwrap(),
    )
}

async fn save_widget(
    client: &Client,
    event: &Event,
    key: &str,
    yaml: &str,
    expected: i64,
) -> (Status, Value) {
    call(
        client,
        "/monitoring/save-config",
        event,
        json!({
            "kind": "widget",
            "key": key,
            "yaml": yaml,
            "expected_revision": expected,
            "change": "UPSERT",
        }),
    )
    .await
}

#[rocket::async_test]
async fn a_save_from_a_stale_revision_is_refused_naming_who_saved_since() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    configure(&client, &event).await;
    let (yaml, revision) = widget(&client, &event, "turnout-summary").await;
    let renamed = yaml.replace("title: Voter turnout", "title: Turnout");
    assert_ne!(renamed, yaml);

    let (status, body) =
        save_widget(&client, &event, "turnout-summary", &renamed, revision)
            .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["revision"], revision + 1);
    assert_eq!(widget(&client, &event, "turnout-summary").await.0, renamed);

    let (status, body) = save_widget(
        &client,
        &event,
        "turnout-summary",
        &yaml.replace("title: Voter turnout", "title: Votes"),
        revision,
    )
    .await;
    assert_eq!(status, Status::Conflict, "{body}");
    let extensions = &body["extensions"];
    assert_eq!(extensions["code"], "MONITORING_CONFLICT");
    assert_eq!(extensions["current_revision"], revision + 1);
    assert_eq!(extensions["author"]["id"], "configurator", "{body}");
    assert!(extensions["time"].is_string(), "{body}");
}

#[rocket::async_test]
async fn an_invalid_document_leaves_the_live_revision_unchanged() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    configure(&client, &event).await;
    let (yaml, revision) = widget(&client, &event, "turnout-summary").await;

    let (status, body) = save_widget(
        &client,
        &event,
        "turnout-summary",
        &yaml.replace("source: voter_turnout", "source: no_such_source"),
        revision,
    )
    .await;
    assert_eq!(status, Status::UnprocessableEntity, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_INVALID");
    assert!(
        !body["extensions"]["problems"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{body}"
    );
    assert_eq!(
        widget(&client, &event, "turnout-summary").await,
        (yaml, revision)
    );
}

#[rocket::async_test]
async fn an_engine_error_blocks_a_save_and_an_engine_warning_does_not() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    configure(&client, &event).await;
    let (yaml, revision) = widget(&client, &event, "turnout-summary").await;
    let renamed = yaml.replace("title: Voter turnout", "title: Turnout");
    *services.monitoring_renderer.problems.lock().unwrap() =
        vec![Problem::error(
            Code::ChartSchema,
            "charts.voted",
            "No such type.",
        )
        .with_engine_code("ERR-CHART-TYPE")];

    let (status, body) =
        save_widget(&client, &event, "turnout-summary", &renamed, revision)
            .await;
    assert_eq!(status, Status::UnprocessableEntity, "{body}");
    assert_eq!(
        body["extensions"]["problems"][0]["engine_code"], "ERR-CHART-TYPE",
        "{body}"
    );
    assert_eq!(
        body["extensions"]["problems"][0]["path"], "chart.charts.voted",
        "{body}"
    );
    assert_eq!(widget(&client, &event, "turnout-summary").await.1, revision);

    *services.monitoring_renderer.problems.lock().unwrap() =
        vec![Problem::warning(
            Code::ChartSchema,
            "charts",
            "A pinned colour is unseen.",
        )
        .with_engine_code("WARN-CATEGORY-COLOR-PIN-UNSEEN")];
    let (status, body) =
        save_widget(&client, &event, "turnout-summary", &renamed, revision)
            .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["revision"], revision + 1);
    assert_eq!(body["warnings"][0]["severity"], "WARNING", "{body}");
    assert_eq!(
        services.monitoring_audit.changes().len(),
        2,
        "the reset and the save"
    );
}

#[rocket::async_test]
async fn changes_need_election_event_write_and_an_event_not_locked_down() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    configure(&client, &event).await;
    let (yaml, revision) = widget(&client, &event, "turnout-summary").await;

    let reader = Claims::new(&event.tenant_id, "configurator")
        .roles([Permissions::MONITORING_CONFIGURE]);
    let (status, body) = json(
        post(
            &client,
            "/monitoring/save-config",
            &reader,
            &json!({
                "election_event_id": event.election_event_id,
                "kind": "widget",
                "key": "turnout-summary",
                "yaml": yaml,
                "expected_revision": revision,
                "change": "UPSERT",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Unauthorized, "{body}");

    event
        .present(&services.hasura, json!({"locked_down": "locked-down"}))
        .await;
    let (status, body) =
        save_widget(&client, &event, "turnout-summary", &yaml, revision).await;
    assert_eq!(status, Status::Forbidden, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_LOCKED_DOWN");
    let (status, body) = call(
        &client,
        "/monitoring/reset-to-preset",
        &event,
        json!({"preset_id": "campus"}),
    )
    .await;
    assert_eq!(status, Status::Forbidden, "{body}");
}

#[rocket::async_test]
async fn a_checked_widget_is_previewed_on_the_sample_figures() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    configure(&client, &event).await;
    let (yaml, _) = widget(&client, &event, "turnout-summary").await;

    let (status, body) = call(
        &client,
        "/monitoring/validate-config",
        &event,
        json!({"kind": "widget", "key": "turnout-summary", "yaml": yaml}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["result"], "VALID", "{body}");
    assert_eq!(body["preview"]["state"], "RENDERED", "{body}");

    let (status, body) = call(
        &client,
        "/monitoring/validate-config",
        &event,
        json!({"kind": "widget", "key": "turnout-summary", "yaml": "id: [".to_string()}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["result"], "INVALID", "{body}");
    assert_eq!(body["problems"][0]["severity"], "ERROR", "{body}");
}

#[rocket::async_test]
async fn the_configuration_lists_its_documents_presets_and_mode() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, body) =
        call(&client, "/monitoring/list-presets", &event, json!({})).await;
    assert_eq!(status, Status::Ok, "{body}");
    let presets: Vec<_> = body["presets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|preset| preset["id"].as_str().unwrap().to_string())
        .collect();
    assert!(presets.contains(&"comelec".to_string()), "{body}");

    configure(&client, &event).await;
    let (status, body) =
        call(&client, "/monitoring/list-config", &event, json!({})).await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["mode"], "CONFIGURED");
    assert_eq!(body["preset"]["id"], "comelec");
    let documents = body["documents"].as_array().unwrap();
    assert!(documents.iter().any(|document| document["kind"] == "widget"
        && document["key"] == "turnout-summary"
        && document["author"]["id"] == "configurator"));

    let (status, body) = call(
        &client,
        "/monitoring/set-mode",
        &event,
        json!({"mode": "LEGACY"}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let (_, body) =
        call(&client, "/monitoring/list-dashboards", &event, json!({})).await;
    assert_eq!(body["mode"], "LEGACY", "{body}");
}

/// With a running renderer service (`HARVEST_MONITORING_RENDERER_URL` and
/// `_TOKEN`): a preset widget checks, saves and draws through the real
/// engine, and a chart the engine refuses is not saved.
#[rocket::async_test]
#[ignore = "needs a running renderer service"]
async fn a_widget_checks_saves_and_draws_through_the_real_renderer() {
    use crate::adapters::memory::monitoring_snapshots::MemorySnapshots;
    use crate::adapters::monitoring_renderer::HttpMonitoringRenderer;
    use std::sync::Arc;
    use windmill::services::monitoring::snapshot::ScopeRead;

    let services = Services::on_test_database()
        .await
        .with_live_renderer(Arc::new(HttpMonitoringRenderer::from_env()))
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;
    let (yaml, revision) = widget(&client, &event, "turnout-by-group").await;

    let (status, body) = call(
        &client,
        "/monitoring/validate-config",
        &event,
        json!({"kind": "widget", "key": "turnout-by-group", "yaml": yaml}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["result"], "VALID", "{body}");
    assert_eq!(body["preview"]["state"], "RENDERED", "{body}");

    let renamed = yaml.replacen("title: ", "title: Live ", 1);
    let (status, body) =
        save_widget(&client, &event, "turnout-by-group", &renamed, revision)
            .await;
    assert_eq!(status, Status::Ok, "{body}");

    // A chart type the engine does not know passes the policy but not dbt
    // Charts, so the live revision stays.
    let broken = renamed.replacen("type: bar", "type: no_such_chart", 1);
    assert_ne!(broken, renamed, "the widget has a bar chart");
    let (status, body) =
        save_widget(&client, &event, "turnout-by-group", &broken, revision + 1)
            .await;
    assert_eq!(status, Status::UnprocessableEntity, "{body}");
    assert!(
        body["extensions"]["problems"][0]["engine_code"]
            .as_str()
            .is_some_and(|code| code.starts_with("ERR-")),
        "{body}"
    );
    assert_eq!(
        widget(&client, &event, "turnout-by-group").await.1,
        revision + 1
    );

    let viewer = Claims::new(&event.tenant_id, "monitor")
        .roles([Permissions::MONITORING_VIEW]);
    let (status, body) = json(
        post(
            &client,
            "/monitoring/render-widget",
            &viewer,
            &json!({
                "election_event_id": event.election_event_id,
                "dashboard_id": "overview",
                "widget_id": "turnout-summary",
                "color_scheme": "DARK",
                "width": 900,
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["state"], "RENDERED", "{body}");
    let svg = body["svg"].as_str().unwrap();
    assert!(svg.starts_with("<svg") && svg.contains("<text"), "{svg}");
}
