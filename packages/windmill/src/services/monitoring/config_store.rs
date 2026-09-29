// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An election event's monitoring configuration: what it has, and each
//! change to it (a save of one document, a reset to a preset, a switch of
//! the Dashboard tab) as one transaction.
//!
//! A change runs under READ COMMITTED and raises the event's generation
//! before it reads anything it writes from (a reset first holds the
//! election event, so it cannot be deleted meanwhile, and makes its
//! monitoring row if there is none). Raising the generation holds the
//! event's row until the change ends, so the changes of one event queue
//! behind one another and each reads what the one before it left. It then
//! moves the heads, writes the revisions, makes the checks the tables defer
//! to commit, and has the electoral log record the change before it
//! commits. Anything refused on the way writes nothing, and
//! nothing reaches the log. A commit that fails after the log recorded the
//! change leaves an entry for a generation the event never reached; the next
//! change takes that generation again, and its entry says so.
//!
//! A save is checked before its transaction starts, against the event as
//! one statement read it, so a slow chart engine holds back no one. The
//! transaction then only writes if the generation it raises is the next
//! after the one the checks read, meaning nothing changed meanwhile;
//! otherwise the save is checked again against what the other change left.
//!
//! The electoral log is prepared for the author (their signing key made or
//! read) before a change's transaction starts, so what it keeps of that
//! does not depend on the change committing.

use crate::postgres::monitoring_config::{
    advance_head, check_deferred_constraints, create_head, create_monitoring_event,
    election_event_exists, get_event_and_heads, get_event_and_revisions_at, get_head, get_heads,
    get_revision, get_revisions, insert_revision, lock_election_event, raise_generation,
    update_mode_and_preset, MonitoringEvent, NewRevision,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use deadpool_postgres::{Client, Transaction};
use sequent_core::monitoring::config::{ConfigKind, ConfigSet};
use sequent_core::monitoring::presets;
use sequent_core::monitoring::problem::{Code, Problem, Report, Severity};
use sequent_core::monitoring::revision::{
    assemble, check_edit, check_target, document_digest, plan_reset, Assembled, DashboardMode,
    DocumentChange, Edit, LiveDocument, LiveHead, RevisionOrigin,
};
use tokio_postgres::IsolationLevel;
use tracing::{instrument, warn};

pub use crate::postgres::monitoring_config::{Author, EventRef, PresetVersion, StoredRevision};

/// How many times a save is checked against an event that other changes
/// keep moving on before it gives up.
const SAVE_ATTEMPTS: usize = 3;

/// The revision of a document an editor started from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedHead {
    /// The document does not exist: it was never saved, or was removed.
    Absent,
    /// The document is at this revision.
    At(i32),
}

/// One document an administrator saves or removes.
#[derive(Debug, Clone, Copy)]
pub struct DocumentEdit<'a> {
    pub kind: ConfigKind,
    pub key: &'a str,
    pub edit: Edit<'a>,
    pub expected: ExpectedHead,
}

/// What an event is configured with.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveConfig {
    pub mode: DashboardMode,
    /// The preset the configuration was last reset to.
    pub preset: Option<PresetVersion>,
    pub generation: i64,
    /// The head revision of each document the event has, removed ones left
    /// out: settings, themes, widgets, dashboards; each by key.
    pub documents: Vec<StoredRevision>,
    /// The documents as one set, leaving out, and saying why, any that no
    /// longer pass the checks they were saved under.
    pub assembled: Assembled,
}

/// What an event was configured with once a generation was reached: its
/// documents only. The tables keep no history of the Dashboard tab's mode or
/// of the preset, so neither is part of it.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigAtGeneration {
    pub generation: i64,
    /// The revision of each document the event then had, removed ones left
    /// out, in the order of [`LiveConfig::documents`].
    pub documents: Vec<StoredRevision>,
    /// The documents as one set, as [`LiveConfig::assembled`] would have
    /// said then, by today's checks.
    pub assembled: Assembled,
}

/// A revision a change wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenRevision {
    pub kind: ConfigKind,
    pub key: String,
    pub revision: i32,
    pub change: DocumentChange,
    /// The digest of the document; absent for a removal.
    pub digest: Option<String>,
}

impl From<&StoredRevision> for WrittenRevision {
    fn from(revision: &StoredRevision) -> Self {
        WrittenRevision {
            kind: revision.kind,
            key: revision.key.clone(),
            revision: revision.revision,
            change: revision.change,
            digest: revision.sha256.clone(),
        }
    }
}

