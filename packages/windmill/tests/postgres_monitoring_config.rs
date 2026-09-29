// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Saving, resetting and switching an event's monitoring configuration: what
//! each writes and records, what it refuses, and that a refused change writes
//! nothing and records nothing.

#[path = "support/schema.rs"]
mod schema;

use async_trait::async_trait;
use deadpool_postgres::{Client, Pool, Transaction};
use sequent_core::monitoring::config::ConfigKind;
use sequent_core::monitoring::presets::{self, Preset};
use sequent_core::monitoring::problem::{Code, Problem, Report};
use sequent_core::monitoring::revision::{
    document_digest, DashboardMode, DocumentChange, Edit, RevisionOrigin,
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;
use uuid::Uuid;
use windmill::postgres::election_event::delete_election_event;
use windmill::services::monitoring::config_store::{
    get_document, get_live_config, history, reset_to_preset, save, set_mode, Author, DocumentEdit,
    EventRef, ExpectedHead, ModeError, ModeOutcome, MonitoringConfigAudit, MonitoringConfigChecks,
    ProposedChange, RecordedChange, ResetError, ResetOutcome, SaveError, SaveOutcome,
    WrittenRevision,
};

/// A fixed v4 UUID per test (`seed`) and call (`n`).
fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

fn editor() -> Author {
    Author {
        id: "admin-1".to_string(),
        name: Some("Ada Admin".to_string()),
    }
}

fn preset(id: &str) -> Preset {
    presets::load(id).unwrap().unwrap()
}

/// A tenant with an election event, committed, and no monitoring row.
async fn event(pool: &Pool, seed: u32) -> EventRef {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let event = EventRef {
        tenant_id: id(seed, 1),
        election_event_id: id(seed, 2),
    };
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
    tx.commit().await.unwrap();
    event
}

async fn remove(pool: &Pool, event: EventRef) {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    delete_election_event(
        &tx,
        &event.tenant_id.to_string(),
        &event.election_event_id.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// What is stored for the event, committed: its generation, and how many
/// revisions and heads it has.
async fn stored(pool: &Pool, event: EventRef) -> (Option<i64>, i64, i64) {
    let client = pool.get().await.unwrap();
    let row = client
        .query_one(
            "SELECT (SELECT config_generation FROM sequent_backend.monitoring_event
                     WHERE tenant_id = $1 AND election_event_id = $2),
                    (SELECT count(*) FROM sequent_backend.monitoring_config
                     WHERE tenant_id = $1 AND election_event_id = $2),
                    (SELECT count(*) FROM sequent_backend.monitoring_config_head
                     WHERE tenant_id = $1 AND election_event_id = $2)",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1), row.get(2))
}

/// Records what it is given, and can be told to fail. With a pool, it also
/// checks it runs before the change commits: the change is visible in the
/// transaction and nowhere else yet.
#[derive(Default)]
struct Audits {
    entries: Mutex<Vec<RecordedChange>>,
    fail: AtomicBool,
    pool: Option<Pool>,
    seen_before_commit: Mutex<Vec<(i64, Option<i64>)>>,
}

impl Audits {
    fn entries(&self) -> Vec<RecordedChange> {
        self.entries.lock().unwrap().clone()
    }
}

#[async_trait]
impl MonitoringConfigAudit for Audits {
    async fn record(
        &self,
        transaction: &Transaction<'_>,
        change: &RecordedChange,
    ) -> anyhow::Result<()> {
        if let Some(pool) = &self.pool {
            let query = "SELECT config_generation FROM sequent_backend.monitoring_event
                         WHERE tenant_id = $1 AND election_event_id = $2";
            let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] =
                [&change.event.tenant_id, &change.event.election_event_id];
            let inside: i64 = transaction.query_one(query, &params).await?.get(0);
            let outside: Option<i64> = pool
                .get()
                .await?
                .query_opt(query, &params)
                .await?
                .map(|row| row.get(0));
            self.seen_before_commit
                .lock()
                .unwrap()
                .push((inside, outside));
        }
        if self.fail.load(Ordering::SeqCst) {
            anyhow::bail!("the electoral log is unreachable");
        }
        self.entries.lock().unwrap().push(change.clone());
        Ok(())
    }
}

/// Stands in for the chart engine: answers with the problems it is given,
/// or fails, and can hold a save inside its checks until released.
#[derive(Default)]
struct Checks {
    problems: Mutex<Report>,
    fail: AtomicBool,
    calls: AtomicUsize,
    hold: Option<(Arc<Notify>, Arc<Notify>)>,
}

#[async_trait]
impl MonitoringConfigChecks for Checks {
    async fn check(&self, change: &ProposedChange<'_>) -> anyhow::Result<Report> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(
            change.set.widgets.contains_key(change.key)
                || change.kind != ConfigKind::Widget
                || change.edit == Edit::Delete,
            "the checks see the set the save leaves"
        );
        if let Some((entered, release)) = &self.hold {
            entered.notify_one();
            release.notified().await;
        }
        if self.fail.load(Ordering::SeqCst) {
            anyhow::bail!("the renderer is unreachable");
        }
        Ok(self.problems.lock().unwrap().clone())
    }
}

fn kind_order(kind: ConfigKind) -> u8 {
    match kind {
        ConfigKind::Settings => 0,
        ConfigKind::Theme => 1,
        ConfigKind::Widget => 2,
        ConfigKind::Dashboard => 3,
    }
}

/// The first document of a kind in a preset.
fn first(preset: &Preset, kind: ConfigKind) -> (String, &'static str) {
    let document = preset
        .documents
        .iter()
        .find(|document| document.kind == kind)
        .unwrap();
    (document.key.clone(), document.yaml)
}

async fn reset(
    client: &mut Client,
    audit: &Audits,
    event: EventRef,
    preset_id: &str,
    mode: DashboardMode,
) -> Result<ResetOutcome, ResetError> {
    reset_to_preset(client, audit, event, &editor(), preset_id, mode).await
}

async fn edit(
    client: &mut Client,
    checks: &Checks,
    audit: &Audits,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
    edit: Edit<'_>,
    expected: ExpectedHead,
) -> Result<SaveOutcome, SaveError> {
    save(
        client,
        checks,
        audit,
        event,
        &editor(),
        DocumentEdit {
            kind,
            key,
            edit,
            expected,
        },
    )
    .await
}

fn saved(outcome: Result<SaveOutcome, SaveError>) -> i32 {
    match outcome {
        Ok(SaveOutcome::Saved { revision, .. }) => revision.revision,
        other => panic!("expected a saved revision, got {other:?}"),
    }
}

fn refused(outcome: Result<SaveOutcome, SaveError>) -> Report {
    match outcome {
        Err(SaveError::Invalid(report)) => report,
        other => panic!("expected the save refused as invalid, got {other:?}"),
    }
}

fn codes(report: &Report) -> Vec<Code> {
    report.problems.iter().map(|problem| problem.code).collect()
}

#[tokio::test]
async fn an_event_without_configuration_is_left_on_the_standard_dashboard() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());

    let tx = client.transaction().await.unwrap();
    assert!(get_live_config(&tx, event).await.unwrap().is_none());
    tx.rollback().await.unwrap();

    let outcome = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        "turnout",
        Edit::Upsert("id: turnout\n"),
        ExpectedHead::Absent,
    )
    .await;
    assert!(
        matches!(outcome, Err(SaveError::NotConfigured)),
        "{outcome:?}"
    );
    let switched = set_mode(
        &mut client,
        &audit,
        event,
        &editor(),
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(switched, Err(ModeError::NotConfigured)),
        "{switched:?}"
    );

    assert_eq!(stored(&pool, event).await, (None, 0, 0));
    assert!(audit.entries().is_empty());
    assert_eq!(checks.calls.load(Ordering::SeqCst), 0);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_reset_writes_the_preset_and_records_it_before_it_commits() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let audit = Audits {
        pool: Some(pool.clone()),
        ..Audits::default()
    };
    let campus = preset("campus");

    let outcome = reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await;
    let written = match outcome {
        Ok(ResetOutcome::Reset {
            generation,
            revisions,
        }) => {
            assert_eq!(generation, 1);
            revisions
        }
        other => panic!("expected a reset, got {other:?}"),
    };
    let expected: Vec<WrittenRevision> = campus
        .documents
        .iter()
        .map(|document| WrittenRevision {
            kind: document.kind,
            key: document.key.clone(),
            revision: 1,
            change: DocumentChange::Upsert,
            digest: Some(document_digest(document.yaml)),
        })
        .collect();
    assert_eq!(written, expected);
    let documents = campus.documents.len() as i64;
    assert_eq!(stored(&pool, event).await, (Some(1), documents, documents));

    let entries = audit.entries();
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.event, event);
    assert_eq!(entry.author, editor());
    assert_eq!(entry.origin, RevisionOrigin::Preset);
    assert_eq!(
        entry.preset.as_ref().map(|p| (p.id.as_str(), p.version)),
        Some(("campus", campus.manifest.version as i32))
    );
    assert_eq!(entry.mode, DashboardMode::Configured);
    assert_eq!(entry.generation, 1);
    assert_eq!(entry.revisions, expected);
    // The event row was created by the reset itself, so outside the
    // transaction there was no row yet.
    assert_eq!(*audit.seen_before_commit.lock().unwrap(), vec![(1, None)]);

    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(live.mode, DashboardMode::Configured);
    assert_eq!(live.generation, 1);
    assert_eq!(
        live.preset.as_ref().map(|p| (p.id.as_str(), p.version)),
        Some(("campus", campus.manifest.version as i32))
    );
    assert_eq!(live.assembled.set, campus.set);
    assert_eq!(live.assembled.report, Report::default());
    assert_eq!(live.documents.len(), campus.documents.len());
    let listed: Vec<(ConfigKind, &str, Option<&str>)> = live
        .documents
        .iter()
        .map(|document| {
            (
                document.kind,
                document.key.as_str(),
                document.yaml.as_deref(),
            )
        })
        .collect();
    let mut expected_listing: Vec<(ConfigKind, &str, Option<&str>)> = campus
        .documents
        .iter()
        .map(|document| (document.kind, document.key.as_str(), Some(document.yaml)))
        .collect();
    expected_listing.sort_by_key(|(kind, key, _)| (kind_order(*kind), key.to_string()));
    assert_eq!(
        listed, expected_listing,
        "settings, themes, widgets, dashboards; each by key"
    );
    for document in &live.documents {
        assert_eq!(document.origin, RevisionOrigin::Preset);
        assert_eq!(document.author, editor());
        assert_eq!(document.config_generation, 1);
        assert_eq!(document.revision, 1);
    }
    tx.rollback().await.unwrap();

    // The same preset again changes nothing, and records nothing.
    let again = reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await;
    assert!(matches!(again, Ok(ResetOutcome::Unchanged)), "{again:?}");
    assert_eq!(stored(&pool, event).await, (Some(1), documents, documents));
    assert_eq!(audit.entries().len(), 1);

    // An unknown preset is refused before anything is written.
    let unknown = reset(
        &mut client,
        &audit,
        event,
        "nowhere",
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(unknown, Err(ResetError::UnknownPreset)),
        "{unknown:?}"
    );
    assert_eq!(stored(&pool, event).await, (Some(1), documents, documents));
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_reset_to_another_preset_leaves_exactly_its_documents() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());
    let (campus, comelec) = (preset("campus"), preset("comelec"));
    reset(&mut client, &audit, event, "campus", DashboardMode::Legacy)
        .await
        .unwrap();
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    let edited = format!("{yaml}\n# edited\n");
    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                &widget,
                Edit::Upsert(&edited),
                ExpectedHead::At(1),
            )
            .await
        ),
        2
    );

    let outcome = reset(
        &mut client,
        &audit,
        event,
        "comelec",
        DashboardMode::Configured,
    )
    .await;
    let Ok(ResetOutcome::Reset {
        generation,
        revisions,
    }) = outcome
    else {
        panic!("expected a reset, got {outcome:?}");
    };
    assert_eq!(generation, 3);
    let removed: Vec<&WrittenRevision> = revisions
        .iter()
        .filter(|revision| revision.change == DocumentChange::Delete)
        .collect();
    let gone = campus
        .documents
        .iter()
        .filter(|document| {
            !comelec
                .documents
                .iter()
                .any(|other| other.kind == document.kind && other.key == document.key)
        })
        .count();
    assert_eq!(removed.len(), gone);
    assert!(removed.iter().all(|revision| revision.digest.is_none()));

    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(live.assembled.set, comelec.set);
    assert_eq!(live.mode, DashboardMode::Configured);
    assert_eq!(live.preset.as_ref().map(|p| p.id.as_str()), Some("comelec"));
    // A removed document is kept in the history, at its removal.
    let removed_first = removed.first().expect("campus has documents comelec lacks");
    let kept = history(&tx, event, removed_first.kind, &removed_first.key)
        .await
        .unwrap();
    assert_eq!(
        kept.len() as i32,
        removed_first.revision,
        "one revision each"
    );
    assert_eq!(
        (kept[0].revision, kept[0].change, kept[0].origin),
        (
            removed_first.revision,
            DocumentChange::Delete,
            RevisionOrigin::Preset
        )
    );
    assert_eq!(kept[1].change, DocumentChange::Upsert);
    tx.rollback().await.unwrap();

    let entries = audit.entries();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[2].revisions, revisions);
    assert_eq!(entries[2].generation, 3);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_save_writes_one_revision_and_records_it() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let checks = Checks::default();
    let audit = Audits {
        pool: Some(pool.clone()),
        ..Audits::default()
    };
    let campus = preset("campus");
    reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    let edited = format!("{yaml}\n# edited\n");
    *checks.problems.lock().unwrap() = Report {
        problems: vec![
            Problem::warning(Code::ChartSchema, "chart.charts.bars", "labels overlap")
                .with_engine_code("WARN-LABELS"),
        ],
    };

    let outcome = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        &widget,
        Edit::Upsert(&edited),
        ExpectedHead::At(1),
    )
    .await;
    let Ok(SaveOutcome::Saved { revision, warnings }) = outcome else {
        panic!("expected a saved revision, got {outcome:?}");
    };
    assert_eq!(
        (revision.revision, revision.change, revision.yaml.as_deref()),
        (2, DocumentChange::Upsert, Some(edited.as_str()))
    );
    assert_eq!(
        revision.sha256.as_deref(),
        Some(document_digest(&edited).as_str())
    );
    assert_eq!(
        (
            revision.origin,
            revision.preset.clone(),
            revision.config_generation
        ),
        (RevisionOrigin::Editor, None, 2)
    );
    assert_eq!(revision.author, editor());
    assert_eq!(
        codes(&warnings),
        vec![Code::ChartSchema],
        "the engine's warning comes back"
    );
    assert_eq!(checks.calls.load(Ordering::SeqCst), 1);

    let entries = audit.entries();
    let entry = entries.last().unwrap();
    assert_eq!(
        (
            entry.origin,
            entry.preset.clone(),
            entry.mode,
            entry.generation
        ),
        (RevisionOrigin::Editor, None, DashboardMode::Configured, 2)
    );
    assert_eq!(
        entry.revisions,
        vec![WrittenRevision {
            kind: ConfigKind::Widget,
            key: widget.clone(),
            revision: 2,
            change: DocumentChange::Upsert,
            digest: Some(document_digest(&edited)),
        }]
    );
    assert_eq!(
        audit.seen_before_commit.lock().unwrap().last(),
        Some(&(2, Some(1)))
    );

    let tx = client.transaction().await.unwrap();
    let revisions = history(&tx, event, ConfigKind::Widget, &widget)
        .await
        .unwrap();
    assert_eq!(
        revisions
            .iter()
            .map(|revision| (revision.revision, revision.origin))
            .collect::<Vec<_>>(),
        vec![(2, RevisionOrigin::Editor), (1, RevisionOrigin::Preset)],
        "newest first"
    );
    let older = get_document(&tx, event, ConfigKind::Widget, &widget, Some(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(older.yaml.as_deref(), Some(yaml));
    let head = get_document(&tx, event, ConfigKind::Widget, &widget, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(head, revision);
    assert!(
        get_document(&tx, event, ConfigKind::Widget, &widget, Some(3))
            .await
            .unwrap()
            .is_none()
    );
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(live.generation, 2);
    assert!(live.documents.iter().any(|document| document == &revision));
    tx.rollback().await.unwrap();

    // Saving the text the document already has writes and records nothing.
    let same = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        &widget,
        Edit::Upsert(&edited),
        ExpectedHead::At(2),
    )
    .await;
    assert!(
        matches!(&same, Ok(SaveOutcome::Unchanged(current)) if current.revision == 2),
        "{same:?}"
    );
    let documents = campus.documents.len() as i64;
    assert_eq!(
        stored(&pool, event).await,
        (Some(2), documents + 1, documents)
    );
    assert_eq!(audit.entries().len(), 2);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_save_from_an_older_revision_is_a_conflict_that_names_the_current_one() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());
    let campus = preset("campus");
    reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    let (mine, theirs) = (format!("{yaml}\n# mine\n"), format!("{yaml}\n# theirs\n"));
    saved(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            &widget,
            Edit::Upsert(&theirs),
            ExpectedHead::At(1),
        )
        .await,
    );
    let before = stored(&pool, event).await;
    let calls = checks.calls.load(Ordering::SeqCst);

    for (change, expected) in [
        (Edit::Upsert(&mine), ExpectedHead::At(1)),
        (Edit::Upsert(&mine), ExpectedHead::Absent),
        (Edit::Upsert(&theirs), ExpectedHead::At(1)),
        (Edit::Delete, ExpectedHead::At(1)),
        (Edit::Upsert(&mine), ExpectedHead::At(3)),
    ] {
        let outcome = edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            &widget,
            change,
            expected,
        )
        .await;
        match outcome {
            Err(SaveError::Conflict {
                current: Some(current),
            }) => {
                assert_eq!(
                    (current.revision, current.yaml.as_deref()),
                    (2, Some(theirs.as_str()))
                );
                assert_eq!(current.author, editor());
            }
            other => panic!("{change:?} from {expected:?}: expected a conflict, got {other:?}"),
        }
    }
    // A document that does not exist has no current revision to name.
    let outcome = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        "nobody",
        Edit::Upsert("id: nobody\n"),
        ExpectedHead::At(1),
    )
    .await;
    assert!(
        matches!(outcome, Err(SaveError::Conflict { current: None })),
        "{outcome:?}"
    );

    assert_eq!(
        stored(&pool, event).await,
        before,
        "a conflict writes nothing"
    );
    assert_eq!(audit.entries().len(), 2, "and records nothing");
    assert_eq!(
        checks.calls.load(Ordering::SeqCst),
        calls,
        "nor asks the renderer"
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_refused_save_writes_nothing_and_records_nothing() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());
    let campus = preset("campus");
    reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    let before = stored(&pool, event).await;
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    let used = campus.set.dashboards[0].layout[0].widget.clone();
    let (settings, _) = first(&campus, ConfigKind::Settings);

    let unreadable = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            &widget,
            Edit::Upsert("id: [\n"),
            ExpectedHead::At(1),
        )
        .await,
    );
    assert_eq!(codes(&unreadable), vec![Code::Unreadable]);
    let preset_only = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Settings,
            &settings,
            Edit::Upsert("time_zone: UTC\n"),
            ExpectedHead::At(1),
        )
        .await,
    );
    assert_eq!(codes(&preset_only), vec![Code::PresetOnly]);
    let default_theme = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Theme,
            "default",
            Edit::Delete,
            ExpectedHead::At(1),
        )
        .await,
    );
    assert_eq!(codes(&default_theme), vec![Code::DanglingReference]);
    let dangling = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            &used,
            Edit::Delete,
            ExpectedHead::At(1),
        )
        .await,
    );
    assert!(
        codes(&dangling).contains(&Code::DanglingReference),
        "{dangling:?}"
    );
    let not_an_id = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            "Not An Id",
            Edit::Upsert(yaml),
            ExpectedHead::Absent,
        )
        .await,
    );
    assert_eq!(codes(&not_an_id), vec![Code::InvalidId]);
    let nothing_to_remove = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            "nobody",
            Edit::Delete,
            ExpectedHead::Absent,
        )
        .await,
    );
    assert_eq!(codes(&nothing_to_remove), vec![Code::DanglingReference]);
    assert_eq!(
        checks.calls.load(Ordering::SeqCst),
        0,
        "the renderer is asked only about what may be saved"
    );

    // The chart engine refuses the chart.
    let edited = format!("{yaml}\n# edited\n");
    *checks.problems.lock().unwrap() = Report {
        problems: vec![
            Problem::error(Code::ChartSchema, "chart", "unknown mark").with_engine_code("ERR-MARK")
        ],
    };
    let engine = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            &widget,
            Edit::Upsert(&edited),
            ExpectedHead::At(1),
        )
        .await,
    );
    assert_eq!(codes(&engine), vec![Code::ChartSchema]);
    assert_eq!(engine.problems[0].engine_code.as_deref(), Some("ERR-MARK"));
    // The chart engine cannot be asked.
    *checks.problems.lock().unwrap() = Report::default();
    checks.fail.store(true, Ordering::SeqCst);
    let unreachable = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        &widget,
        Edit::Upsert(&edited),
        ExpectedHead::At(1),
    )
    .await;
    assert!(
        matches!(unreachable, Err(SaveError::Internal(_))),
        "{unreachable:?}"
    );
    checks.fail.store(false, Ordering::SeqCst);
    // The electoral log cannot be written: the change is not kept either.
    audit.fail.store(true, Ordering::SeqCst);
    let unaudited = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        &widget,
        Edit::Upsert(&edited),
        ExpectedHead::At(1),
    )
    .await;
    assert!(
        matches!(unaudited, Err(SaveError::Internal(_))),
        "{unaudited:?}"
    );
    let unaudited_reset = reset(
        &mut client,
        &audit,
        event,
        "comelec",
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(unaudited_reset, Err(ResetError::Internal(_))),
        "{unaudited_reset:?}"
    );
    let unaudited_switch =
        set_mode(&mut client, &audit, event, &editor(), DashboardMode::Legacy).await;
    assert!(
        matches!(unaudited_switch, Err(ModeError::Internal(_))),
        "{unaudited_switch:?}"
    );
    audit.fail.store(false, Ordering::SeqCst);

    assert_eq!(stored(&pool, event).await, before);
    assert_eq!(audit.entries().len(), 1, "only the reset");
    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(
        (live.mode, live.assembled.set),
        (DashboardMode::Configured, campus.set.clone())
    );
    tx.rollback().await.unwrap();

    // Once the log is back, the same save goes through.
    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                &widget,
                Edit::Upsert(&edited),
                ExpectedHead::At(1)
            )
            .await
        ),
        2
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_removed_document_keeps_its_history_and_its_key_can_be_used_again() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());
    let campus = preset("campus");
    reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    let copy = yaml.replacen(&format!("id: {widget}"), "id: copy", 1);
    assert_ne!(copy, yaml, "the widget's id is on a line of its own");

    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                "copy",
                Edit::Upsert(&copy),
                ExpectedHead::Absent
            )
            .await
        ),
        1
    );
    let outcome = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        "copy",
        Edit::Upsert(&copy),
        ExpectedHead::Absent,
    )
    .await;
    assert!(
        matches!(&outcome, Err(SaveError::Conflict { current: Some(current) }) if current.revision == 1),
        "{outcome:?}"
    );

    let removed = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        "copy",
        Edit::Delete,
        ExpectedHead::At(1),
    )
    .await;
    let Ok(SaveOutcome::Saved { revision, .. }) = removed else {
        panic!("expected a removal, got {removed:?}");
    };
    assert_eq!(
        (
            revision.revision,
            revision.change,
            revision.yaml,
            revision.sha256
        ),
        (2, DocumentChange::Delete, None, None)
    );
    assert_eq!(audit.entries().last().unwrap().revisions[0].digest, None);

    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert!(!live.assembled.set.widgets.contains_key("copy"));
    assert!(
        live.documents.iter().all(|document| document.key != "copy"),
        "a removed document is not live"
    );
    assert_eq!(
        get_document(&tx, event, ConfigKind::Widget, "copy", None)
            .await
            .unwrap()
            .map(|head| head.change),
        Some(DocumentChange::Delete)
    );
    tx.rollback().await.unwrap();

    // Removed, it is absent: saving from its removal is a conflict that
    // names the removal, and removing it again has nothing to remove.
    let outcome = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        "copy",
        Edit::Upsert(&copy),
        ExpectedHead::At(2),
    )
    .await;
    assert!(
        matches!(&outcome, Err(SaveError::Conflict { current: Some(current) }) if (current.revision, current.change) == (2, DocumentChange::Delete)),
        "{outcome:?}"
    );
    let again = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            "copy",
            Edit::Delete,
            ExpectedHead::Absent,
        )
        .await,
    );
    assert_eq!(codes(&again), vec![Code::DanglingReference]);

    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                "copy",
                Edit::Upsert(&copy),
                ExpectedHead::Absent
            )
            .await
        ),
        3
    );
    let tx = client.transaction().await.unwrap();
    let revisions = history(&tx, event, ConfigKind::Widget, "copy")
        .await
        .unwrap();
    assert_eq!(
        revisions
            .iter()
            .map(|revision| (revision.revision, revision.change))
            .collect::<Vec<_>>(),
        vec![
            (3, DocumentChange::Upsert),
            (2, DocumentChange::Delete),
            (1, DocumentChange::Upsert)
        ]
    );
    tx.rollback().await.unwrap();
    remove(&pool, event).await;
}

