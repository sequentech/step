// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Saving, resetting and switching an event's monitoring configuration: what
//! each writes and records, what it refuses, and that a refused change writes
//! nothing and records nothing.

#[path = "support/schema.rs"]
mod schema;

use async_trait::async_trait;
use deadpool_postgres::{Client, Pool, Transaction};
use sequent_core::monitoring::config::{ConfigKind, ConfigSet};
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
    get_config_at_generation, get_document, get_live_config, history, reset_to_preset, save,
    set_mode, Author, DocumentEdit, EventRef, ExpectedHead, ModeError, ModeOutcome,
    MonitoringConfigAudit, MonitoringConfigChecks, ProposedChange, RecordedChange, ResetError,
    ResetOutcome, SaveError, SaveOutcome, WrittenRevision,
};
use windmill::services::vault::admin_user_signing_key_exists;

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
/// transaction and nowhere else yet. With `probe`, it also checks that the
/// transaction's deferred checks were made before it was asked to record: a
/// head naming no revision is refused at once.
#[derive(Default)]
struct Audits {
    entries: Mutex<Vec<RecordedChange>>,
    fail: AtomicBool,
    pool: Option<Pool>,
    seen_before_commit: Mutex<Vec<(i64, Option<i64>)>>,
    /// Each preparation: the author, and the event's generation committed
    /// then.
    prepared: Mutex<Vec<(String, Option<i64>)>>,
    fail_prepare: AtomicBool,
    probe: AtomicBool,
    immediate: Mutex<Vec<bool>>,
    /// Whether prepare leaves a committed row in `prepared_for`, as the
    /// electoral log leaves an author's signing key.
    mark: AtomicBool,
    /// Each call, in order.
    calls: Mutex<Vec<&'static str>>,
}

impl Audits {
    fn entries(&self) -> Vec<RecordedChange> {
        self.entries.lock().unwrap().clone()
    }

    fn prepared(&self) -> Vec<(String, Option<i64>)> {
        self.prepared.lock().unwrap().clone()
    }
}

const GENERATION: &str = "SELECT config_generation FROM sequent_backend.monitoring_event
                          WHERE tenant_id = $1 AND election_event_id = $2";

#[async_trait]
impl MonitoringConfigAudit for Audits {
    async fn prepare(
        &self,
        client: &mut Client,
        event: EventRef,
        author: &Author,
    ) -> anyhow::Result<()> {
        self.calls.lock().unwrap().push("prepare");
        let generation: Option<i64> = client
            .query_opt(GENERATION, &[&event.tenant_id, &event.election_event_id])
            .await?
            .map(|row| row.get(0));
        self.prepared
            .lock()
            .unwrap()
            .push((author.id.clone(), generation));
        if self.fail_prepare.load(Ordering::SeqCst) {
            anyhow::bail!("the vault is unreachable");
        }
        if self.mark.load(Ordering::SeqCst) {
            client
                .execute(
                    "INSERT INTO prepared_for (election_event_id, author_id) VALUES ($1, $2)",
                    &[&event.election_event_id, &author.id],
                )
                .await?;
        }
        Ok(())
    }

    async fn record(
        &self,
        transaction: &Transaction<'_>,
        change: &RecordedChange,
    ) -> anyhow::Result<()> {
        self.calls.lock().unwrap().push("record");
        let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] =
            [&change.event.tenant_id, &change.event.election_event_id];
        if let Some(pool) = &self.pool {
            let inside: i64 = transaction.query_one(GENERATION, &params).await?.get(0);
            let outside: Option<i64> = pool
                .get()
                .await?
                .query_opt(GENERATION, &params)
                .await?
                .map(|row| row.get(0));
            self.seen_before_commit
                .lock()
                .unwrap()
                .push((inside, outside));
        }
        if self.probe.load(Ordering::SeqCst) {
            transaction.batch_execute("SAVEPOINT audit_probe").await?;
            let inserted = transaction
                .execute(
                    "INSERT INTO sequent_backend.monitoring_config_head
                         (tenant_id, election_event_id, kind, key, revision)
                     VALUES ($1, $2, 'widget', 'audit-probe', 1)",
                    &params,
                )
                .await;
            let immediate = inserted.as_ref().is_err_and(|error| {
                error.as_db_error().and_then(|db| db.constraint())
                    == Some("monitoring_config_head_names_a_revision")
            });
            transaction
                .batch_execute("ROLLBACK TO SAVEPOINT audit_probe")
                .await?;
            self.immediate.lock().unwrap().push(immediate);
        }
        if self.fail.load(Ordering::SeqCst) {
            anyhow::bail!("the electoral log is unreachable");
        }
        self.entries.lock().unwrap().push(change.clone());
        Ok(())
    }
}

/// Stands in for the chart engine: answers with the problems it is given,
/// or fails. It can hold its first check until released, and, with a pool,
/// move the event on (raise its generation) during each check, as a change
/// made meanwhile would.
#[derive(Default)]
struct Checks {
    problems: Mutex<Report>,
    fail: AtomicBool,
    calls: AtomicUsize,
    hold: Option<(Arc<Notify>, Arc<Notify>)>,
    move_on: Option<Pool>,
    /// The set each check was of, in order.
    sets: Mutex<Vec<ConfigSet>>,
}

