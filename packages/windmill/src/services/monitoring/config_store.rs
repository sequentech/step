// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An election event's monitoring configuration: what it has, and each
//! change to it (a save of one document, a reset to a preset, a switch of
//! the Dashboard tab) as one transaction.
//!
//! A change runs under READ COMMITTED and starts by raising the event's
//! generation, which holds the event's row until it ends, so the changes of
//! one event queue behind one another and each reads what the one before
//! it left. It then checks the document's head against the revision the
//! editor started from, the edit against the policy and the chart engine,
//! moves the head, writes the revision, and has the electoral log record the
//! change before it commits. Anything refused on the way writes nothing, and
//! nothing reaches the log. A commit that fails after the log recorded the
//! change leaves an entry for a generation the event never reached; the next
//! change takes that generation again, and its entry says so.

use crate::postgres::monitoring_config::{
    advance_head, create_head, create_monitoring_event, get_head, get_heads, get_monitoring_event,
    get_revision, get_revisions, insert_revision, raise_generation, update_mode_and_preset,
    NewRevision,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use deadpool_postgres::{Client, Transaction};
use sequent_core::monitoring::config::{ConfigKind, ConfigSet};
use sequent_core::monitoring::presets;
use sequent_core::monitoring::problem::{Code, Problem, Report, Severity};
use sequent_core::monitoring::revision::{
    assemble, check_edit, document_digest, plan_reset, Assembled, DashboardMode, DocumentChange,
    Edit, LiveDocument, LiveHead, RevisionOrigin,
};
use tokio_postgres::IsolationLevel;
use tracing::{instrument, warn};

pub use crate::postgres::monitoring_config::{Author, EventRef, PresetVersion, StoredRevision};

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
    Internal(anyhow::Error),
}

#[derive(Debug)]
pub enum ResetOutcome {
    Reset {
        generation: i64,
        revisions: Vec<WrittenRevision>,
    },
    /// The event already has the preset's documents, preset and mode.
    Unchanged,
}

#[derive(Debug)]
pub enum ResetError {
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
            Code::DanglingReference,
            "",
            "The event has no dashboard to show; reset it to a preset first.",
        )],
    }
}

fn errors_only(mut report: Report) -> Report {
    report
        .problems
        .retain(|problem| problem.severity == Severity::Error);
    report
}

/// What an event is configured with; `None` for an event on the standard
/// dashboard that was never configured.
#[instrument(err, skip(transaction))]
pub async fn get_live_config(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> anyhow::Result<Option<LiveConfig>> {
    let Some(stored) = get_monitoring_event(transaction, event).await? else {
        return Ok(None);
    };
    let (documents, assembled) = live_documents(get_heads(transaction, event).await?);
    Ok(Some(LiveConfig {
        mode: stored.mode,
        preset: stored.preset,
        generation: stored.generation,
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

/// Commits what `change` did when it saved something, and rolls it back
/// otherwise.
async fn finish<T, E: From<anyhow::Error>>(
    transaction: Transaction<'_>,
    result: Result<(T, bool), E>,
) -> Result<T, E> {
    match result {
        Ok((outcome, true)) => {
            transaction
                .commit()
                .await
                .context("Failed to commit the monitoring configuration change")?;
            Ok(outcome)
        }
        Ok((outcome, false)) => {
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

/// Saves or removes one document.
#[instrument(skip(client, checks, audit, edit), fields(kind = %edit.kind, key = edit.key, expected = ?edit.expected))]
pub async fn save(
    client: &mut Client,
    checks: &dyn MonitoringConfigChecks,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    edit: DocumentEdit<'_>,
) -> Result<SaveOutcome, SaveError> {
    let transaction = begin(client).await?;
    let result = save_in(&transaction, checks, audit, event, author, edit).await;
    finish(transaction, result).await
}

async fn save_in(
    transaction: &Transaction<'_>,
    checks: &dyn MonitoringConfigChecks,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    edit: DocumentEdit<'_>,
) -> Result<(SaveOutcome, bool), SaveError> {
    let DocumentEdit {
        kind,
        key,
        edit: change,
        expected,
    } = edit;
    let Some(stored) = raise_generation(transaction, event).await? else {
        return Err(SaveError::NotConfigured);
    };
    let head = get_head(transaction, event, kind, key).await?;
    let exists = head
        .as_ref()
        .filter(|head| head.change == DocumentChange::Upsert);
    let started_from_head = match (expected, exists) {
        (ExpectedHead::Absent, None) => true,
        (ExpectedHead::At(revision), Some(head)) => head.revision == revision,
        _ => false,
    };
    if !started_from_head {
        return Err(SaveError::Conflict { current: head });
    }
    match (change, exists) {
        (Edit::Upsert(yaml), Some(head))
            if head.sha256.as_deref() == Some(document_digest(yaml).as_str()) =>
        {
            return Ok((SaveOutcome::Unchanged(head.clone()), false));
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

    let (_, live) = live_documents(get_heads(transaction, event).await?);
    let checked = check_edit(&live.set, kind, key, change).map_err(SaveError::Invalid)?;
    let engine = checks
        .check(&ProposedChange {
            event,
            kind,
            key,
            edit: change,
            set: &checked.set,
        })
        .await
        .context("Failed to have the chart engine check the edit")?;
    if !engine.is_accepted() {
        return Err(SaveError::Invalid(errors_only(engine)));
    }
    let mut warnings = checked.report;
    warnings.extend(engine);

    let Some(revision) = move_head(transaction, event, kind, key, head.as_ref()).await? else {
        return Err(SaveError::Conflict {
            current: get_head(transaction, event, kind, key).await?,
        });
    };
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
        SaveOutcome::Saved {
            revision: stored_revision,
            warnings,
        },
        true,
    ))
}

/// Leaves the event with exactly the preset's documents, and the Dashboard
/// tab in `mode`. An event without monitoring configuration gets it: this is
/// how an event is first configured.
#[instrument(skip(client, audit))]
pub async fn reset_to_preset(
    client: &mut Client,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    preset_id: &str,
    mode: DashboardMode,
) -> Result<ResetOutcome, ResetError> {
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
) -> Result<(ResetOutcome, bool), ResetError> {
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
        return Ok((ResetOutcome::Unchanged, false));
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
        },
        true,
    ))
}

/// Switches the Dashboard tab between the standard dashboard and the
/// configured ones. The configuration is kept either way.
#[instrument(skip(client, audit))]
pub async fn set_mode(
    client: &mut Client,
    audit: &dyn MonitoringConfigAudit,
    event: EventRef,
    author: &Author,
    mode: DashboardMode,
) -> Result<ModeOutcome, ModeError> {
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
) -> Result<(ModeOutcome, bool), ModeError> {
    let Some(stored) = raise_generation(transaction, event).await? else {
        return Err(ModeError::NotConfigured);
    };
    if stored.mode == mode {
        return Ok((ModeOutcome::Unchanged, false));
    }
    if mode == DashboardMode::Configured {
        let (_, live) = live_documents(get_heads(transaction, event).await?);
        if live.set.dashboards.is_empty() {
            return Err(ModeError::Invalid(nothing_to_show()));
        }
    }
    update_mode_and_preset(transaction, event, mode, stored.preset.as_ref()).await?;
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
        true,
    ))
}
