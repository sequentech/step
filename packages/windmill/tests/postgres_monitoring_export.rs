// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exports read the revision the dashboard showed, with the configuration it
//! was counted under, and only the figures of the viewer's elections; a
//! pruned revision exports nothing.

#[path = "support/schema.rs"]
mod schema;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use deadpool_postgres::{Client, Transaction};
use indexmap::IndexMap;
use sequent_core::monitoring::presets;
use sequent_core::monitoring::revision::DashboardMode;
use serde_json::{json, Value};
use tokio_postgres::IsolationLevel;
use uuid::Uuid;
use windmill::services::monitoring::config_store::{
    reset_to_preset, Author, EventRef, MonitoringConfigAudit, RecordedChange, ResetOutcome,
};
use windmill::services::monitoring::export::{
    build_file, collect_export, export_table, ExportData, MonitoringExportError,
    MonitoringExportFormat, MonitoringExportRequest,
};
use windmill::services::monitoring::snapshot::{count_event, request_election_set, PassOutcome};

struct NoAudit;

#[async_trait]
impl MonitoringConfigAudit for NoAudit {
    async fn prepare(&self, _: &mut Client, _: EventRef, _: &Author) -> anyhow::Result<()> {
        Ok(())
    }
    async fn record(&self, _: &Transaction<'_>, _: &RecordedChange) -> anyhow::Result<()> {
        Ok(())
    }
}

struct Seeded {
    event: EventRef,
    madrid: Uuid,
    tokyo: Uuid,
    revision: i64,
}

/// An event on the COMELEC preset with two Posts and a voter in each, the
/// Madrid-only set asked for, and one pass counted.
async fn seed(client: &mut Client) -> Seeded {
    let event = EventRef {
        tenant_id: Uuid::new_v4(),
        election_event_id: Uuid::new_v4(),
    };
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&event.tenant_id, &format!("tenant-{}", event.tenant_id)],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event.election_event_id, &event.tenant_id],
    )
    .await
    .unwrap();
    let mut ids = Vec::new();
    for (name, region) in [("Madrid", "Europe"), ("Tokyo", "Asia")] {
        let id = Uuid::new_v4();
        tx.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, presentation, annotations, status)
             VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &id,
                &event.tenant_id,
                &event.election_event_id,
                &json!({"i18n": {"en": {"name": name}}}),
                &json!({"miru:geographical-region": region}),
                &json!({"voting_status": "OPEN"}),
            ],
        )
        .await
        .unwrap();
        ids.push(id);
    }
    tx.commit().await.unwrap();
    let (madrid, tokyo) = (ids[0], ids[1]);

    let author = Author {
        id: "admin-1".to_string(),
        name: None,
    };
    let ResetOutcome::Reset { generation, .. } = reset_to_preset(
        client,
        &NoAudit,
        event,
        &author,
        "comelec",
        DashboardMode::Configured,
    )
    .await
    .unwrap() else {
        panic!("the reset writes the preset");
    };

    for (voter, election, region) in [("ana", madrid, "Europe"), ("ben", tokyo, "Asia")] {
        client
            .execute(
                "INSERT INTO sequent_backend.monitoring_voter
                     (tenant_id, election_event_id, election_id, voter_id, region, country, dims,
                      first_voted_at, attributes_hash, settings_revision)
                 VALUES ($1, $2, $3, $4, $5, 'Spain', '{}', now() - interval '1 hour', 'h', 1)",
                &[
                    &event.tenant_id,
                    &event.election_event_id,
                    &election,
                    &voter,
                    &region,
                ],
            )
            .await
            .unwrap();
    }
    let tx = client.transaction().await.unwrap();
    request_election_set(&tx, event, &[madrid]).await.unwrap();
    tx.commit().await.unwrap();

    let settings = presets::load("comelec")
        .unwrap()
        .unwrap()
        .set
        .settings
        .clone()
        .unwrap();
    let PassOutcome::Completed { revision, .. } =
        count_event(client, event, &settings, 1, generation)
            .await
            .unwrap()
    else {
        panic!("the pass completes");
    };
    Seeded {
        event,
        madrid,
        tokyo,
        revision,
    }
}

fn request(seeded: &Seeded, elections: &[Uuid], revision: i64) -> MonitoringExportRequest {
    MonitoringExportRequest {
        tenant_id: seeded.event.tenant_id.to_string(),
        election_event_id: seeded.event.election_event_id.to_string(),
        dashboard_id: "req-0259".to_string(),
        widget_id: Some("turnout-by-post".to_string()),
        election_ids: elections.iter().map(Uuid::to_string).collect(),
        pinned_post: None,
        scope: Default::default(),
        selector_values: IndexMap::new(),
        snapshot_revision: revision,
        format: MonitoringExportFormat::Csv,
        from: None,
        to: None,
        document_id: Uuid::new_v4().to_string(),
    }
}