#[async_trait]
impl MonitoringConfigChecks for Checks {
    async fn check(&self, change: &ProposedChange<'_>) -> anyhow::Result<Report> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        self.sets.lock().unwrap().push(change.set.clone());
        assert!(
            change.set.widgets.contains_key(change.key)
                || change.kind != ConfigKind::Widget
                || change.edit == Edit::Delete,
            "the checks see the set the save leaves"
        );
        if let (Some((entered, release)), 0) = (&self.hold, call) {
            entered.notify_one();
            release.notified().await;
        }
        if let Some(pool) = &self.move_on {
            pool.get()
                .await?
                .execute(
                    "UPDATE sequent_backend.monitoring_event
                     SET config_generation = config_generation + 1
                     WHERE tenant_id = $1 AND election_event_id = $2",
                    &[&change.event.tenant_id, &change.event.election_event_id],
                )
                .await?;
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
            warnings,
        }) => {
            assert_eq!(generation, 1);
            assert_eq!(warnings, campus.warnings, "the preset's warnings come back");
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
        ..
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
    // What the document says already is no conflict, from any revision.
    let same = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        &widget,
        Edit::Upsert(&theirs),
        ExpectedHead::At(1),
    )
    .await;
    assert!(
        matches!(&same, Ok(SaveOutcome::Unchanged(head)) if head.revision == 2),
        "{same:?}"
    );
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
    assert_eq!(codes(&nothing_to_remove), vec![Code::NothingToRemove]);
    // What may be edited at all is checked before the document's head: the
    // settings' own text is still refused, and a key that is no id is not a
    // conflict with a document that cannot exist.
    let (_, settings_yaml) = first(&campus, ConfigKind::Settings);
    let same_settings = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Settings,
            &settings,
            Edit::Upsert(settings_yaml),
            ExpectedHead::At(1),
        )
        .await,
    );
    assert_eq!(codes(&same_settings), vec![Code::PresetOnly]);
    let not_an_id_at_one = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            "Not An Id",
            Edit::Upsert(yaml),
            ExpectedHead::At(1),
        )
        .await,
    );
    assert_eq!(codes(&not_an_id_at_one), vec![Code::InvalidId]);
    assert_eq!(
        checks.calls.load(Ordering::SeqCst),
        0,
        "the renderer is asked only about what may be saved"
    );
    assert_eq!(
        audit.prepared().len(),
        1,
        "nor is the log prepared for what is refused"
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
        matches!(unreachable, Err(SaveError::ChecksUnavailable(_))),
        "{unreachable:?}"
    );
    checks.fail.store(false, Ordering::SeqCst);
    // The electoral log cannot be prepared: nothing is written or recorded.
    audit.fail_prepare.store(true, Ordering::SeqCst);
    let unprepared = edit(
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
        matches!(unprepared, Err(SaveError::Internal(_))),
        "{unprepared:?}"
    );
    let unprepared_reset = reset(
        &mut client,
        &audit,
        event,
        "comelec",
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(unprepared_reset, Err(ResetError::Internal(_))),
        "{unprepared_reset:?}"
    );
    let unprepared_switch =
        set_mode(&mut client, &audit, event, &editor(), DashboardMode::Legacy).await;
    assert!(
        matches!(unprepared_switch, Err(ModeError::Internal(_))),
        "{unprepared_switch:?}"
    );
    audit.fail_prepare.store(false, Ordering::SeqCst);
    assert_eq!(
        audit
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| **call == "record")
            .count(),
        1,
        "only the reset was recorded"
    );
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
        Edit::Upsert(&format!("{copy}\n# another\n")),
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

    // Removed, it is absent; removing it again has nothing to remove,
    // whether the editor started from its removal or from nothing, and a
    // save from a revision it never had is a conflict naming the removal.
    let again_from_removal = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            "copy",
            Edit::Delete,
            ExpectedHead::At(2),
        )
        .await,
    );
    assert_eq!(codes(&again_from_removal), vec![Code::NothingToRemove]);
    let outcome = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Widget,
        "copy",
        Edit::Upsert(&copy),
        ExpectedHead::At(1),
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
    assert_eq!(codes(&again), vec![Code::NothingToRemove]);

    // An editor who saw the removal started from it, as one who saw nothing
    // did: either brings the key back.
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
                ExpectedHead::At(2)
            )
            .await
        ),
        3
    );
    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                "copy",
                Edit::Delete,
                ExpectedHead::At(3)
            )
            .await
        ),
        4
    );
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
        5
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
            (5, DocumentChange::Upsert),
            (4, DocumentChange::Delete),
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
            assert_eq!(codes(&report), vec![Code::NoDashboard])
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

    // Saved again as it is, it is refused, not taken as unchanged.
    let checks = Checks::default();
    let again = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            "stale",
            Edit::Upsert(stale),
            ExpectedHead::At(1),
        )
        .await,
    );
    assert!(!again.is_accepted(), "{again:?}");

    // The stale document does not block editing another.
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

