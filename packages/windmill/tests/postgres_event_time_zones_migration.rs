// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Migration `1791000000100_event_time_zones`: the monitoring settings' zone
//! moves to the event presentation, and back on down.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use sequent_core::monitoring::config::ConfigKind;
use sequent_core::monitoring::presets::{self, SETTINGS_KEY};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use windmill::postgres::monitoring_config::EventRef;
use windmill::services::monitoring::config_store::get_live_config;
use windmill::services::time_zones::event_time_zone;

const UP: &str =
    include_str!("../../../hasura/migrations/backend-db/1791000000100_event_time_zones/up.sql");
const DOWN: &str =
    include_str!("../../../hasura/migrations/backend-db/1791000000100_event_time_zones/down.sql");

fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// The preset's settings as this release ships them.
fn preset_settings(preset: &str) -> &'static str {
    presets::load(preset)
        .unwrap()
        .unwrap()
        .documents
        .into_iter()
        .find(|document| document.kind == ConfigKind::Settings)
        .unwrap()
        .yaml
}

async fn election_event(tx: &Transaction<'_>, tenant: Uuid, presentation: Option<Value>) -> Uuid {
    let event = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol, presentation)
         VALUES ($1, $2, 'RSA256', $3)",
        &[&event, &tenant, &presentation],
    )
    .await
    .unwrap();
    event
}

/// Stores `yaml` as the event's settings, reset to `preset`, the way a reset
/// writes it: generation 1, head and revision 1.
async fn monitoring_settings(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    preset: &str,
    yaml: &str,
) {
    tx.batch_execute(&format!(
        "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
         VALUES ('{tenant}', '{event}');
         UPDATE sequent_backend.monitoring_event
         SET config_generation = 1, dashboard_mode = 'CONFIGURED',
             preset_id = '{preset}', preset_version = 1
         WHERE tenant_id = '{tenant}' AND election_event_id = '{event}';
         INSERT INTO sequent_backend.monitoring_config_head
             (tenant_id, election_event_id, kind, key, revision)
         VALUES ('{tenant}', '{event}', 'settings', '{SETTINGS_KEY}', 1);"
    ))
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_config
             (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
              origin, preset_id, preset_version, config_generation, author_id)
         VALUES ($1, $2, 'settings', $3, 1, 'UPSERT', $4, $5, 'PRESET', $6, 1, 1, 'admin')",
        &[
            &tenant,
            &event,
            &SETTINGS_KEY,
            &yaml,
            &sha256(yaml),
            &preset,
        ],
    )
    .await
    .unwrap();
}

async fn presentation(tx: &Transaction<'_>, event: Uuid) -> Value {
    tx.query_one(
        "SELECT presentation FROM sequent_backend.election_event WHERE id = $1",
        &[&event],
    )
    .await
    .unwrap()
    .get(0)
}

/// The settings head: (revision, yaml, sha256, origin, preset, generation, author).
async fn head(
    tx: &Transaction<'_>,
    event: Uuid,
) -> (i32, String, String, String, Option<String>, i64, String) {
    let row = tx
        .query_one(
            "SELECT c.revision, c.yaml, c.sha256, c.origin, c.preset_id,
                    c.config_generation, c.author_id
             FROM sequent_backend.monitoring_config_head h
             JOIN sequent_backend.monitoring_config c USING
                 (tenant_id, election_event_id, kind, key, revision)
             WHERE h.election_event_id = $1 AND h.kind = 'settings'",
            &[&event],
        )
        .await
        .unwrap();
    (
        row.get(0),
        row.get(1),
        row.get(2),
        row.get(3),
        row.get(4),
        row.get(5),
        row.get(6),
    )
}

/// Runs a migration file in a transaction of its own and commits it, as
/// Hasura does: the checks the tables defer to commit (a revision written is
/// its document's head) then hold for what it wrote.
async fn run(client: &mut deadpool_postgres::Object, sql: &str) {
    let tx = client.transaction().await.unwrap();
    tx.batch_execute(sql).await.unwrap();
    tx.commit().await.unwrap();
}

fn zones(zone: &str) -> Value {
    json!({"configured": [zone], "primary": zone, "logs": "election"})
}