/// A change, as the electoral log records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedChange {
    pub event: EventRef,
    pub author: Author,
    pub origin: RevisionOrigin,
    /// The preset a reset wrote.
    pub preset: Option<PresetVersion>,
    /// The Dashboard tab's mode once the change is saved.
    pub mode: DashboardMode,
    /// The event's generation once the change is saved.
    pub generation: i64,
    /// The revisions written, in the order they were stored.
    pub revisions: Vec<WrittenRevision>,
}

/// Records each change before it commits. A change it cannot record is not
/// saved.
#[async_trait]
pub trait MonitoringConfigAudit: Send + Sync {
    /// Readies the log for a change by `author`, before the change's
    /// transaction starts: whatever it writes is its own to commit, and is
    /// kept whether or not the change is. A change it cannot prepare for is
    /// not made.
    async fn prepare(
        &self,
        client: &mut Client,
        event: EventRef,
        author: &Author,
    ) -> anyhow::Result<()>;

    async fn record(
        &self,
        transaction: &Transaction<'_>,
        change: &RecordedChange,
    ) -> anyhow::Result<()>;
}

/// An edit that passed the policy, as the chart engine is asked about it.
#[derive(Debug, Clone, Copy)]
pub struct ProposedChange<'a> {
    pub event: EventRef,
    pub kind: ConfigKind,
    pub key: &'a str,
    pub edit: Edit<'a>,
    /// The event's documents once the edit is saved.
    pub set: &'a ConfigSet,
}