/// Starts a save of a widget on a connection of its own.
fn spawn_edit(
    pool: &Pool,
    checks: Arc<Checks>,
    audit: Arc<Audits>,
    event: EventRef,
    key: String,
    text: String,
    expected: ExpectedHead,
) -> tokio::task::JoinHandle<Result<SaveOutcome, SaveError>> {
    spawn_change(
        pool,
        checks,
        audit,
        event,
        ConfigKind::Widget,
        key,
        Some(text),
        expected,
    )
}

/// Starts a save on a connection of its own: of `text`, or a removal.
fn spawn_change(
    pool: &Pool,
    checks: Arc<Checks>,
    audit: Arc<Audits>,
    event: EventRef,
    kind: ConfigKind,
    key: String,
    text: Option<String>,
    expected: ExpectedHead,
) -> tokio::task::JoinHandle<Result<SaveOutcome, SaveError>> {
    let pool = pool.clone();
    tokio::spawn(async move {
        let mut client = pool.get().await.unwrap();
        let change = match &text {
            Some(text) => Edit::Upsert(text),
            None => Edit::Delete,
        };
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            kind,
            &key,
            change,
            expected,
        )
        .await
    })
}

/// A save held in its checks, and what releases it.
struct Held {
    checks: Arc<Checks>,
    release: Arc<Notify>,
    save: tokio::task::JoinHandle<Result<SaveOutcome, SaveError>>,
}

async fn hold_a_save(
    pool: &Pool,
    audit: &Arc<Audits>,
    event: EventRef,
    key: &str,
    text: String,
) -> Held {
    hold_a_change(pool, audit, event, ConfigKind::Widget, key, Some(text)).await
}

/// A save from revision 1, of `text` or a removal, held in its checks.
async fn hold_a_change(
    pool: &Pool,
    audit: &Arc<Audits>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
    text: Option<String>,
) -> Held {
    let (entered, release) = (Arc::new(Notify::new()), Arc::new(Notify::new()));
    let checks = Arc::new(Checks {
        hold: Some((entered.clone(), release.clone())),
        ..Checks::default()
    });
    let save = spawn_change(
        pool,
        checks.clone(),
        audit.clone(),
        event,
        kind,
        key.to_string(),
        text,
        ExpectedHead::At(1),
    );
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .expect("the save reaches its checks");
    Held {
        checks,
        release,
        save,
    }
}

/// Waits for a change that nothing should hold back.
async fn promptly<T>(change: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), change)
        .await
        .expect("nothing holds the change back")
}