#[tokio::test]
async fn the_monitoring_zone_moves_to_the_event_and_back() {
    // This binary's database holds only this test's rows: the migration
    // reads and writes every event, and its writes are committed.
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();

    // The two configurations: the zone as each preset wrote it, plain and quoted.
    let configurations = [
        ("comelec", "Asia/Manila", "time_zone: Asia/Manila"),
        (
            "campus",
            "Europe/Madrid",
            "time_zone : \"Europe/Madrid\" # the campus",
        ),
    ];
    let mut configured = Vec::new();
    for (preset, zone, line) in configurations {
        let shipped = preset_settings(preset);
        assert!(!shipped.contains("time_zone"), "{preset}");
        let stored = shipped.replacen("scope:", &format!("{line}\nscope:"), 1);
        let event = election_event(&tx, tenant, Some(json!({"css": "body {}"}))).await;
        monitoring_settings(&tx, tenant, event, preset, &stored).await;
        configured.push((event, preset, zone, shipped, stored));
    }
    // An event with no monitoring and no presentation.
    let bare = election_event(&tx, tenant, None).await;
    // An event whose presentation already names its zones keeps them, and
    // its settings lose the zone all the same.
    let kept = election_event(
        &tx,
        tenant,
        Some(json!({"timezones": zones("Atlantic/Canary")})),
    )
    .await;
    let kept_stored =
        preset_settings("campus").replacen("scope:", "time_zone: Asia/Manila\nscope:", 1);
    monitoring_settings(&tx, tenant, kept, "campus", &kept_stored).await;
    // Presentations that aren't objects become one holding the timezones.
    let mut not_objects = Vec::new();
    for presentation in [json!(null), json!([]), json!("x")] {
        not_objects.push(election_event(&tx, tenant, Some(presentation)).await);
    }
    // A zone that isn't a zone name: the line goes, the event gets UTC.
    let unnamed = election_event(&tx, tenant, None).await;
    let unnamed_stored =
        preset_settings("campus").replacen("scope:", "time_zone: Madrid\nscope:", 1);
    monitoring_settings(&tx, tenant, unnamed, "campus", &unnamed_stored).await;
    // A value on the next line isn't rewritten (nor half-stripped).
    let block = election_event(&tx, tenant, None).await;
    let block_stored =
        preset_settings("campus").replacen("scope:", "time_zone:\n  Asia/Manila\nscope:", 1);
    monitoring_settings(&tx, tenant, block, "campus", &block_stored).await;
    tx.commit().await.unwrap();

    run(&mut client, UP).await;
    let tx = client.transaction().await.unwrap();

    for (event, preset, zone, shipped, stored) in &configured {
        assert_eq!(
            presentation(&tx, *event).await,
            json!({"css": "body {}", "timezones": zones(zone)}),
            "{preset}"
        );
        assert_eq!(
            event_time_zone(&tx, tenant, *event).await.unwrap().name(),
            *zone
        );
        let (revision, yaml, digest, origin, preset_id, generation, author) =
            head(&tx, *event).await;
        assert_eq!(revision, 2);
        assert_eq!(
            &yaml, shipped,
            "{preset}: the stored text without its zone line"
        );
        assert_eq!(digest, sha256(shipped));
        assert_eq!(origin, "PRESET");
        assert_eq!(preset_id.as_deref(), Some(*preset));
        assert_eq!(generation, 2);
        assert_eq!(author, "system:1791000000100_event_time_zones");
        let live = get_live_config(
            &tx,
            EventRef {
                tenant_id: tenant,
                election_event_id: *event,
            },
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(live.generation, 2);
        assert!(
            live.assembled.set.settings.is_some(),
            "{preset}: the settings parse"
        );
        // The revision it follows stays as it was saved.
        let first: String = tx
            .query_one(
                "SELECT yaml FROM sequent_backend.monitoring_config
                 WHERE election_event_id = $1 AND revision = 1",
                &[event],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(&first, stored);
    }
    assert_eq!(
        presentation(&tx, bare).await,
        json!({"timezones": zones("UTC")})
    );
    assert_eq!(
        presentation(&tx, kept).await,
        json!({"timezones": zones("Atlantic/Canary")})
    );
    assert_eq!(head(&tx, kept).await.1, preset_settings("campus"));
    for event in &not_objects {
        assert_eq!(
            presentation(&tx, *event).await,
            json!({"timezones": zones("UTC")})
        );
    }
    assert_eq!(
        presentation(&tx, unnamed).await,
        json!({"timezones": zones("UTC")})
    );
    assert_eq!(head(&tx, unnamed).await.1, preset_settings("campus"));
    assert_eq!(
        presentation(&tx, block).await,
        json!({"timezones": zones("UTC")})
    );
    let (revision, yaml, ..) = head(&tx, block).await;
    assert_eq!((revision, yaml.as_str()), (1, block_stored.as_str()));

    tx.rollback().await.unwrap();

    // Running it again changes nothing.
    run(&mut client, UP).await;
    let tx = client.transaction().await.unwrap();
    for (event, ..) in &configured {
        assert_eq!(head(&tx, *event).await.0, 2);
    }
    tx.rollback().await.unwrap();

    run(&mut client, DOWN).await;
    let tx = client.transaction().await.unwrap();
    for (event, preset, zone, _, stored) in &configured {
        let (revision, yaml, digest, _, _, generation, author) = head(&tx, *event).await;
        assert_eq!(revision, 3);
        assert_eq!(
            &yaml, stored,
            "{preset}: the text from before, zone included"
        );
        assert_eq!(digest, sha256(stored));
        assert_eq!(generation, 3);
        assert_eq!(author, "system:1791000000100_event_time_zones:down");
        assert_eq!(
            presentation(&tx, *event).await["timezones"],
            zones(zone),
            "the presentation keeps its zones"
        );
    }
    assert_eq!(head(&tx, kept).await.1, kept_stored);

    tx.rollback().await.unwrap();

    // Down a second time finds nothing it wrote; up again strips again.
    run(&mut client, DOWN).await;
    run(&mut client, UP).await;
    let tx = client.transaction().await.unwrap();
    for (event, _, _, shipped, _) in &configured {
        let (revision, yaml, ..) = head(&tx, *event).await;
        assert_eq!((revision, yaml.as_str()), (4, *shipped));
    }
    tx.rollback().await.unwrap();
}