/// The checks only the chart engine can make: that each chart it would draw
/// passes its schema, and draws. Errors refuse the save; warnings come back
/// with it.
#[async_trait]
pub trait MonitoringConfigChecks: Send + Sync {
    async fn check(&self, change: &ProposedChange<'_>) -> anyhow::Result<Report>;
}

#[derive(Debug)]
pub enum SaveOutcome {
    Saved {
        revision: StoredRevision,
        /// What the policy and the chart engine warn of.
        warnings: Report,
    },
    /// The document already is this text; nothing was written.
    Unchanged(StoredRevision),
}

#[derive(Debug)]
pub enum SaveError {
    /// The event has no monitoring configuration to change.
    NotConfigured,
    /// The document is not at the revision the editor started from.
    Conflict {
        /// Its head: absent for a document never saved.
        current: Option<StoredRevision>,
    },
    /// Why the edit may not be saved.
    Invalid(Report),
    /// The chart engine could not be asked; nothing was written.
    ChecksUnavailable(anyhow::Error),
    /// Other changes of the event kept coming first; nothing was written,
    /// and the same save may be tried again.
    Busy,
    Internal(anyhow::Error),
}

#[derive(Debug)]
pub enum ResetOutcome {
    Reset {
        generation: i64,
        revisions: Vec<WrittenRevision>,
        /// What the preset's own checks warn of.
        warnings: Report,
    },
    /// The event already has the preset's documents, preset and mode.
    Unchanged,
}

#[derive(Debug)]
pub enum ResetError {
    /// There is no such election event.
    NotFound,
    UnknownPreset,
    Invalid(Report),
    Internal(anyhow::Error),
}

#[derive(Debug)]
pub enum ModeOutcome {
    Switched { generation: i64 },
    Unchanged,
}

#[derive(Debug)]
pub enum ModeError {
    NotConfigured,
    Invalid(Report),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for SaveError {
    fn from(error: anyhow::Error) -> Self {
        SaveError::Internal(error)
    }
}

impl From<anyhow::Error> for ResetError {
    fn from(error: anyhow::Error) -> Self {
        ResetError::Internal(error)
    }
}

impl From<anyhow::Error> for ModeError {
    fn from(error: anyhow::Error) -> Self {
        ModeError::Internal(error)
    }
}

/// The event's documents that exist, and the set they amount to.
fn live_documents(heads: Vec<StoredRevision>) -> (Vec<StoredRevision>, Assembled) {
    let documents: Vec<StoredRevision> = heads
        .into_iter()
        .filter(|head| head.change == DocumentChange::Upsert)
        .collect();
    let assembled = assemble(documents.iter().filter_map(|document| {
        document.yaml.as_deref().map(|yaml| LiveDocument {
            kind: document.kind,
            key: &document.key,
            yaml,
        })
    }));
    (documents, assembled)
}

/// Why the Dashboard tab may not show the configured dashboards.
fn nothing_to_show() -> Report {
    Report {
        problems: vec![Problem::error(
            Code::NoDashboard,
            "",
            "The event has no dashboard to show; reset it to a preset first.",
        )],
    }
}

/// Why a save may not remove the last dashboard the Dashboard tab shows.
fn last_dashboard() -> Report {
    Report {
        problems: vec![Problem::error(
            Code::NoDashboard,
            "",
            "The Dashboard tab shows the configured dashboards, and this is the last one; \
             switch the tab to the standard dashboard before removing it.",
        )],
    }
}

/// Who a change is by: the electoral log signs each entry as its author, so
/// a change by no one is not made.
fn check_author(author: &Author) -> Result<(), Report> {
    if author.id.trim().is_empty() {
        return Err(Report {
            problems: vec![Problem::error(
                Code::InvalidValue,
                "author",
                "A change must name the administrator who makes it.",
            )],
        });
    }
    Ok(())
}

fn errors_only(mut report: Report) -> Report {
    report
        .problems
        .retain(|problem| problem.severity == Severity::Error);
    report
}

/// What an event is configured with; `None` for an event on the standard
/// dashboard that was never configured. One statement reads it, so the
/// generation is the one the documents are of.
#[instrument(err, skip(transaction))]
pub async fn get_live_config(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> anyhow::Result<Option<LiveConfig>> {
    let Some((stored, heads)) = get_event_and_heads(transaction, event).await? else {
        return Ok(None);
    };
    let (documents, assembled) = live_documents(heads);
    Ok(Some(LiveConfig {
        mode: stored.mode,
        preset: stored.preset,
        generation: stored.generation,
        documents,
        assembled,
    }))
}

/// What the event was configured with once it reached `generation`; `None`
/// for an event without monitoring configuration, or one that has not
/// reached it.
#[instrument(err, skip(transaction))]
pub async fn get_config_at_generation(
    transaction: &Transaction<'_>,
    event: EventRef,
    generation: i64,
) -> anyhow::Result<Option<ConfigAtGeneration>> {
    if generation < 0 {
        return Ok(None);
    }
    let Some((stored, revisions)) =
        get_event_and_revisions_at(transaction, event, generation).await?
    else {
        return Ok(None);
    };
    if generation > stored.generation {
        return Ok(None);
    }
    let (documents, assembled) = live_documents(revisions);
    Ok(Some(ConfigAtGeneration {
        generation,
        documents,
        assembled,
    }))
}

/// Every revision of a document, newest first.
pub async fn history(
    transaction: &Transaction<'_>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
) -> anyhow::Result<Vec<StoredRevision>> {
    get_revisions(transaction, event, kind, key).await
}

/// A document at a revision, or at its head; a removal is returned as the
/// DELETE revision it is.
pub async fn get_document(
    transaction: &Transaction<'_>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
    revision: Option<i32>,
) -> anyhow::Result<Option<StoredRevision>> {
    match revision {
        Some(revision) => get_revision(transaction, event, kind, key, revision).await,
        None => get_head(transaction, event, kind, key).await,
    }
}

async fn begin(client: &mut Client) -> anyhow::Result<Transaction<'_>> {
    client
        .build_transaction()
        .isolation_level(IsolationLevel::ReadCommitted)
        .start()
        .await
        .context("Failed to start the monitoring configuration transaction")
}

/// Whether a change's transaction is to be kept.
enum Finish {
    Commit,
    Rollback,
}

/// Commits or rolls back what `result` says, and rolls back on an error.
async fn finish<T, E: From<anyhow::Error>>(
    transaction: Transaction<'_>,
    result: Result<(T, Finish), E>,
) -> Result<T, E> {
    match result {
        Ok((outcome, Finish::Commit)) => {
            transaction
                .commit()
                .await
                .context("Failed to commit the monitoring configuration change")?;
            Ok(outcome)
        }
        Ok((outcome, Finish::Rollback)) => {
            rollback(transaction).await;
            Ok(outcome)
        }
        Err(error) => {
            rollback(transaction).await;
            Err(error)
        }
    }
}

async fn rollback(transaction: Transaction<'_>) {
    if let Err(error) = transaction.rollback().await {
        warn!("Failed to roll back a monitoring configuration change: {error:?}");
    }
}

/// Moves the head of a document from `head` to the next revision, or starts
/// it: the revision to write, or `None` when the head moved meanwhile.
async fn move_head(
    transaction: &Transaction<'_>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
    head: Option<&StoredRevision>,
) -> anyhow::Result<Option<i32>> {
    Ok(match head {
        None => create_head(transaction, event, kind, key)
            .await?
            .then_some(1),
        Some(head) => advance_head(transaction, event, kind, key, head.revision)
            .await?
            .then_some(head.revision + 1),
    })
}

/// What a save was checked against, and what it would write.
struct CheckedSave {
    /// The generation the checks read.
    generation: i64,
    mode: DashboardMode,
    /// The document's head then.
    head: Option<StoredRevision>,
    warnings: Report,
}

enum Checked {
    /// The document already is this text; nothing is to be written.
    Unchanged(StoredRevision),
    Save(CheckedSave),
}

/// Saves or removes one document.
#[instrument(
    err(Debug, level = "warn"),
    skip(client, checks, audit, author, edit),
    fields(author = %author.id, kind = %edit.kind, key = edit.key, expected = ?edit.expected)
)]
pub async fn save(
    client: &mut Client,
    checks: &dyn MonitoringConfigChecks,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    edit: DocumentEdit<'_>,
) -> Result<SaveOutcome, SaveError> {
    check_author(author).map_err(SaveError::Invalid)?;
    check_target(edit.kind, edit.key).map_err(SaveError::Invalid)?;
    let mut prepared = false;
    for _ in 0..SAVE_ATTEMPTS {
        let checked = match check_save(client, checks, event, edit).await? {
            Checked::Unchanged(head) => return Ok(SaveOutcome::Unchanged(head)),
            Checked::Save(checked) => checked,
        };
        if !prepared {
            audit
                .prepare(client, event, author)
                .await
                .context("Failed to prepare the electoral log for the change")?;
            prepared = true;
        }
        let transaction = begin(client).await?;
        let result = write_save(&transaction, audit, event, author, edit, checked).await;
        if let Some(outcome) = finish(transaction, result).await? {
            return Ok(outcome);
        }
    }
    Err(SaveError::Busy)
}