#[tokio::test]
async fn a_save_held_in_its_checks_holds_back_no_one_and_then_conflicts() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let audit = Arc::new(Audits::default());
    let campus = preset("campus");
    reset(
        &mut pool.get().await.unwrap(),
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    let (widget, yaml) = first(&campus, ConfigKind::Widget);

    let held = hold_a_save(&pool, &audit, event, &widget, format!("{yaml}\n# first\n")).await;
    let free = Arc::new(Checks::default());
    let second = promptly(spawn_edit(
        &pool,
        free.clone(),
        audit.clone(),
        event,
        widget.clone(),
        format!("{yaml}\n# second\n"),
        ExpectedHead::At(1),
    ))
    .await
    .unwrap();
    assert_eq!(saved(second), 2, "the save that finished first is kept");
    let mut client = pool.get().await.unwrap();
    assert!(
        matches!(
            promptly(set_mode(
                &mut client,
                &*audit,
                event,
                &editor(),
                DashboardMode::Legacy
            ))
            .await,
            Ok(ModeOutcome::Switched { .. })
        ),
        "nor does it hold back a switch"
    );

    held.release.notify_one();
    let first_save = held.save.await.unwrap();
    assert!(
        matches!(&first_save, Err(SaveError::Conflict { current: Some(current) }) if current.revision == 2),
        "{first_save:?}"
    );
    assert_eq!(
        held.checks.calls.load(Ordering::SeqCst),
        1,
        "the conflict is found before the renderer is asked again"
    );
    assert_eq!(audit.entries().len(), 3);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_save_is_checked_again_when_another_change_came_first() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let audit = Arc::new(Audits::default());
    let campus = preset("campus");
    reset(
        &mut pool.get().await.unwrap(),
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    let widgets: Vec<&str> = campus
        .documents
        .iter()
        .filter(|document| document.kind == ConfigKind::Widget)
        .map(|document| document.key.as_str())
        .collect();
    let (mine, theirs) = (widgets[0], widgets[1]);
    let text = |key: &str| {
        let yaml = campus
            .documents
            .iter()
            .find(|document| document.key == key)
            .unwrap()
            .yaml;
        format!("{yaml}\n# edited\n")
    };

    let renamed = text(theirs).replacen("\ntitle: ", "\ntitle: Renamed ", 1);
    assert_ne!(renamed, text(theirs));

    let held = hold_a_save(&pool, &audit, event, mine, text(mine)).await;
    let other = promptly(spawn_edit(
        &pool,
        Arc::new(Checks::default()),
        audit.clone(),
        event,
        theirs.to_string(),
        renamed,
        ExpectedHead::At(1),
    ))
    .await
    .unwrap();
    assert_eq!(saved(other), 2);

    held.release.notify_one();
    let outcome = held.save.await.unwrap();
    let Ok(SaveOutcome::Saved { revision, .. }) = outcome else {
        panic!("expected the save kept, got {outcome:?}");
    };
    assert_eq!(
        (revision.revision, revision.config_generation),
        (2, 3),
        "saved on top of the other change"
    );
    assert_eq!(
        held.checks.calls.load(Ordering::SeqCst),
        2,
        "and checked again"
    );
    let sets = held.checks.sets.lock().unwrap().clone();
    let title = |set: &ConfigSet| set.widgets[theirs].title.clone();
    assert_eq!(
        title(&sets[0]),
        title(&campus.set),
        "first against the reset"
    );
    assert_ne!(
        title(&sets[1]),
        title(&campus.set),
        "then against what the other change left"
    );
    let entries = audit.entries();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.generation)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_save_that_is_overtaken_each_time_it_is_checked_gives_up_busy() {
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
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    let (_, revisions, heads) = stored(&pool, event).await;
    let overtaken = Checks {
        move_on: Some(pool.clone()),
        ..Checks::default()
    };

    let outcome = edit(
        &mut client,
        &overtaken,
        &audit,
        event,
        ConfigKind::Widget,
        &widget,
        Edit::Upsert(&format!("{yaml}\n# edited\n")),
        ExpectedHead::At(1),
    )
    .await;
    assert!(matches!(outcome, Err(SaveError::Busy)), "{outcome:?}");
    assert_eq!(overtaken.calls.load(Ordering::SeqCst), 3, "three attempts");
    assert_eq!(
        stored(&pool, event).await,
        (Some(4), revisions, heads),
        "each moved the event on; the save wrote nothing"
    );
    assert_eq!(audit.entries().len(), 1, "and recorded nothing");
    assert_eq!(audit.prepared().len(), 2, "the save was prepared once");
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_save_overtaken_by_a_reset_conflicts_with_what_the_reset_left() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let audit = Arc::new(Audits::default());
    let campus = preset("campus");
    reset(
        &mut pool.get().await.unwrap(),
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    let (widget, yaml) = first(&campus, ConfigKind::Widget);

    let held = hold_a_save(&pool, &audit, event, &widget, format!("{yaml}\n# mine\n")).await;
    let mut client = pool.get().await.unwrap();
    let outcome = promptly(reset(
        &mut client,
        &audit,
        event,
        "comelec",
        DashboardMode::Configured,
    ))
    .await;
    assert!(
        matches!(outcome, Ok(ResetOutcome::Reset { generation: 2, .. })),
        "{outcome:?}"
    );

    held.release.notify_one();
    let mine = held.save.await.unwrap();
    assert!(
        matches!(&mine, Err(SaveError::Conflict { current: Some(current) })
            if current.revision == 2 && current.origin == RevisionOrigin::Preset),
        "{mine:?}"
    );
    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(live.assembled.set, preset("comelec").set);
    tx.rollback().await.unwrap();
    remove(&pool, event).await;
}

#[tokio::test]
async fn two_first_resets_at_once_configure_the_event_once() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let audit = Arc::new(Audits::default());
    let start = || {
        let (pool, audit) = (pool.clone(), audit.clone());
        tokio::spawn(async move {
            let mut client = pool.get().await.unwrap();
            reset(
                &mut client,
                &audit,
                event,
                "campus",
                DashboardMode::Configured,
            )
            .await
        })
    };
    let (one, other) = (start(), start());
    let outcomes = [one.await.unwrap(), other.await.unwrap()];
    let reset_count = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, Ok(ResetOutcome::Reset { generation: 1, .. })))
        .count();
    let unchanged = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, Ok(ResetOutcome::Unchanged)))
        .count();
    assert_eq!((reset_count, unchanged), (1, 1), "{outcomes:?}");
    let documents = preset("campus").documents.len() as i64;
    assert_eq!(stored(&pool, event).await, (Some(1), documents, documents));
    assert_eq!(audit.entries().len(), 1);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_save_may_not_leave_the_configured_dashboard_tab_without_a_dashboard() {
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
    let dashboards: Vec<String> = campus.set.dashboards.keys().cloned().collect();
    let (last, others) = dashboards.split_last().expect("campus has dashboards");
    assert!(!others.is_empty(), "campus has more than one dashboard");
    for dashboard in others {
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Dashboard,
                dashboard,
                Edit::Delete,
                ExpectedHead::At(1),
            )
            .await,
        );
    }
    let before = stored(&pool, event).await;
    let calls = checks.calls.load(Ordering::SeqCst);

    let refused_last = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Dashboard,
            last,
            Edit::Delete,
            ExpectedHead::At(1),
        )
        .await,
    );
    assert_eq!(codes(&refused_last), vec![Code::NoDashboard]);
    assert_eq!(stored(&pool, event).await, before);
    assert_eq!(checks.calls.load(Ordering::SeqCst), calls);

    // On the standard dashboard, the last one may go; the tab then cannot
    // be switched back until there is one.
    set_mode(&mut client, &audit, event, &editor(), DashboardMode::Legacy)
        .await
        .unwrap();
    saved(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Dashboard,
            last,
            Edit::Delete,
            ExpectedHead::At(1),
        )
        .await,
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
        matches!(&switched, Err(ModeError::Invalid(report)) if codes(report) == vec![Code::NoDashboard]),
        "{switched:?}"
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn the_configuration_an_event_had_at_a_generation_is_read_back() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());
    let campus = preset("campus");
    reset(&mut client, &audit, event, "campus", DashboardMode::Legacy)
        .await
        .unwrap();
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    let edited = format!("{yaml}\n# edited\n");
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
        .await,
    );
    let (dashboard, _) = first(&campus, ConfigKind::Dashboard);
    saved(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Dashboard,
            &dashboard,
            Edit::Delete,
            ExpectedHead::At(1),
        )
        .await,
    );

    let tx = client.transaction().await.unwrap();
    let at = |generation| get_config_at_generation(&tx, event, generation);
    let none = at(0).await.unwrap().unwrap();
    assert!(none.documents.is_empty(), "before the first reset");
    let first_reset = at(1).await.unwrap().unwrap();
    assert_eq!(first_reset.generation, 1);
    assert_eq!(first_reset.assembled.set, campus.set);
    assert!(first_reset.documents.iter().all(|d| d.revision == 1));
    let second = at(2).await.unwrap().unwrap();
    let saved_widget = second.documents.iter().find(|d| d.key == widget).unwrap();
    assert_eq!(
        (saved_widget.revision, saved_widget.yaml.as_deref()),
        (2, Some(edited.as_str()))
    );
    assert!(second.documents.iter().any(|d| d.key == dashboard));
    let third = at(3).await.unwrap().unwrap();
    assert!(
        third.documents.iter().all(|d| d.key != dashboard),
        "a removed document is left out"
    );
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(
        (third.documents, third.assembled),
        (live.documents, live.assembled),
        "the last generation is the live configuration"
    );
    assert!(at(4).await.unwrap().is_none(), "a generation not reached");
    assert!(at(-1).await.unwrap().is_none(), "nor one before any");
    let elsewhere = EventRef {
        election_event_id: id(line!(), 9),
        ..event
    };
    assert!(get_config_at_generation(&tx, elsewhere, 1)
        .await
        .unwrap()
        .is_none());
    tx.rollback().await.unwrap();
    remove(&pool, event).await;
}

