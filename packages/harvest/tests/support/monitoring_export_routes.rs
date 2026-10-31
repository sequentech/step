// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Exporting a dashboard's figures: what Harvest checks before it hands the
//! export to Windmill's task.

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

fn viewer(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, "monitor")
        .username("monitor")
        .roles([Permissions::MONITORING_VIEW])
}

fn configurator(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, "configurator").roles([
        Permissions::MONITORING_CONFIGURE,
        Permissions::ELECTION_EVENT_WRITE,
    ])
}

/// Resets the event to the COMELEC preset; the generation it made.
async fn configure(client: &Client, event: &Event) -> i64 {
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
    body["generation"].as_i64().unwrap()
}

/// The `kind` document `key`'s YAML and revision.
async fn document(
    client: &Client,
    event: &Event,
    kind: &str,
    key: &str,
) -> (String, Value) {
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
    (
        body["yaml"].as_str().unwrap().to_string(),
        body["revision"].clone(),
    )
}

/// Saves the `kind` document `key` as `yaml` over `expected` (`null` for a
/// new one); the generation the save made.
async fn save(
    client: &Client,
    event: &Event,
    kind: &str,
    key: &str,
    yaml: &str,
    expected: Value,
) -> i64 {
    let (status, body) = json(
        post(
            client,
            "/monitoring/save-config",
            &configurator(event),
            &json!({
                "election_event_id": event.election_event_id,
                "kind": kind,
                "key": key,
                "yaml": yaml,
                "expected_revision": expected,
                "change": "UPSERT",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    body["generation"].as_i64().unwrap()
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

async fn export(
    client: &Client,
    claims: &Claims,
    event: &Event,
    extra: Value,
) -> (Status, Value) {
    let mut body = json!({
        "election_event_id": event.election_event_id,
        "dashboard_id": "overview",
        "snapshot_revision": 7,
        "format": "CSV",
    });
    for (key, value) in extra.as_object().cloned().unwrap_or_default() {
        body[key] = value;
    }
    json(post(client, "/monitoring/export", claims, &body).await).await
}

#[rocket::async_test]
async fn an_export_sends_the_task_with_the_elections_the_viewer_may_see() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let north = labelled_election(&services, &event, "north").await;
    labelled_election(&services, &event, "south").await;
    configure(&client, &event).await;

    let (status, body) = export(
        &client,
        &viewer(&event).permission_labels(&["north"]),
        &event,
        json!({"widget_id": "turnout-summary", "format": "SQL"}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "export_monitoring_data");
    let request = &sent[0].kwargs["request"];
    assert_eq!(request["election_ids"], json!([north]));
    assert_eq!(request["widget_id"], "turnout-summary");
    assert_eq!(request["format"], "SQL");
    assert_eq!(request["snapshot_revision"], 7);
    assert_eq!(request["document_id"], body["document_id"]);
    assert_eq!(
        services.ledger.tasks()[0].task_type,
        "EXPORT_MONITORING_DATA"
    );
}

#[rocket::async_test]
async fn a_pruned_run_is_gone_and_a_foreign_post_refused_before_anything_is_sent(
) {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    labelled_election(&services, &event, "north").await;
    let south = labelled_election(&services, &event, "south").await;
    configure(&client, &event).await;

    let (status, body) = export(
        &client,
        &viewer(&event),
        &event,
        json!({"snapshot_revision": 3}),
    )
    .await;
    assert_eq!(status, Status::Gone, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_SNAPSHOT_PRUNED");
    // A revision after the live one was never issued, so it is not "no
    // longer kept".
    let (status, body) = export(
        &client,
        &viewer(&event),
        &event,
        json!({"snapshot_revision": 999999}),
    )
    .await;
    assert_eq!(status, Status::NotFound, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_NOT_FOUND");

    let calls = services.monitoring_snapshots.calls().len();
    let (status, body) = export(
        &client,
        &viewer(&event).permission_labels(&["north"]),
        &event,
        json!({"scope": {"post": south}}),
    )
    .await;
    assert_eq!(status, Status::Forbidden, "{body}");
    assert_eq!(services.monitoring_snapshots.calls().len(), calls);

    let (status, body) = export(
        &client,
        &viewer(&event),
        &event,
        json!({"from": "2026-05-04T00:00:00Z", "to": "2026-05-03T00:00:00Z"}),
    )
    .await;
    assert_eq!(status, Status::UnprocessableEntity, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_INVALID");
    // Said as what is wrong, not as a configuration problem.
    assert_eq!(
        body["message"], "The end of the range must come after its start.",
        "{body}"
    );
    assert_eq!(body["extensions"]["problems"][0]["path"], "to", "{body}");
    assert!(services.tasks.sent().is_empty());
    assert!(services.ledger.tasks().is_empty());
}

#[rocket::async_test]
async fn a_range_with_any_offset_and_each_widgets_values_reach_the_task() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;

    let (status, body) = export(
        &client,
        &viewer(&event),
        &event,
        json!({
            "from": "2026-05-04T08:00:00+08:00",
            "to": "2026-05-04T10:30:00Z",
            "widget_selector_values": {"voting-activity": {"grain": "hour"}},
        }),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let request = &services.tasks.sent()[0].kwargs["request"];
    assert_eq!(request["from"], "2026-05-04T00:00:00Z");
    assert_eq!(request["to"], "2026-05-04T10:30:00Z");
    assert_eq!(
        request["widget_selector_values"],
        json!({"voting-activity": {"grain": "hour"}})
    );
}

#[rocket::async_test]
async fn a_time_without_an_offset_is_refused_as_a_bad_request() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;

    for range in [
        json!({"from": "2026-05-04T08:00:00"}),
        json!({"to": "2026-05-04"}),
        json!({"from": "yesterday"}),
    ] {
        let (status, body) =
            export(&client, &viewer(&event), &event, range.clone()).await;
        assert_eq!(status, Status::UnprocessableEntity, "{range}: {body}");
        assert_eq!(body["extensions"]["code"], "MONITORING_BAD_REQUEST");
        assert!(
            body["message"].as_str().unwrap().contains("offset"),
            "{body}"
        );
    }
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn the_dashboard_is_looked_up_in_the_configuration_it_is_drawn_with() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    let generation = configure(&client, &event).await;
    // The run was counted under a configuration the event no longer has,
    // with the settings it still has: the dashboard draws it with the live
    // configuration, and an export of it reads that one too, named in the
    // request so the task reads exactly it.
    services
        .monitoring_snapshots
        .head
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .config_generation = 99;

    let (status, body) =
        export(&client, &viewer(&event), &event, json!({})).await;
    assert_eq!(status, Status::Ok, "{body}");
    let request = &services.tasks.sent()[0].kwargs["request"];
    assert_eq!(request["config_generation"], generation, "{request}");
}

#[rocket::async_test]
async fn a_widget_added_since_the_run_is_exported_without_waiting_for_a_pass() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;

    // Saved after the run was counted, under the same settings: the
    // dashboard draws the new widget from the figures in hand, so its
    // export reads the configuration that has it.
    let (yaml, _) =
        document(&client, &event, "widget", "turnout-summary").await;
    let copy = yaml.replacen("id: turnout-summary", "id: turnout-copy", 1);
    assert_ne!(copy, yaml);
    save(
        &client,
        &event,
        "widget",
        "turnout-copy",
        &copy,
        Value::Null,
    )
    .await;
    let (layout, revision) =
        document(&client, &event, "dashboard", "overview").await;
    let placed = layout.replacen(
        "layout:\n",
        "layout:\n  - {widget: turnout-copy, width: 12}\n",
        1,
    );
    assert_ne!(placed, layout);
    let generation =
        save(&client, &event, "dashboard", "overview", &placed, revision).await;

    let (status, body) = export(
        &client,
        &viewer(&event),
        &event,
        json!({"widget_id": "turnout-copy"}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let request = &services.tasks.sent()[0].kwargs["request"];
    assert_eq!(request["widget_id"], "turnout-copy");
    assert_eq!(request["config_generation"], generation, "{request}");
}

#[rocket::async_test]
async fn a_run_counted_under_a_configuration_no_longer_kept_is_not_exported() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;
    // Counted under other settings, at a generation no longer kept: the
    // dashboard draws it with the live configuration and says so, but a
    // file would mix the live queries with figures counted otherwise.
    {
        let mut head = services.monitoring_snapshots.head.lock().unwrap();
        let head = head.as_mut().expect("a live run");
        head.config_generation = 99;
        head.settings_revision = 0;
    }

    let (status, body) =
        export(&client, &viewer(&event), &event, json!({})).await;
    assert_eq!(status, Status::NotFound, "{body}");
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_widgets_pick_outside_its_options_is_refused() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;

    for picks in [
        json!({"voting-activity": {"grain": "week"}}),
        json!({"voting-activity": {"colour": "red"}}),
    ] {
        let (status, body) = export(
            &client,
            &viewer(&event),
            &event,
            json!({"widget_selector_values": picks}),
        )
        .await;
        assert_eq!(status, Status::UnprocessableEntity, "{picks}: {body}");
        assert_eq!(body["extensions"]["code"], "MONITORING_INVALID");
        assert_eq!(
            body["extensions"]["problems"][0]["path"]
                .as_str()
                .unwrap()
                .split('.')
                .nth(1),
            Some("voting-activity"),
            "{body}"
        );
    }
    assert!(services.tasks.sent().is_empty());
}