/// Checks a save against the event as one statement reads it, outside any
/// transaction that holds the event.
async fn check_save(
    client: &mut Client,
    checks: &dyn MonitoringConfigChecks,
    event: EventRef,
    edit: DocumentEdit<'_>,
) -> Result<Checked, SaveError> {
    let DocumentEdit {
        kind,
        key,
        edit: change,
        expected,
    } = edit;
    let (stored, heads) = read(client, event).await?.ok_or(SaveError::NotConfigured)?;
    let head = heads
        .iter()
        .find(|head| head.kind == kind && head.key == key)
        .cloned();
    let exists = head
        .as_ref()
        .filter(|head| head.change == DocumentChange::Upsert);
    let started_from_head = match (expected, &head) {
        (ExpectedHead::Absent, None) => true,
        (ExpectedHead::Absent, Some(head)) => head.change == DocumentChange::Delete,
        (ExpectedHead::At(revision), Some(head)) => head.revision == revision,
        (ExpectedHead::At(_), None) => false,
    };
    if !started_from_head {
        return Err(SaveError::Conflict { current: head });
    }
    match (change, exists) {
        (Edit::Upsert(yaml), Some(head))
            if head.sha256.as_deref() == Some(document_digest(yaml).as_str()) =>
        {
            return Ok(Checked::Unchanged(head.clone()));
        }
        (Edit::Delete, None) => {
            return Err(SaveError::Invalid(Report {
                problems: vec![Problem::error(
                    Code::DanglingReference,
                    "",
                    format!("There is no {kind} '{key}' to remove."),
                )],
            }));
        }
        _ => {}
    }

    let (_, live) = live_documents(heads);
    let checked = check_edit(&live.set, kind, key, change).map_err(SaveError::Invalid)?;
    // Dashboards that no longer pass are shown by no one already, so a save
    // among them takes none away.
    if stored.mode == DashboardMode::Configured
        && !live.set.dashboards.is_empty()
        && checked.set.dashboards.is_empty()
    {
        return Err(SaveError::Invalid(last_dashboard()));
    }
    let engine = checks
        .check(&ProposedChange {
            event,
            kind,
            key,
            edit: change,
            set: &checked.set,
        })
        .await
        .map_err(|error| {
            SaveError::ChecksUnavailable(
                error.context("Failed to have the chart engine check the edit"),
            )
        })?;
    if !engine.is_accepted() {
        return Err(SaveError::Invalid(errors_only(engine)));
    }
    let mut warnings = checked.report;
    warnings.extend(engine);
    Ok(Checked::Save(CheckedSave {
        generation: stored.generation,
        mode: stored.mode,
        head,
        warnings,
    }))
}