#[tokio::test]
async fn each_change_is_prepared_before_it_starts_and_checked_before_it_is_recorded() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let checks = Checks::default();
    let audit = Audits::default();
    audit.probe.store(true, Ordering::SeqCst);
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
    saved(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Widget,
            &widget,
            Edit::Upsert(&format!("{yaml}\n# edited\n")),
            ExpectedHead::At(1),
        )
        .await,
    );
    set_mode(&mut client, &audit, event, &editor(), DashboardMode::Legacy)
        .await
        .unwrap();

    assert_eq!(
        *audit.calls.lock().unwrap(),
        vec!["prepare", "record", "prepare", "record", "prepare", "record"]
    );
    assert_eq!(
        audit.prepared(),
        vec![
            ("admin-1".to_string(), None),
            ("admin-1".to_string(), Some(1)),
            ("admin-1".to_string(), Some(2)),
        ],
        "each prepared before its change started"
    );
    assert_eq!(
        *audit.immediate.lock().unwrap(),
        vec![true, true, true],
        "and the deferred checks made before the log was asked to record it"
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_change_by_no_one_is_refused() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());
    reset(&mut client, &audit, event, "campus", DashboardMode::Legacy)
        .await
        .unwrap();
    let before = stored(&pool, event).await;
    let (widget, yaml) = first(&preset("campus"), ConfigKind::Widget);
    let no_one = Author {
        id: " \t".to_string(),
        name: Some("Nobody".to_string()),
    };
    let edited = format!("{yaml}\n# edited\n");

    let saved_by_no_one = save(
        &mut client,
        &checks,
        &audit,
        event,
        &no_one,
        DocumentEdit {
            kind: ConfigKind::Widget,
            key: &widget,
            edit: Edit::Upsert(&edited),
            expected: ExpectedHead::At(1),
        },
    )
    .await;
    assert!(
        matches!(&saved_by_no_one, Err(SaveError::Invalid(report)) if codes(report) == vec![Code::InvalidValue]),
        "{saved_by_no_one:?}"
    );
    let reset_by_no_one = reset_to_preset(
        &mut client,
        &audit,
        event,
        &no_one,
        "comelec",
        DashboardMode::Legacy,
    )
    .await;
    assert!(
        matches!(&reset_by_no_one, Err(ResetError::Invalid(report)) if codes(report) == vec![Code::InvalidValue]),
        "{reset_by_no_one:?}"
    );
    let switched_by_no_one = set_mode(
        &mut client,
        &audit,
        event,
        &no_one,
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(&switched_by_no_one, Err(ModeError::Invalid(report)) if codes(report) == vec![Code::InvalidValue]),
        "{switched_by_no_one:?}"
    );

    assert_eq!(stored(&pool, event).await, before);
    assert_eq!(audit.prepared().len(), 1, "only the first reset");
    assert_eq!(checks.calls.load(Ordering::SeqCst), 0);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_reset_of_an_event_that_does_not_exist_is_not_found() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let audit = Audits::default();
    let nowhere = EventRef {
        tenant_id: id(line!(), 1),
        election_event_id: id(line!(), 2),
    };
    let outcome = reset(
        &mut client,
        &audit,
        nowhere,
        "campus",
        DashboardMode::Configured,
    )
    .await;
    assert!(matches!(outcome, Err(ResetError::NotFound)), "{outcome:?}");
    assert_eq!(stored(&pool, nowhere).await, (None, 0, 0));
    assert!(audit.prepared().is_empty());
}

