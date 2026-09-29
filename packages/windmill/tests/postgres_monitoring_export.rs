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
use sequent_core::monitoring::scope::ScopeSelection;
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
use windmill::services::monitoring::snapshot::{
    count_event, prune_snapshots, request_election_set, PassOutcome,
};

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
    generation: i64,
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
        generation,
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
        widget_selector_values: IndexMap::new(),
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
async fn an_export_holds_the_viewers_elections_only() {
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

    // A Post, region or country outside the viewer's elections is refused
    // too when it is the dashboard's selection.
    let refused = |scope: ScopeSelection| {
        let mut asked = request(&seeded, &[seeded.madrid], seeded.revision);
        asked.scope = scope;
        asked
    };
    for scope in [
        ScopeSelection {
            post: Some(seeded.tokyo.to_string()),
            ..Default::default()
        },
        ScopeSelection {
            region: Some("Asia".to_string()),
            ..Default::default()
        },
        ScopeSelection {
            country: Some("Atlantis".to_string()),
            ..Default::default()
        },
    ] {
        let outcome = export(&mut client, &refused(scope.clone())).await;
        assert!(
            matches!(outcome, Err(MonitoringExportError::ForbiddenScope(_))),
            "{scope:?}: {outcome:?}"
        );
    }
    let europe = export(
        &mut client,
        &refused(ScopeSelection {
            region: Some("Europe".to_string()),
            ..Default::default()
        }),
    )
    .await;
    assert!(europe.is_ok(), "{europe:?}");

    // The whole dashboard, as SQL: every widget of it.
    let mut dashboard = request(&seeded, &[seeded.madrid, seeded.tokyo], seeded.revision);
    dashboard.widget_id = None;
    dashboard.format = MonitoringExportFormat::Sql;
    let data = export(&mut client, &dashboard).await.unwrap();
    let widgets: Vec<&str> = data
        .widgets
        .iter()
        .map(|widget| widget.id.as_str())
        .collect();
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

    // The SQL runs, and holds what the table holds: inside a transaction
    // that is rolled back, so the test leaves nothing behind.
    let statements: String = sql
        .lines()
        .filter(|line| *line != "BEGIN;" && *line != "COMMIT;")
        .map(|line| format!("{line}\n"))
        .collect();
    let tx = client.transaction().await.unwrap();
    tx.batch_execute(&statements).await.unwrap();
    let rows: i64 = tx
        .query_one("SELECT count(*) FROM monitoring_export", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(rows as usize, export_table(&data).rows.len());
    let registered: Value = tx
        .query_one(
            "SELECT to_jsonb(registered) FROM monitoring_export
             WHERE widget_id = 'turnout-summary' AND query = 'totals'",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(registered, json!(2));
    let revisions: i64 = tx
        .query_one(
            "SELECT count(DISTINCT snapshot_revision) FROM monitoring_export
             WHERE snapshot_revision = $1",
            &[&seeded.revision],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(revisions, 1, "every row names the revision");
    tx.rollback().await.unwrap();
    let left: bool = client
        .query_one("SELECT to_regclass('monitoring_export') IS NOT NULL", &[])
        .await
        .unwrap()
        .get(0);
    assert!(!left);

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

#[tokio::test]
async fn a_revision_pruned_after_it_was_shown_exports_nothing() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let seeded = seed(&mut client).await;
    let madrid_only = request(&seeded, &[seeded.madrid], seeded.revision);
    assert!(export(&mut client, &madrid_only).await.is_ok());

    // A later vote makes a later pass write a new run; with no window left
    // the one shown before is pruned.
    client
        .execute(
            "INSERT INTO sequent_backend.monitoring_voter
                 (tenant_id, election_event_id, election_id, voter_id, region, country, dims,
                  first_voted_at, attributes_hash, settings_revision)
             VALUES ($1, $2, $3, 'cruz', 'Europe', 'Spain', '{}', now(), 'h', 1)",
            &[
                &seeded.event.tenant_id,
                &seeded.event.election_event_id,
                &seeded.madrid,
            ],
        )
        .await
        .unwrap();
    let settings = presets::load("comelec")
        .unwrap()
        .unwrap()
        .set
        .settings
        .clone()
        .unwrap();
    let PassOutcome::Completed { revision, .. } =
        count_event(&mut client, seeded.event, &settings, 1, seeded.generation)
            .await
            .unwrap()
    else {
        panic!("the second pass completes");
    };
    assert!(revision > seeded.revision);
    let pruned = prune_snapshots(&mut client, seeded.event, Duration::zero())
        .await
        .unwrap();
    assert_eq!(pruned.runs, 1);

    let outcome = export(&mut client, &madrid_only).await;
    assert!(
        matches!(outcome, Err(MonitoringExportError::SnapshotPruned { revision }) if revision == seeded.revision),
        "{outcome:?}"
    );
    let mut now = madrid_only.clone();
    now.snapshot_revision = revision;
    assert!(export(&mut client, &now).await.is_ok());
}