#[tokio::test]
async fn the_dashboard_tab_is_switched_only_to_dashboards_that_exist() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let audit = Audits::default();
    reset(&mut client, &audit, event, "campus", DashboardMode::Legacy)
        .await
        .unwrap();

    let same = set_mode(&mut client, &audit, event, &editor(), DashboardMode::Legacy).await;
    assert!(matches!(same, Ok(ModeOutcome::Unchanged)), "{same:?}");
    let on = set_mode(
        &mut client,
        &audit,
        event,
        &editor(),
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(on, Ok(ModeOutcome::Switched { generation: 2 })),
        "{on:?}"
    );
    let entry = audit.entries().pop().unwrap();
    assert_eq!(
        (
            entry.origin,
            entry.mode,
            entry.generation,
            entry.revisions.len()
        ),
        (RevisionOrigin::Editor, DashboardMode::Configured, 2, 0)
    );
    let off = set_mode(&mut client, &audit, event, &editor(), DashboardMode::Legacy).await;
    assert!(
        matches!(off, Ok(ModeOutcome::Switched { generation: 3 })),
        "{off:?}"
    );
    assert_eq!(audit.entries().len(), 3);
    remove(&pool, event).await;

    // An event whose configuration has no dashboard stays on the standard
    // one: a configuration written before the checks it would now fail.
    let event = self::event(&pool, line!()).await;
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id) VALUES ($1, $2)",
        &[&event.tenant_id, &event.election_event_id],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let empty = set_mode(
        &mut client,
        &audit,
        event,
        &editor(),
        DashboardMode::Configured,
    )
    .await;
    match empty {
        Err(ModeError::Invalid(report)) => {
            assert_eq!(codes(&report), vec![Code::DanglingReference])
        }
        other => panic!("expected the switch refused, got {other:?}"),
    }
    assert_eq!(stored(&pool, event).await, (Some(0), 0, 0));
    assert_eq!(audit.entries().len(), 3);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_document_that_no_longer_passes_is_left_out_and_said_why() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let audit = Audits::default();
    let campus = preset("campus");
    reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    let (widget, _) = first(&campus, ConfigKind::Widget);
    // Written as an older release might have, by the save protocol.
    let tx = client.transaction().await.unwrap();
    let stale = "id: stale\nunknown_field: true\n";
    let generation: i64 = tx
        .query_one(
            "UPDATE sequent_backend.monitoring_event SET config_generation = config_generation + 1
             WHERE tenant_id = $1 AND election_event_id = $2 RETURNING config_generation",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .unwrap()
        .get(0);
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_config_head (tenant_id, election_event_id, kind, key, revision)
         VALUES ($1, $2, 'widget', 'stale', 1)",
        &[&event.tenant_id, &event.election_event_id],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_config
             (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256, origin,
              config_generation, author_id)
         VALUES ($1, $2, 'widget', 'stale', 1, 'UPSERT', $3, $4, 'EDITOR', $5, 'admin-1')",
        &[
            &event.tenant_id,
            &event.election_event_id,
            &stale,
            &document_digest(stale),
            &generation,
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(
        live.assembled.set, campus.set,
        "the stale widget is left out"
    );
    assert!(
        live.assembled
            .report
            .errors()
            .all(|problem| problem.path.starts_with("widgets.stale")),
        "{:?}",
        live.assembled.report
    );
    assert!(!live.assembled.report.is_accepted());
    assert!(
        live.documents
            .iter()
            .any(|document| document.key == "stale"),
        "and still listed, to be fixed"
    );
    tx.rollback().await.unwrap();

    // The stale document does not block editing another.
    let checks = Checks::default();
    let (_, yaml) = first(&campus, ConfigKind::Widget);
    let edited = format!("{yaml}\n# edited\n");
    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                &widget,
                Edit::Upsert(&edited),
                ExpectedHead::At(1)
            )
            .await
        ),
        2
    );
    // And it can be fixed from its revision.
    let fixed = campus
        .documents
        .iter()
        .find(|d| d.key == widget)
        .unwrap()
        .yaml
        .replacen(&format!("id: {widget}"), "id: stale", 1);
    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                "stale",
                Edit::Upsert(&fixed),
                ExpectedHead::At(1)
            )
            .await
        ),
        2
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn of_two_saves_from_one_revision_the_second_waits_then_conflicts() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let audit = Arc::new(Audits::default());
    let campus = preset("campus");
    {
        let mut client = pool.get().await.unwrap();
        reset(
            &mut client,
            &audit,
            event,
            "campus",
            DashboardMode::Configured,
        )
        .await
        .unwrap();
    }
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    let (entered, release) = (Arc::new(Notify::new()), Arc::new(Notify::new()));
    let held = Arc::new(Checks {
        hold: Some((entered.clone(), release.clone())),
        ..Checks::default()
    });
    let free = Arc::new(Checks::default());

    let start = |checks: Arc<Checks>, text: String| {
        let (pool, audit, widget) = (pool.clone(), audit.clone(), widget.clone());
        tokio::spawn(async move {
            let mut client = pool.get().await.unwrap();
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                &widget,
                Edit::Upsert(&text),
                ExpectedHead::At(1),
            )
            .await
        })
    };
    let first_save = start(held.clone(), format!("{yaml}\n# first\n"));
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .expect("the first save reaches its checks");
    let second_save = start(free.clone(), format!("{yaml}\n# second\n"));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !second_save.is_finished(),
        "the second save waits for the first"
    );
    assert_eq!(free.calls.load(Ordering::SeqCst), 0);

    release.notify_one();
    assert_eq!(saved(first_save.await.unwrap()), 2);
    let second = second_save.await.unwrap();
    assert!(
        matches!(&second, Err(SaveError::Conflict { current: Some(current) }) if current.revision == 2),
        "{second:?}"
    );
    assert_eq!(
        free.calls.load(Ordering::SeqCst),
        0,
        "a conflict is found before the renderer is asked"
    );
    assert_eq!(audit.entries().len(), 2);
    remove(&pool, event).await;
}