#[tokio::test]
async fn a_reset_to_the_preset_the_event_has_still_sets_its_mode_and_version() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let audit = Audits::default();
    let campus = preset("campus");
    let version = campus.manifest.version as i32;
    reset(&mut client, &audit, event, "campus", DashboardMode::Legacy)
        .await
        .unwrap();
    let preset_of = |live: &windmill::services::monitoring::config_store::LiveConfig| {
        live.preset.as_ref().map(|p| (p.id.clone(), p.version))
    };

    // The same documents, in another mode.
    let outcome = reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(&outcome, Ok(ResetOutcome::Reset { generation: 2, revisions, .. }) if revisions.is_empty()),
        "{outcome:?}"
    );
    // The switch keeps the preset.
    set_mode(&mut client, &audit, event, &editor(), DashboardMode::Legacy)
        .await
        .unwrap();
    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(
        (live.mode, preset_of(&live)),
        (DashboardMode::Legacy, Some(("campus".to_string(), version)))
    );
    tx.rollback().await.unwrap();

    // The same documents, from another version of the preset.
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.monitoring_event
         SET config_generation = config_generation + 1, preset_version = preset_version + 100
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[&event.tenant_id, &event.election_event_id],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let outcome = reset(&mut client, &audit, event, "campus", DashboardMode::Legacy).await;
    assert!(
        matches!(&outcome, Ok(ResetOutcome::Reset { generation: 5, revisions, .. }) if revisions.is_empty()),
        "{outcome:?}"
    );
    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(preset_of(&live), Some(("campus".to_string(), version)));
    tx.rollback().await.unwrap();
    let entries = audit.entries();
    assert_eq!(entries.len(), 4);
    assert_eq!(
        entries
            .last()
            .and_then(|entry| entry.preset.as_ref())
            .map(|p| p.version),
        Some(version)
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_reset_brings_back_what_an_earlier_one_removed() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let audit = Audits::default();
    let (campus, comelec) = (preset("campus"), preset("comelec"));
    for preset_id in ["campus", "comelec", "campus"] {
        reset(
            &mut client,
            &audit,
            event,
            preset_id,
            DashboardMode::Configured,
        )
        .await
        .unwrap();
    }
    let only_campus = campus
        .documents
        .iter()
        .find(|document| {
            !comelec
                .documents
                .iter()
                .any(|other| other.kind == document.kind && other.key == document.key)
        })
        .expect("campus has a document comelec lacks");

    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert_eq!(live.assembled.set, campus.set);
    let kept = history(&tx, event, only_campus.kind, &only_campus.key)
        .await
        .unwrap();
    assert_eq!(
        kept.iter()
            .map(|revision| (
                revision.revision,
                revision.change,
                revision.config_generation
            ))
            .collect::<Vec<_>>(),
        vec![
            (3, DocumentChange::Upsert, 3),
            (2, DocumentChange::Delete, 2),
            (1, DocumentChange::Upsert, 1)
        ]
    );
    assert_eq!(kept[0].yaml.as_deref(), Some(only_campus.yaml));
    tx.rollback().await.unwrap();
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_save_returns_what_is_warned_of_and_is_refused_only_for_errors() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());
    reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    assert!(preset("campus").set.widgets.contains_key("polls"));
    // Polls have no country: a dashboard of them alone that offers one warns.
    let unused = "id: poll-watch\ntitle: Polls\nselectors: [region, post, country]\nlayout: [{widget: polls, width: 12}]\n";
    let warning = Problem::warning(Code::ChartSchema, "chart", "labels overlap")
        .with_engine_code("WARN-LABELS");
    let error =
        Problem::error(Code::ChartSchema, "chart", "unknown mark").with_engine_code("ERR-MARK");

    *checks.problems.lock().unwrap() = Report {
        problems: vec![warning.clone(), error.clone()],
    };
    let mixed = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Dashboard,
            "poll-watch",
            Edit::Upsert(unused),
            ExpectedHead::Absent,
        )
        .await,
    );
    assert_eq!(mixed.problems, vec![error], "only what refuses it");

    *checks.problems.lock().unwrap() = Report {
        problems: vec![warning.clone()],
    };
    let outcome = edit(
        &mut client,
        &checks,
        &audit,
        event,
        ConfigKind::Dashboard,
        "poll-watch",
        Edit::Upsert(unused),
        ExpectedHead::Absent,
    )
    .await;
    let Ok(SaveOutcome::Saved { warnings, .. }) = outcome else {
        panic!("expected the save kept, got {outcome:?}");
    };
    assert_eq!(
        codes(&warnings),
        vec![Code::UnusedSelector, Code::ChartSchema],
        "the policy's warnings, then the engine's"
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn two_events_with_the_same_keys_are_kept_apart() {
    let pool = schema::pool().await;
    let one = event(&pool, line!()).await;
    let other = event(&pool, line!() + 100_000).await;
    let mut client = pool.get().await.unwrap();
    let (checks, audit) = (Checks::default(), Audits::default());
    let campus = preset("campus");
    for event in [one, other] {
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
    saved(
        edit(
            &mut client,
            &checks,
            &audit,
            one,
            ConfigKind::Widget,
            &widget,
            Edit::Upsert(&format!("{yaml}\n# edited\n")),
            ExpectedHead::At(1),
        )
        .await,
    );
    let documents = campus.documents.len() as i64;
    assert_eq!(
        stored(&pool, one).await,
        (Some(2), documents + 1, documents)
    );
    assert_eq!(stored(&pool, other).await, (Some(1), documents, documents));
    // The other event's document is still at the revision it was reset to.
    let from_other = edit(
        &mut client,
        &checks,
        &audit,
        other,
        ConfigKind::Widget,
        &widget,
        Edit::Upsert(&format!("{yaml}\n# edited\n")),
        ExpectedHead::At(2),
    )
    .await;
    assert!(
        matches!(&from_other, Err(SaveError::Conflict { current: Some(current) }) if current.revision == 1),
        "{from_other:?}"
    );
    remove(&pool, one).await;
    remove(&pool, other).await;
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

/// Writes `yaml` as revision `revision` of a document through the tables
/// alone, as an older release might have, raising the event's generation.
async fn write_directly(
    pool: &Pool,
    event: EventRef,
    kind: &str,
    key: &str,
    revision: i32,
    yaml: &str,
) {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
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
        "INSERT INTO sequent_backend.monitoring_config
             (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256, origin,
              config_generation, author_id)
         VALUES ($1, $2, $3, $4, $5, 'UPSERT', $6, $7, 'EDITOR', $8, 'admin-1')",
        &[
            &event.tenant_id,
            &event.election_event_id,
            &kind,
            &key,
            &revision,
            &yaml,
            &document_digest(yaml),
            &generation,
        ],
    )
    .await
    .unwrap();
    let head = if revision == 1 {
        "INSERT INTO sequent_backend.monitoring_config_head
             (tenant_id, election_event_id, kind, key, revision)
         VALUES ($1, $2, $3, $4, $5)"
    } else {
        "UPDATE sequent_backend.monitoring_config_head SET revision = $5
         WHERE tenant_id = $1 AND election_event_id = $2 AND kind = $3 AND key = $4"
    };
    tx.execute(
        head,
        &[
            &event.tenant_id,
            &event.election_event_id,
            &kind,
            &key,
            &revision,
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn dashboards_that_no_longer_pass_do_not_block_the_configured_tab() {
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
    let dashboards: Vec<String> = campus.set.dashboards.keys().cloned().collect();
    let stale = |dashboard: &str| format!("id: {dashboard}\nunknown_field: true\n");
    let (passing, others) = dashboards.split_first().unwrap();
    for dashboard in others {
        write_directly(&pool, event, "dashboard", dashboard, 2, &stale(dashboard)).await;
    }
    // The one that passes is the last the tab shows.
    let last = refused(
        edit(
            &mut client,
            &checks,
            &audit,
            event,
            ConfigKind::Dashboard,
            passing,
            Edit::Delete,
            ExpectedHead::At(1),
        )
        .await,
    );
    assert_eq!(codes(&last), vec![Code::NoDashboard]);
    write_directly(&pool, event, "dashboard", passing, 2, &stale(passing)).await;
    let tx = client.transaction().await.unwrap();
    let live = get_live_config(&tx, event).await.unwrap().unwrap();
    assert!(live.assembled.set.dashboards.is_empty(), "each is left out");
    assert_eq!(live.mode, DashboardMode::Configured);
    tx.rollback().await.unwrap();

    // The tab shows no dashboard already, so a save takes none away.
    let (widget, yaml) = first(&campus, ConfigKind::Widget);
    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Widget,
                &widget,
                Edit::Upsert(&format!("{yaml}\n# edited\n")),
                ExpectedHead::At(1)
            )
            .await
        ),
        2
    );
    assert_eq!(
        saved(
            edit(
                &mut client,
                &checks,
                &audit,
                event,
                ConfigKind::Dashboard,
                &dashboards[0],
                Edit::Delete,
                ExpectedHead::At(2)
            )
            .await
        ),
        3,
        "nor does removing one of them"
    );
    // But the tab is not switched back to show none.
    set_mode(&mut client, &audit, event, &editor(), DashboardMode::Legacy)
        .await
        .unwrap();
    let switched = set_mode(
        &mut client,
        &audit,
        event,
        &editor(),
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(&switched, Err(ModeError::Invalid(report)) if codes(report) == vec![Code::NoDashboard]),
        "{switched:?}"
    );
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_removal_checked_before_the_tab_was_switched_may_not_take_its_last_dashboard() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let audit = Arc::new(Audits::default());
    let campus = preset("campus");
    reset(&mut client, &audit, event, "campus", DashboardMode::Legacy)
        .await
        .unwrap();
    let dashboards: Vec<String> = campus.set.dashboards.keys().cloned().collect();
    let (last, others) = dashboards.split_last().unwrap();
    for dashboard in others {
        saved(
            edit(
                &mut client,
                &Checks::default(),
                &audit,
                event,
                ConfigKind::Dashboard,
                dashboard,
                Edit::Delete,
                ExpectedHead::At(1),
            )
            .await,
        );
    }

    let held = hold_a_change(&pool, &audit, event, ConfigKind::Dashboard, last, None).await;
    let switched = promptly(set_mode(
        &mut client,
        &*audit,
        event,
        &editor(),
        DashboardMode::Configured,
    ))
    .await;
    assert!(
        matches!(switched, Ok(ModeOutcome::Switched { .. })),
        "{switched:?}"
    );
    let before = stored(&pool, event).await;

    held.release.notify_one();
    let removal = refused(held.save.await.unwrap());
    assert_eq!(codes(&removal), vec![Code::NoDashboard]);
    assert_eq!(stored(&pool, event).await, before, "and wrote nothing");
    assert_eq!(held.checks.calls.load(Ordering::SeqCst), 1);
    remove(&pool, event).await;
}

/// How many preparations `Audits::mark` left for the event, committed.
async fn prepared_rows(client: &Client, event: EventRef) -> i64 {
    client
        .query_one(
            "SELECT count(*) FROM prepared_for WHERE election_event_id = $1",
            &[&event.election_event_id],
        )
        .await
        .unwrap()
        .get(0)
}

#[tokio::test]
async fn what_a_change_prepared_is_kept_when_the_change_fails() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    client
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS prepared_for (election_event_id uuid, author_id text)",
        )
        .await
        .unwrap();
    let audit = Audits::default();
    audit.mark.store(true, Ordering::SeqCst);
    audit.fail.store(true, Ordering::SeqCst);

    let outcome = reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await;
    assert!(
        matches!(outcome, Err(ResetError::Internal(_))),
        "{outcome:?}"
    );
    assert_eq!(stored(&pool, event).await, (None, 0, 0));
    assert_eq!(prepared_rows(&client, event).await, 1);

    audit.fail.store(false, Ordering::SeqCst);
    reset(
        &mut client,
        &audit,
        event,
        "campus",
        DashboardMode::Configured,
    )
    .await
    .unwrap();
    audit.fail.store(true, Ordering::SeqCst);
    let (widget, yaml) = first(&preset("campus"), ConfigKind::Widget);
    let outcome = edit(
        &mut client,
        &Checks::default(),
        &audit,
        event,
        ConfigKind::Widget,
        &widget,
        Edit::Upsert(&format!("{yaml}\n# edited\n")),
        ExpectedHead::At(1),
    )
    .await;
    assert!(
        matches!(outcome, Err(SaveError::Internal(_))),
        "{outcome:?}"
    );
    assert_eq!(prepared_rows(&client, event).await, 3);
    remove(&pool, event).await;
}