/// The event and its heads, committed.
async fn read(
    client: &mut Client,
    event: EventRef,
) -> anyhow::Result<Option<(MonitoringEvent, Vec<StoredRevision>)>> {
    let transaction = client
        .build_transaction()
        .isolation_level(IsolationLevel::ReadCommitted)
        .read_only(true)
        .start()
        .await
        .context("Failed to start reading the monitoring configuration")?;
    let read = get_event_and_heads(&transaction, event).await?;
    transaction
        .commit()
        .await
        .context("Failed to finish reading the monitoring configuration")?;
    Ok(read)
}

/// Writes a checked save, unless the event moved on since it was checked:
/// then `None`, and nothing is written.
async fn write_save(
    transaction: &Transaction<'_>,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    edit: DocumentEdit<'_>,
    checked: CheckedSave,
) -> Result<(Option<SaveOutcome>, Finish), SaveError> {
    let DocumentEdit {
        kind,
        key,
        edit: change,
        ..
    } = edit;
    let Some(stored) = raise_generation(transaction, event).await? else {
        return Err(SaveError::NotConfigured);
    };
    // The tab is switched only with the generation raised, so the mode is
    // the checked one unless something else moved the event on: then the
    // save is checked again.
    if stored.generation != checked.generation + 1 || stored.mode != checked.mode {
        return Ok((None, Finish::Rollback));
    }
    // Nothing changed since the checks read the head, so it is where they
    // found it.
    let revision = move_head(transaction, event, kind, key, checked.head.as_ref())
        .await?
        .ok_or_else(|| anyhow!("The head of {kind} {key} moved while the generation did not"))?;
    let (document_change, yaml) = match change {
        Edit::Upsert(yaml) => (DocumentChange::Upsert, Some(yaml)),
        Edit::Delete => (DocumentChange::Delete, None),
    };
    let sha256 = yaml.map(document_digest);
    let stored_revision = insert_revision(
        transaction,
        event,
        NewRevision {
            kind,
            key,
            revision,
            change: document_change,
            yaml,
            sha256: sha256.as_deref(),
            origin: RevisionOrigin::Editor,
            preset: None,
            config_generation: stored.generation,
            author,
        },
    )
    .await?;
    check_deferred_constraints(transaction).await?;
    audit
        .record(
            transaction,
            &RecordedChange {
                event,
                author: author.clone(),
                origin: RevisionOrigin::Editor,
                preset: None,
                mode: stored.mode,
                generation: stored.generation,
                revisions: vec![WrittenRevision::from(&stored_revision)],
            },
        )
        .await
        .context("Failed to record the monitoring configuration change in the electoral log")?;
    Ok((
        Some(SaveOutcome::Saved {
            revision: stored_revision,
            warnings: checked.warnings,
        }),
        Finish::Commit,
    ))
}

/// Leaves the event with exactly the preset's documents, and the Dashboard
/// tab in `mode`. An event without monitoring configuration gets it: this is
/// how an event is first configured.
#[instrument(err(Debug, level = "warn"), skip(client, audit, author), fields(author = %author.id))]
pub async fn reset_to_preset(
    client: &mut Client,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    preset_id: &str,
    mode: DashboardMode,
) -> Result<ResetOutcome, ResetError> {
    check_author(author).map_err(ResetError::Invalid)?;
    let preset = match presets::load(preset_id) {
        None => return Err(ResetError::UnknownPreset),
        Some(Err(report)) => {
            return Err(ResetError::Internal(anyhow!(
                "The preset '{preset_id}' does not load:\n{report}"
            )))
        }
        Some(Ok(preset)) => preset,
    };
    if mode == DashboardMode::Configured && preset.set.dashboards.is_empty() {
        return Err(ResetError::Invalid(nothing_to_show()));
    }
    let version = PresetVersion {
        id: preset.manifest.id.clone(),
        version: i32::try_from(preset.manifest.version)
            .context("The preset's version does not fit the monitoring tables")?,
    };
    if !election_event_exists(&*client, event).await? {
        return Err(ResetError::NotFound);
    }
    audit
        .prepare(client, event, author)
        .await
        .context("Failed to prepare the electoral log for the reset")?;
    let transaction = begin(client).await?;
    let result = reset_in(&transaction, audit, event, author, &preset, version, mode).await;
    finish(transaction, result).await
}

