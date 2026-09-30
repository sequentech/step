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

async fn configure(client: &Client, event: &Event) {
    let configurator = Claims::new(&event.tenant_id, "configurator").roles([
        Permissions::MONITORING_CONFIGURE,
        Permissions::ELECTION_EVENT_WRITE,
    ]);
    let (status, body) = json(
        post(
            client,
            "/monitoring/reset-to-preset",
            &configurator,
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
async fn the_dashboard_is_looked_up_at_the_runs_configuration() {
    let services = Services::on_test_database()
        .await
        .with_monitoring_snapshots(MemorySnapshots::at(7, ScopeRead::Empty));
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    configure(&client, &event).await;
    // The run was counted under a configuration the event no longer has,
    // with the settings it still has. The dashboard draws such a run with
    // the live configuration; an export always reads the run's own, so the
    // same revision always exports the same file.
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
    assert_eq!(status, Status::NotFound, "{body}");
    assert_eq!(body["extensions"]["code"], "MONITORING_NOT_FOUND");
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