#[test]
fn the_electoral_log_records_each_revision_with_its_digest() {
    use electoral_log::messages::newtypes::{
        MonitoringConfigChangeAction, MonitoringConfigOrigin, MonitoringDashboardMode,
    };
    use windmill::services::monitoring::audit::change_details;
    use windmill::services::monitoring::config_store::PresetVersion;

    let change = RecordedChange {
        event: EventRef {
            tenant_id: id(1, 1),
            election_event_id: id(1, 2),
        },
        author: editor(),
        origin: RevisionOrigin::Preset,
        preset: Some(PresetVersion {
            id: "campus".to_string(),
            version: 3,
        }),
        mode: DashboardMode::Configured,
        generation: 7,
        revisions: vec![
            WrittenRevision {
                kind: ConfigKind::Widget,
                key: "turnout".to_string(),
                revision: 2,
                change: DocumentChange::Upsert,
                digest: Some(document_digest("id: turnout\n")),
            },
            WrittenRevision {
                kind: ConfigKind::Dashboard,
                key: "old".to_string(),
                revision: 5,
                change: DocumentChange::Delete,
                digest: None,
            },
        ],
    };
    let details = change_details(&change).unwrap();
    assert_eq!(details.origin, MonitoringConfigOrigin::Preset);
    assert_eq!(
        details
            .preset
            .as_ref()
            .map(|p| (p.id.0.as_str(), p.version)),
        Some(("campus", 3))
    );
    assert_eq!(details.mode, MonitoringDashboardMode::Configured);
    assert_eq!(details.generation, 7);
    let revisions: Vec<_> = details
        .revisions
        .iter()
        .map(|r| {
            (
                r.kind.0.as_str(),
                r.key.0.as_str(),
                r.revision,
                r.action,
                r.digest.as_ref().map(|d| d.0.clone()),
            )
        })
        .collect();
    assert_eq!(
        revisions,
        vec![
            (
                "widget",
                "turnout",
                2,
                MonitoringConfigChangeAction::Upsert,
                Some(document_digest("id: turnout\n"))
            ),
            (
                "dashboard",
                "old",
                5,
                MonitoringConfigChangeAction::Delete,
                None
            ),
        ]
    );

    let editor_change = RecordedChange {
        origin: RevisionOrigin::Editor,
        preset: None,
        mode: DashboardMode::Legacy,
        revisions: Vec::new(),
        ..change
    };
    let details = change_details(&editor_change).unwrap();
    assert_eq!(
        (
            details.origin,
            details.preset,
            details.mode,
            details.revisions.len()
        ),
        (
            MonitoringConfigOrigin::Editor,
            None,
            MonitoringDashboardMode::Legacy,
            0
        )
    );
}