#[tokio::test]
async fn a_save_sent_again_after_it_was_kept_is_unchanged() {
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
    let before = stored(&pool, event).await;

    // Its answer was lost, so the editor sends it again from where it
    // started: the document already says it.
    let again = edit(
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
        matches!(&again, Ok(SaveOutcome::Unchanged(head)) if head.revision == 2),
        "{again:?}"
    );
    assert_eq!(stored(&pool, event).await, before);
    assert_eq!(audit.entries().len(), 2);
    remove(&pool, event).await;
}

#[tokio::test]
async fn an_administrator_signing_key_is_found_without_being_read() {
    let pool = schema::pool().await;
    let event = event(&pool, line!()).await;
    let mut client = pool.get().await.unwrap();
    let tenant = event.tenant_id.to_string();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.secret (tenant_id, key, value) VALUES ($1, $2, '\\x00')",
        &[
            &event.tenant_id,
            &format!("admin_signing_key-{tenant}-admin-1"),
        ],
    )
    .await
    .unwrap();
    assert!(admin_user_signing_key_exists(&tx, &tenant, "admin-1")
        .await
        .unwrap());
    assert!(!admin_user_signing_key_exists(&tx, &tenant, "admin-2")
        .await
        .unwrap());
    tx.rollback().await.unwrap();
    remove(&pool, event).await;
}