async fn reset_in(
    transaction: &Transaction<'_>,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    preset: &presets::Preset,
    version: PresetVersion,
    mode: DashboardMode,
) -> Result<(ResetOutcome, Finish), ResetError> {
    if !lock_election_event(transaction, event).await? {
        return Err(ResetError::NotFound);
    }
    create_monitoring_event(transaction, event).await?;
    let stored = raise_generation(transaction, event)
        .await?
        .ok_or_else(|| anyhow!("The monitoring event was not created"))?;
    let heads = get_heads(transaction, event).await?;
    let live: Vec<LiveHead> = heads
        .iter()
        .map(|head| LiveHead {
            kind: head.kind,
            key: head.key.clone(),
            digest: head.sha256.clone(),
        })
        .collect();
    let planned = plan_reset(preset, &live);
    if planned.is_empty() && stored.mode == mode && stored.preset.as_ref() == Some(&version) {
        return Ok((ResetOutcome::Unchanged, Finish::Rollback));
    }

    let mut written = Vec::with_capacity(planned.len());
    for plan in &planned {
        let head = heads
            .iter()
            .find(|head| head.kind == plan.kind && head.key == plan.key);
        let revision = move_head(transaction, event, plan.kind, &plan.key, head)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "The head of {} {} moved during a reset",
                    plan.kind,
                    plan.key
                )
            })?;
        let sha256 = plan.yaml.map(document_digest);
        let stored_revision = insert_revision(
            transaction,
            event,
            NewRevision {
                kind: plan.kind,
                key: &plan.key,
                revision,
                change: plan.change,
                yaml: plan.yaml,
                sha256: sha256.as_deref(),
                origin: RevisionOrigin::Preset,
                preset: Some(&version),
                config_generation: stored.generation,
                author,
            },
        )
        .await?;
        written.push(WrittenRevision::from(&stored_revision));
    }
    update_mode_and_preset(transaction, event, mode, Some(&version)).await?;
    check_deferred_constraints(transaction).await?;
    audit
        .record(
            transaction,
            &RecordedChange {
                event,
                author: author.clone(),
                origin: RevisionOrigin::Preset,
                preset: Some(version),
                mode,
                generation: stored.generation,
                revisions: written.clone(),
            },
        )
        .await
        .context("Failed to record the reset in the electoral log")?;
    Ok((
        ResetOutcome::Reset {
            generation: stored.generation,
            revisions: written,
            warnings: preset.warnings.clone(),
        },
        Finish::Commit,
    ))
}

/// Switches the Dashboard tab between the standard dashboard and the
/// configured ones. The configuration is kept either way.
#[instrument(err(Debug, level = "warn"), skip(client, audit, author), fields(author = %author.id))]
pub async fn set_mode(
    client: &mut Client,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    mode: DashboardMode,
) -> Result<ModeOutcome, ModeError> {
    check_author(author).map_err(ModeError::Invalid)?;
    match read(client, event).await? {
        None => return Err(ModeError::NotConfigured),
        Some((stored, _)) if stored.mode == mode => return Ok(ModeOutcome::Unchanged),
        Some(_) => {}
    }
    audit
        .prepare(client, event, author)
        .await
        .context("Failed to prepare the electoral log for the switch")?;
    let transaction = begin(client).await?;
    let result = set_mode_in(&transaction, audit, event, author, mode).await;
    finish(transaction, result).await
}

async fn set_mode_in(
    transaction: &Transaction<'_>,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    mode: DashboardMode,
) -> Result<(ModeOutcome, Finish), ModeError> {
    let Some(stored) = raise_generation(transaction, event).await? else {
        return Err(ModeError::NotConfigured);
    };
    if stored.mode == mode {
        return Ok((ModeOutcome::Unchanged, Finish::Rollback));
    }
    if mode == DashboardMode::Configured {
        let (_, live) = live_documents(get_heads(transaction, event).await?);
        if live.set.dashboards.is_empty() {
            return Err(ModeError::Invalid(nothing_to_show()));
        }
    }
    update_mode_and_preset(transaction, event, mode, stored.preset.as_ref()).await?;
    check_deferred_constraints(transaction).await?;
    audit
        .record(
            transaction,
            &RecordedChange {
                event,
                author: author.clone(),
                origin: RevisionOrigin::Editor,
                preset: None,
                mode,
                generation: stored.generation,
                revisions: Vec::new(),
            },
        )
        .await
        .context("Failed to record the dashboard switch in the electoral log")?;
    Ok((
        ModeOutcome::Switched {
            generation: stored.generation,
        },
        Finish::Commit,
    ))
}