async fn export(
    client: &mut Client,
    request: &MonitoringExportRequest,
) -> Result<ExportData, MonitoringExportError> {
    let tx = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await
        .unwrap();
    collect_export(&tx, request).await
}

/// The Posts a by-Post export has rows of.
fn posts(data: &ExportData) -> Vec<String> {
    let table = export_table(data);
    let index = table
        .columns
        .iter()
        .position(|(name, _)| name == "group_key")
        .expect("a by-group export names each group");
    table
        .rows
        .iter()
        .filter_map(|row| row[index].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn an_export_holds_the_viewers_elections_only_and_a_pruned_revision_none() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let seeded = seed(&mut client).await;

    let everything = export(
        &mut client,
        &request(&seeded, &[seeded.madrid, seeded.tokyo], seeded.revision),
    )
    .await
    .unwrap();
    let mut both = posts(&everything);
    both.sort();
    let mut expected = vec![seeded.madrid.to_string(), seeded.tokyo.to_string()];
    expected.sort();
    assert_eq!(both, expected);

    let madrid_only = export(
        &mut client,
        &request(&seeded, &[seeded.madrid], seeded.revision),
    )
    .await
    .unwrap();
    assert_eq!(posts(&madrid_only), vec![seeded.madrid.to_string()]);
    let csv =
        String::from_utf8(build_file(&madrid_only, MonitoringExportFormat::Csv).unwrap()).unwrap();
    assert!(csv.contains("Madrid"));
    assert!(!csv.contains("Tokyo"), "Tokyo is not the viewer's: {csv}");
    assert!(!csv.contains(&seeded.tokyo.to_string()));

    // A set the pass did not count is pending, never another set's figures.
    let tokyo_only = export(
        &mut client,
        &request(&seeded, &[seeded.tokyo], seeded.revision),
    )
    .await;
    assert!(
        matches!(tokyo_only, Err(MonitoringExportError::ScopePending { .. })),
        "{tokyo_only:?}"
    );

    // A Post outside the viewer's elections is refused.
    let mut foreign_post = request(&seeded, &[seeded.madrid], seeded.revision);
    foreign_post.pinned_post = Some(seeded.tokyo.to_string());
    assert!(matches!(
        export(&mut client, &foreign_post).await,
        Err(MonitoringExportError::ForbiddenScope(_))
    ));

    let pruned = export(
        &mut client,
        &request(&seeded, &[seeded.madrid], seeded.revision + 100),
    )
    .await;
    assert!(
        matches!(pruned, Err(MonitoringExportError::SnapshotPruned { revision }) if revision == seeded.revision + 100),
        "{pruned:?}"
    );

    // The whole dashboard, as SQL: every widget of it.
    let mut dashboard = request(&seeded, &[seeded.madrid, seeded.tokyo], seeded.revision);
    dashboard.widget_id = None;
    dashboard.format = MonitoringExportFormat::Sql;
    let data = export(&mut client, &dashboard).await.unwrap();
    let widgets: Vec<&str> = data.widgets.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        widgets,
        vec![
            "turnout-summary",
            "turnout-by-group",
            "turnout-by-post",
            "turnout-by-country"
        ]
    );
    let sql = String::from_utf8(build_file(&data, MonitoringExportFormat::Sql).unwrap()).unwrap();
    assert!(sql.starts_with("-- Monitoring export (PostgreSQL)\n"));
    assert!(sql.contains("CREATE TABLE \"monitoring_export\""));

    // The SQL runs, and holds what the table holds.
    client.batch_execute(&sql).await.unwrap();
    let rows: i64 = client
        .query_one("SELECT count(*) FROM monitoring_export", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(rows as usize, export_table(&data).rows.len());
    let registered: Value = client
        .query_one(
            "SELECT to_jsonb(registered) FROM monitoring_export
             WHERE widget_id = 'turnout-summary' AND query = 'totals'",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(registered, json!(2));
    client
        .batch_execute("DROP TABLE monitoring_export")
        .await
        .unwrap();

    // Series buckets outside [from, to) are left out; totals are kept.
    let mut activity = request(&seeded, &[seeded.madrid, seeded.tokyo], seeded.revision);
    activity.dashboard_id = "overview".to_string();
    activity.widget_id = Some("voting-activity".to_string());
    let series_rows = |data: &ExportData| export_table(data).rows.len();
    let all = export(&mut client, &activity).await.unwrap();
    assert!(
        series_rows(&all) > 0,
        "the votes an hour ago are in a bucket"
    );
    activity.from = Some(Utc::now() + Duration::days(1));
    activity.to = Some(Utc::now() + Duration::days(2));
    let later = export(&mut client, &activity).await.unwrap();
    assert_eq!(series_rows(&later), 0);
    activity.to = activity.from;
    assert!(matches!(
        export(&mut client, &activity).await,
        Err(MonitoringExportError::Invalid(_))
    ));
}
