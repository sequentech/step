// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Electoral-log audits.
//!
//! An audit compares an election event's stored records with their Merkle leaves,
//! recomputes the stored tree, and checks every published checkpoint: its signature,
//! its signer, and that it is part of the log's history. The result is recorded on a
//! task execution; findings are reported, never repaired.
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::electoral_log_checkpoint::{
    get_election_events_with_open_voting, get_election_events_with_recent_checkpoints,
    get_electoral_log_checkpoints, get_last_published_tree_size, PublishedCheckpoint,
};
use crate::postgres::tally_session_execution::append_tally_session_log;
use crate::services::ballot_box::{request_sequencer, wait_for_sequencer, CLOSING_SEQUENCER_WAIT};
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_board::{
    get_board_client, get_board_public_key, get_election_board, get_event_store,
};
use crate::services::electoral_log_checkpoint_copies::{
    cross_check, read_checkpoint_copies, CheckpointCopyConfig, CheckpointCopyPolicy,
};
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::tasks_execution::{post, update_fail, update_with_annotations};
use crate::tasks::audit_electoral_log::audit_electoral_log;
use crate::types::tasks::ETasksExecution;
use anyhow::{Context, Result};
use electoral_log::adapters::postgres::{AuditAnnotations, AuditOutcome};
use electoral_log::domain::{LogQuery, OrderColumn, SortDirection};
use electoral_log::messages::newtypes::ElectoralLogCheckpointReason;
use electoral_log::messages::statement::StatementType;
use electoral_log::proofs::{verify_checkpoint_signature, Checkpoint, LogState};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::Value;
use std::collections::HashSet;
use tracing::instrument;
use voting_crypto::RistrettoCtx;

/// Recorded as the executor of audits the platform starts by itself.
pub const SYSTEM_EXECUTOR: &str = "system";
/// Findings written to the task logs; the total count is always recorded.
const LOGGED_FINDINGS: usize = 100;

/// Result of one audit run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditResult {
    pub board: String,
    /// Size of the log in the audited snapshot.
    pub tree_size: i64,
    /// Hex-encoded stored root in the audited snapshot.
    pub root: String,
    pub published_checkpoints: usize,
    /// Integrity problems found; empty when the log is consistent.
    pub findings: Vec<String>,
}

/// After a tally completes: publish a checkpoint of the event's electoral log and
/// start an audit, reporting both in the tally session's logs. Failures are reported
/// and never propagated, so they cannot fail the tally.
pub async fn checkpoint_and_audit_electoral_log(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) {
    let published = publish_event_checkpoint(
        tenant_id,
        election_event_id,
        ElectoralLogCheckpointReason::TallyCompleted,
    )
    .await;
    let line = match published {
        Ok(checkpoint) => format!(
            "Published electoral-log checkpoint at {} entries, root {}",
            checkpoint.tree_size,
            hex::encode(&checkpoint.root)
        ),
        Err(error) => {
            tracing::error!(
                "Could not publish the electoral-log checkpoint for tally session {tally_session_id}: {error:?}"
            );
            format!("Could not publish the electoral-log checkpoint: {error:#}")
        }
    };
    log_to_tally(tenant_id, election_event_id, tally_session_id, &line).await;
    if let Err(error) = start_electoral_log_audit(
        tenant_id,
        election_event_id,
        Some(tally_session_id),
        SYSTEM_EXECUTOR,
    )
    .await
    {
        tracing::error!(
            "Could not start the electoral-log audit for tally session {tally_session_id}: {error:?}"
        );
        log_to_tally(
            tenant_id,
            election_event_id,
            tally_session_id,
            &format!("Could not start the electoral-log audit: {error:#}"),
        )
        .await;
    }
}

/// Publish a signed checkpoint of an election event's electoral log, signed with the
/// event's protocol-manager key.
pub async fn publish_event_checkpoint(
    tenant_id: &str,
    election_event_id: &str,
    reason: ElectoralLogCheckpointReason,
) -> Result<Checkpoint> {
    let mut client = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    let event = get_election_event_by_id(&transaction, tenant_id, election_event_id).await?;
    let board = get_election_event_board(event.bulletin_board_reference)
        .context("Election event has no electoral-log board")?;
    let electoral_log =
        ElectoralLog::new(&transaction, tenant_id, Some(election_event_id), &board).await?;
    transaction.commit().await?;
    drop(client);
    // Ballots accepted before voting closed reach the log through the sequencer, so
    // the closing checkpoint waits for them.
    if matches!(reason, ElectoralLogCheckpointReason::VotingClosed) {
        request_sequencer(election_event_id).await?;
        let store = get_event_store(election_event_id).await?;
        let waiting = wait_for_sequencer(CLOSING_SEQUENCER_WAIT, || {
            store.pending_count(election_event_id)
        })
        .await?;
        if waiting > 0 {
            tracing::warn!(
                "{waiting} accepted ballot(s) of election event {election_event_id} are not \
                 in the electoral log yet; the checkpoint of voting's close does not cover them"
            );
        }
    }
    electoral_log
        .publish_checkpoint(tenant_id, election_event_id, reason)
        .await
}

/// Environment variable with the seconds between periodic checkpoints of the logs of
/// election events with open voting.
pub const CHECKPOINT_INTERVAL_ENV: &str = "ELECTORAL_LOG_CHECKPOINT_INTERVAL_SECS";
/// Seconds between periodic checkpoints when the variable is unset or empty.
pub const DEFAULT_CHECKPOINT_INTERVAL_SECS: u64 = 300;

/// Seconds between periodic checkpoints, from `ELECTORAL_LOG_CHECKPOINT_INTERVAL_SECS`.
pub fn checkpoint_interval_secs() -> Result<u64> {
    parse_checkpoint_interval(std::env::var(CHECKPOINT_INTERVAL_ENV).ok().as_deref())
}

fn parse_checkpoint_interval(value: Option<&str>) -> Result<u64> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(DEFAULT_CHECKPOINT_INTERVAL_SECS),
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|secs| *secs > 0)
            .with_context(|| {
                format!(
                    "{CHECKPOINT_INTERVAL_ENV} must be a positive number of seconds, got {value:?}"
                )
            }),
    }
}

/// What a periodic run does with a log, from its size and the size of its last
/// published checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeriodicCheck {
    /// Nothing was appended since the last checkpoint.
    UpToDate,
    /// One record was appended: usually the record of the last checkpoint itself,
    /// which needs no checkpoint of its own.
    NewestRecordDecides,
    /// Records were appended since the last checkpoint.
    Due,
}

fn periodic_check(tree_size: i64, last_published: Option<i64>) -> Result<PeriodicCheck> {
    let Some(last_published) = last_published else {
        return Ok(if tree_size == 0 {
            PeriodicCheck::UpToDate
        } else {
            PeriodicCheck::Due
        });
    };
    match tree_size - last_published {
        grown if grown < 0 => anyhow::bail!(
            "The log has {tree_size} records, fewer than the {last_published} of its last \
             published checkpoint: it may have been rolled back"
        ),
        0 => Ok(PeriodicCheck::UpToDate),
        1 => Ok(PeriodicCheck::NewestRecordDecides),
        _ => Ok(PeriodicCheck::Due),
    }
}

/// Outcome of one periodic checkpoint run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PeriodicCheckpoints {
    pub published: usize,
    pub up_to_date: usize,
    pub failed: usize,
}

/// Publish a checkpoint of the log of every election event with open voting that grew
/// since its last checkpoint. A failure for one event is logged and does not stop the
/// others.
#[instrument(err)]
pub async fn publish_periodic_checkpoints() -> Result<PeriodicCheckpoints> {
    let (events, recent) = {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.build_transaction().read_only(true).start().await?;
        (
            get_election_events_with_open_voting(&transaction).await?,
            get_election_events_with_recent_checkpoints(&transaction, RECORD_SWEEP_HOURS).await?,
        )
    };
    // Events whose voting is no longer open but that stored a checkpoint lately
    // (e.g. at voting's close): their missing checkpoint records are recorded too.
    for (tenant_id, election_event_id) in recent.iter().filter(|event| !events.contains(event)) {
        match record_missing_checkpoints(tenant_id, election_event_id).await {
            Ok(0) => {}
            Ok(recorded) => tracing::warn!(
                election_event_id,
                recorded,
                "Recorded missing checkpoint records"
            ),
            Err(error) => tracing::error!(
                election_event_id,
                "Could not record the missing checkpoint records: {error:#}"
            ),
        }
    }
    let mut outcome = PeriodicCheckpoints::default();
    for (tenant_id, election_event_id) in events {
        match publish_periodic_checkpoint(&tenant_id, &election_event_id).await {
            Ok(true) => outcome.published += 1,
            Ok(false) => outcome.up_to_date += 1,
            // Stored, its record refused for the log's order: recorded later
            // by the sweeper; not a failure.
            Err(error) if crate::services::electoral_log_deferral::is_scope_gated(&error) => {
                outcome.published += 1;
                tracing::info!(
                    election_event_id,
                    "The periodic checkpoint is stored; its record waits for the log's order: {error:#}"
                );
            }
            Err(error) => {
                outcome.failed += 1;
                tracing::error!(
                    "Could not publish the periodic electoral-log checkpoint of election event {election_event_id}: {error:?}"
                );
            }
        }
    }
    Ok(outcome)
}

/// Records in the event's log every stored checkpoint of it whose record the board
/// lacks: one refused for the log's order (it had entries waiting) or that failed
/// after the checkpoint was stored. Idempotent (each record's delivery ID is fixed
/// per log size); each signed only once admitted. Stops at the first refusal (the
/// next beat goes on). Returns how many it recorded.
pub async fn record_missing_checkpoints(tenant_id: &str, election_event_id: &str) -> Result<usize> {
    let (electoral_log, board, checkpoints) = {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.transaction().await?;
        let event = get_election_event_by_id(&transaction, tenant_id, election_event_id).await?;
        let board = get_election_event_board(event.bulletin_board_reference)
            .context("Election event has no electoral-log board")?;
        let checkpoints =
            get_electoral_log_checkpoints(&transaction, tenant_id, election_event_id).await?;
        let electoral_log =
            ElectoralLog::new(&transaction, tenant_id, Some(election_event_id), &board).await?;
        transaction.commit().await?;
        (electoral_log, board, checkpoints)
    };
    let store = crate::services::electoral_log_board::get_event_databases()
        .await?
        .board_store(&board)
        .await?;
    let mut recorded = 0;
    let mut delivered = HashSet::new();
    for checkpoint in checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.board_name == board)
    {
        let delivery_id = checkpoint_record_delivery_id(checkpoint);
        if store.delivered(&board, &delivery_id).await?.is_some() {
            delivered.insert(delivery_id);
        }
    }
    for checkpoint in unrecorded_checkpoints(&checkpoints, &board, &delivered) {
        let log_uid = checkpoint.log_uid.hyphenated().to_string();
        let reason: ElectoralLogCheckpointReason = checkpoint
            .reason
            .parse()
            .with_context(|| format!("Unknown checkpoint reason {}", checkpoint.reason))?;
        match electoral_log
            .post_checkpoint_record(
                election_event_id,
                &log_uid,
                u64::try_from(checkpoint.tree_size)?,
                &checkpoint.root,
                reason,
            )
            .await
        {
            Ok(()) => {
                tracing::warn!(
                    election_event_id,
                    tree_size = checkpoint.tree_size,
                    "Recorded a stored checkpoint whose record was missing from the log"
                );
                recorded += 1;
            }
            Err(error) if crate::services::electoral_log_deferral::is_scope_gated(&error) => {
                tracing::info!(
                    election_event_id,
                    "Missing checkpoint records wait for the log's order: {error:#}"
                );
                break;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(recorded)
}

/// How many hours after an event's last stored checkpoint its records are still
/// swept when its voting is no longer open.
pub const RECORD_SWEEP_HOURS: i32 = 24;

/// The delivery ID of a checkpoint's record in its log (fixed per log and size).
pub fn checkpoint_record_delivery_id(checkpoint: &PublishedCheckpoint) -> String {
    format!(
        "{}{}:{}",
        crate::services::electoral_log::CHECKPOINT_DELIVERY_PREFIX,
        checkpoint.log_uid.hyphenated(),
        checkpoint.tree_size
    )
}

/// The stored checkpoints of `board` whose record is not among `delivered`
/// (delivery IDs the board holds), oldest first; another board's are not its.
pub fn unrecorded_checkpoints<'a>(
    checkpoints: &'a [PublishedCheckpoint],
    board: &str,
    delivered: &HashSet<String>,
) -> Vec<&'a PublishedCheckpoint> {
    let mut missing: Vec<&PublishedCheckpoint> = checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.board_name == board)
        .filter(|checkpoint| !delivered.contains(&checkpoint_record_delivery_id(checkpoint)))
        .collect();
    missing.sort_by_key(|checkpoint| checkpoint.tree_size);
    missing
}

/// Publish a periodic checkpoint of one election event's log if it grew since its
/// last checkpoint, and return whether it did.
async fn publish_periodic_checkpoint(tenant_id: &str, election_event_id: &str) -> Result<bool> {
    // First the records of stored checkpoints that are missing from the log (a
    // checkpoint is never left unrecorded for good).
    if let Err(error) = record_missing_checkpoints(tenant_id, election_event_id).await {
        tracing::error!(
            election_event_id,
            "Could not record the missing checkpoint records: {error:#}"
        );
    }
    let (board, checkpoint, last_published) = {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.build_transaction().read_only(true).start().await?;
        let event = get_election_event_by_id(&transaction, tenant_id, election_event_id).await?;
        let board = get_election_event_board(event.bulletin_board_reference)
            .context("Election event has no electoral-log board")?;
        let checkpoint = get_event_store(election_event_id)
            .await?
            .journal()
            .checkpoint(&board)
            .await?;
        let last_published = get_last_published_tree_size(
            &transaction,
            tenant_id,
            election_event_id,
            &checkpoint.log_uid,
        )
        .await?;
        (board, checkpoint, last_published)
    };
    let due = match periodic_check(i64::try_from(checkpoint.tree_size)?, last_published)? {
        PeriodicCheck::UpToDate => false,
        PeriodicCheck::Due => true,
        PeriodicCheck::NewestRecordDecides => !newest_record_is_checkpoint(&board).await?,
    };
    if due {
        publish_event_checkpoint(
            tenant_id,
            election_event_id,
            ElectoralLogCheckpointReason::Periodic,
        )
        .await?;
    }
    Ok(due)
}

/// Whether the newest record of a board records a published checkpoint.
async fn newest_record_is_checkpoint(board: &str) -> Result<bool> {
    let query = LogQuery {
        order: vec![(OrderColumn::Id, SortDirection::Desc)],
        limit: 1,
        ..LogQuery::default()
    };
    let newest = get_board_client().await?.query(board, &query).await?;
    Ok(newest.first().is_some_and(|record| {
        record.statement_kind == StatementType::ElectoralLogCheckpoint.to_string()
    }))
}

/// Create the audit task and enqueue it. Audits started by a tally also report to
/// the tally session's logs.
#[instrument(err)]
pub async fn start_electoral_log_audit(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: Option<&str>,
    executed_by_user: &str,
) -> Result<TasksExecution> {
    let task = post(
        tenant_id,
        Some(election_event_id),
        ETasksExecution::AUDIT_ELECTORAL_LOG,
        executed_by_user,
    )
    .await?;
    if let Some(tally_session_id) = tally_session_id {
        log_to_tally(
            tenant_id,
            election_event_id,
            tally_session_id,
            &format!("Electoral-log audit queued (task {})", task.id),
        )
        .await;
    }
    let sent = get_celery_app()
        .await
        .send_task(audit_electoral_log::new(
            tenant_id.to_string(),
            election_event_id.to_string(),
            tally_session_id.map(str::to_string),
            task.clone(),
        ))
        .await;
    if let Err(error) = sent {
        let message = format!("Error enqueuing the electoral-log audit: {error:?}");
        if let Err(update_error) = update_fail(&task, &message).await {
            tracing::error!("Failed to mark the audit task as failed: {update_error:?}");
        }
        anyhow::bail!(message);
    }
    Ok(task)
}

/// Append a line to a tally session's logs. Failures are logged, not propagated: the
/// audit result is already recorded on its task.
pub async fn log_to_tally(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    text: &str,
) {
    let result: Result<()> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.transaction().await?;
        append_tally_session_log(
            &transaction,
            tenant_id,
            election_event_id,
            tally_session_id,
            text,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }
    .await;
    if let Err(error) = result {
        tracing::error!(
            "Could not add '{text}' to the logs of tally session {tally_session_id}: {error:?}"
        );
    }
}

/// Audit an election event's electoral log against its published checkpoints: the
/// event's board and the sealed logs it continues when the event was imported, each
/// against the checkpoints published of it, and the empty boards of its elections and
/// areas.
#[instrument(err)]
pub async fn run_electoral_log_audit(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<AuditResult> {
    let mut client = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    let event = get_election_event_by_id(&transaction, tenant_id, election_event_id).await?;
    let board = get_election_event_board(event.bulletin_board_reference.clone())
        .context("Election event has no electoral-log board")?;
    let expected_signer =
        get_board_public_key(&transaction, tenant_id, Some(election_event_id), &board)
            .await?
            .to_b64_string()?;
    let published =
        get_electoral_log_checkpoints(&transaction, tenant_id, election_event_id).await?;
    transaction.commit().await?;
    drop(client);
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let platform_prefix = platform_board_prefix(tenant_id, &slug);

    let (own, imported): (Vec<_>, Vec<_>) = published
        .iter()
        .cloned()
        .partition(|publication| publication.board_name == board);
    let (mut checkpoints, mut findings) = check_publications(&board, &expected_signer, &own);
    let copies = CheckpointCopyConfig::from_env()?;
    if copies.policy != CheckpointCopyPolicy::Off {
        match read_checkpoint_copies(&copies, tenant_id, election_event_id).await {
            Ok(read) => {
                findings.extend(read.findings);
                findings.extend(cross_check(&own, &read.copies));
                let (copied, copy_findings) = check_labelled_publications(
                    &board,
                    CheckpointSigner::Expected(&expected_signer),
                    &read.copies,
                    "Write-once copy of the checkpoint",
                );
                findings.extend(copy_findings);
                for checkpoint in copied {
                    if !checkpoints.contains(&checkpoint) {
                        checkpoints.push(checkpoint);
                    }
                }
            }
            Err(error) => findings.push(format!(
                "Could not read the write-once checkpoint copies: {error:#}"
            )),
        }
    }
    let store = get_event_store(election_event_id).await?;
    let report = store.audit(&board, &checkpoints).await?;
    findings.extend(report.findings());
    // Every other log of the event's database: the sealed logs an imported event's
    // board continues, audited against the checkpoints published of them before the
    // export, which carry the source event's signatures; and the empty boards the
    // platform creates for the event's elections and areas, which nothing writes to
    // and of which no checkpoints are published.
    let logs = store.logs().await?;
    for log in &logs {
        let name = &log.checkpoint.log_name;
        if *name == board {
            continue;
        }
        let kind = classify_log(name, log.state, log.checkpoint.tree_size, &platform_prefix);
        if kind == AuditedLog::Unexpected {
            findings.push(format!(
                "Log {name} is not sealed but is neither the event's board nor an empty \
                 election or area board"
            ));
        }
        let publications: Vec<_> = imported
            .iter()
            .filter(|publication| publication.board_name == *name)
            .cloned()
            .collect();
        let (checkpoints, signature_findings) = check_log_publications(kind, name, &publications);
        let label = kind.label();
        findings.extend(
            signature_findings
                .into_iter()
                .chain(store.audit(name, &checkpoints).await?.findings())
                .map(|finding| format!("{label} {name}: {finding}")),
        );
    }
    for publication in &imported {
        if !logs
            .iter()
            .any(|log| log.checkpoint.log_name == publication.board_name)
        {
            findings.push(format!(
                "Published checkpoint at size {} names board {}, which is not in the event's \
                 database",
                publication.tree_size, publication.board_name
            ));
        }
    }
    Ok(AuditResult {
        board,
        tree_size: report.committed_size,
        root: report.root,
        published_checkpoints: published.len(),
        findings,
    })
}

/// What the audit makes of a log of the event's database other than its board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuditedLog {
    /// An empty open board that the platform creates for each election and area of
    /// the event (`ensure_election_audit_boards`) and never writes to. It outlives
    /// its election or area, and is created before their row is committed.
    PlatformBoard,
    /// A sealed log that an imported event's board continues.
    Continued,
    /// An open log that is neither the event's board nor an empty platform board.
    Unexpected,
}

impl AuditedLog {
    /// How the log's findings name it.
    fn label(self) -> &'static str {
        match self {
            AuditedLog::PlatformBoard => "Election or area board",
            AuditedLog::Continued => "Continued log",
            AuditedLog::Unexpected => "Unexpected log",
        }
    }
}

/// Classify a log of the event's database other than its board by its name, state
/// and stored size: a sealed log is one an import continues; an open one must be an
/// empty board named as the platform names election and area boards
/// (`platform_prefix` followed by an ID's 32 hexadecimal digits). The stored size is
/// not trusted: the log's audit still checks it against the records and the tree.
fn classify_log(name: &str, state: LogState, tree_size: u64, platform_prefix: &str) -> AuditedLog {
    if state == LogState::Sealed {
        AuditedLog::Continued
    } else if tree_size == 0 && is_platform_board(name, platform_prefix) {
        AuditedLog::PlatformBoard
    } else {
        AuditedLog::Unexpected
    }
}

/// The start of the names `get_election_board` gives the boards of this tenant's
/// elections and areas.
fn platform_board_prefix(tenant_id: &str, slug: &str) -> String {
    get_election_board(tenant_id, "", slug)
}

/// Whether `name` is `prefix` followed by an ID without dashes.
fn is_platform_board(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .is_some_and(|id| id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// The checkpoints to audit a log other than the event's board against, and the
/// findings about its publications. A continued (or unexpected) log's checkpoints
/// must verify with the key they name; the platform publishes none of an election
/// or area board, so each that names one is a finding and none is used.
fn check_log_publications(
    kind: AuditedLog,
    log: &str,
    published: &[PublishedCheckpoint],
) -> (Vec<Checkpoint>, Vec<String>) {
    match kind {
        AuditedLog::PlatformBoard => (
            Vec::new(),
            published
                .iter()
                .map(|publication| {
                    format!(
                        "Published checkpoint at size {} names this board, but the platform \
                         publishes no checkpoints of election or area boards",
                        publication.tree_size
                    )
                })
                .collect(),
        ),
        AuditedLog::Continued | AuditedLog::Unexpected => {
            check_imported_publications(log, published)
        }
    }
}

/// Check the signatures, signers and reasons of published checkpoints, returning the
/// checkpoints to verify against the log history and the findings so far.
fn check_publications(
    board: &str,
    expected_signer: &str,
    published: &[PublishedCheckpoint],
) -> (Vec<Checkpoint>, Vec<String>) {
    check_labelled_publications(
        board,
        CheckpointSigner::Expected(expected_signer),
        published,
        "Published checkpoint",
    )
}

/// Like `check_publications` for the checkpoints of a log an imported board continues,
/// which were signed by the key of the event it was exported from: their signatures
/// must verify against the key they name.
fn check_imported_publications(
    log: &str,
    published: &[PublishedCheckpoint],
) -> (Vec<Checkpoint>, Vec<String>) {
    check_labelled_publications(
        log,
        CheckpointSigner::Named,
        published,
        "Published checkpoint",
    )
}

/// Which key a checkpoint's signature must verify with.
#[derive(Clone, Copy)]
enum CheckpointSigner<'a> {
    /// The event's own key.
    Expected(&'a str),
    /// The key the publication names.
    Named,
}

/// Like `check_publications`, naming the checked items `label` in the findings.
fn check_labelled_publications(
    board: &str,
    signer: CheckpointSigner<'_>,
    published: &[PublishedCheckpoint],
    label: &str,
) -> (Vec<Checkpoint>, Vec<String>) {
    let mut checkpoints = Vec::new();
    let mut findings = Vec::new();
    for publication in published {
        let size = publication.tree_size;
        let (Ok(tree_size), Ok(root), Ok(reason)) = (
            u64::try_from(publication.tree_size),
            hex::decode(&publication.root),
            publication.reason.parse::<ElectoralLogCheckpointReason>(),
        ) else {
            findings.push(format!("{label} at size {size} is malformed"));
            continue;
        };
        let checkpoint = Checkpoint {
            log_name: publication.board_name.clone(),
            log_uid: publication.log_uid,
            tree_size,
            root,
        };
        if publication.board_name != board {
            findings.push(format!(
                "{label} at size {size} names board {} instead of {board}",
                publication.board_name
            ));
            continue;
        }
        let unexpected_signer = match signer {
            CheckpointSigner::Expected(expected) => publication.signer_pk != expected,
            CheckpointSigner::Named => false,
        };
        if unexpected_signer {
            findings.push(format!(
                "{label} at size {size} is signed by an unexpected key"
            ));
        } else if let Err(error) = verify_checkpoint_signature(
            &checkpoint,
            reason,
            &publication.signer_pk,
            &publication.signature,
        ) {
            findings.push(format!(
                "{label} at size {size} has an invalid signature: {error:#}"
            ));
        }
        checkpoints.push(checkpoint);
    }
    (checkpoints, findings)
}

/// Record an audit result on its task and return the one-line summary.
pub async fn record_audit(task: &TasksExecution, result: &Result<AuditResult>) -> Result<String> {
    let mut logs: Option<Value> = task.logs.clone();
    let mut log = |line: &str| -> Result<()> {
        logs = Some(serde_json::to_value(append_general_log(&logs, line))?);
        Ok(())
    };
    let (outcome, summary, annotations) = match result {
        Ok(audit) => {
            log(&format!(
                "Audited board {}: {} entries, root {}, {} published checkpoints",
                audit.board, audit.tree_size, audit.root, audit.published_checkpoints
            ))?;
            for finding in audit.findings.iter().take(LOGGED_FINDINGS) {
                log(&format!("Finding: {finding}"))?;
            }
            if audit.findings.len() > LOGGED_FINDINGS {
                log(&format!(
                    "... and {} more findings",
                    audit.findings.len() - LOGGED_FINDINGS
                ))?;
            }
            let (outcome, summary) = if audit.findings.is_empty() {
                (AuditOutcome::Clean, AuditOutcome::Clean.to_string())
            } else {
                (
                    AuditOutcome::Findings,
                    format!("{} findings", audit.findings.len()),
                )
            };
            (
                outcome,
                summary,
                AuditAnnotations {
                    outcome,
                    board: Some(audit.board.clone()),
                    tree_size: Some(audit.tree_size),
                    root: Some(audit.root.clone()),
                    findings: Some(audit.findings.len()),
                    published_checkpoints: Some(audit.published_checkpoints),
                },
            )
        }
        Err(error) => {
            log(&format!("Error: {error:#}"))?;
            (
                AuditOutcome::Error,
                AuditOutcome::Error.to_string(),
                AuditAnnotations {
                    outcome: AuditOutcome::Error,
                    board: None,
                    tree_size: None,
                    root: None,
                    findings: None,
                    published_checkpoints: None,
                },
            )
        }
    };
    log(&format!("Electoral-log audit {summary}"))?;
    let status = match outcome {
        AuditOutcome::Clean => TasksExecutionStatus::SUCCESS,
        AuditOutcome::Findings | AuditOutcome::Error => TasksExecutionStatus::FAILED,
    };
    update_with_annotations(
        &task.tenant_id,
        &task.id,
        status,
        logs.unwrap_or(Value::Array(vec![])),
        serde_json::to_value(&annotations)?,
    )
    .await?;
    Ok(format!("Electoral-log audit {}: {summary}", task.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::electoral_log::sign_checkpoint;
    use electoral_log::messages::message::SigningData;
    use voting_crypto::signing::SigningKey;
    use voting_crypto::signing::VerificationKey;

    fn publication(sk: &SigningKey, board: &str, size: i64) -> PublishedCheckpoint {
        let checkpoint = Checkpoint {
            log_name: board.to_string(),
            log_uid: electoral_log::proofs::Uuid::from_u128(1),
            tree_size: u64::try_from(size).unwrap(),
            root: vec![9; 32],
        };
        let sd = SigningData::new(sk.clone(), "", sk.clone());
        sign_checkpoint(&sd, &checkpoint, ElectoralLogCheckpointReason::VotingClosed).unwrap()
    }

    fn public_key(sk: &SigningKey) -> String {
        VerificationKey::from_sk(sk)
            .unwrap()
            .to_b64_string()
            .unwrap()
    }

    #[test]
    fn published_checkpoints_pass_the_audit_signature_checks() {
        let key = SigningKey::generate().unwrap();
        let sd = SigningData::new(key.clone(), "", key.clone());
        let checkpoint = Checkpoint {
            log_name: "board".to_string(),
            log_uid: electoral_log::proofs::Uuid::from_u128(2),
            tree_size: 17,
            root: vec![3; 32],
        };
        let mut published = Vec::new();
        for reason in [
            ElectoralLogCheckpointReason::VotingClosed,
            ElectoralLogCheckpointReason::TallyCompleted,
        ] {
            published.push(sign_checkpoint(&sd, &checkpoint, reason).unwrap());
        }
        let (checkpoints, findings) = check_publications("board", &public_key(&key), &published);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(checkpoints, vec![checkpoint.clone(), checkpoint]);

        published[0].reason = ElectoralLogCheckpointReason::TallyCompleted.to_string();
        let (_, findings) = check_publications("board", &public_key(&key), &published);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("invalid signature"));
    }

    #[test]
    fn publications_need_the_event_key_a_valid_signature_and_the_board() {
        let key = SigningKey::generate().unwrap();
        let other = SigningKey::generate().unwrap();
        let good = publication(&key, "board", 4);
        let foreign_signer = publication(&other, "board", 5);
        let mut tampered = publication(&key, "board", 6);
        tampered.root = hex::encode([8; 32]);
        let other_board = publication(&key, "elsewhere", 7);
        let mut malformed = publication(&key, "board", 8);
        malformed.root = "zz".into();
        let mut unknown_reason = publication(&key, "board", 9);
        unknown_reason.reason = "voting_closed".into();
        let (checkpoints, findings) = check_publications(
            "board",
            &public_key(&key),
            &[
                good,
                foreign_signer,
                tampered,
                other_board,
                malformed,
                unknown_reason,
            ],
        );
        assert_eq!(
            checkpoints.iter().map(|c| c.tree_size).collect::<Vec<_>>(),
            vec![4, 5, 6]
        );
        assert_eq!(findings.len(), 5, "{findings:?}");
        assert!(findings[0].contains("unexpected key"));
        assert!(findings[1].contains("invalid signature"));
        assert!(findings[2].contains("names board"));
        assert!(findings[3].contains("malformed"));
        assert!(findings[4].contains("malformed"));
    }
}

#[cfg(test)]
mod log_classification_tests {
    use super::*;

    const TENANT: &str = "90505c8a-23a9-4cdf-a123-456789abcdef";
    const ELECTION: &str = "d9fda03a-6930-4f2f-bee8-4f113fdb7b62";
    const AREA: &str = "b2c8d350-4f3c-4f85-87c8-c61cd3ed8ec8";
    /// An election whose row was deleted, or never committed: its board stays.
    const GONE: &str = "5e4d3c2b-1a09-4f8e-8d7c-6b5a49382716";
    /// The name the platform gives `ELECTION`'s board with slug `dev` (as observed
    /// in a dev audit that wrongly reported it).
    const ELECTION_BOARD: &str =
        "devtenant90505c8a23a94cdfaelectiond9fda03a69304f2fbee84f113fdb7b62";

    fn prefix() -> String {
        platform_board_prefix(TENANT, "dev")
    }

    fn classify(name: &str, state: LogState, tree_size: u64) -> AuditedLog {
        classify_log(name, state, tree_size, &prefix())
    }

    fn publication(board: &str, size: i64) -> PublishedCheckpoint {
        PublishedCheckpoint {
            board_name: board.into(),
            log_uid: uuid::Uuid::nil(),
            tree_size: size,
            root: "00".into(),
            reason: "PERIODIC".into(),
            signer_pk: String::new(),
            signature: String::new(),
        }
    }

    #[test]
    fn election_and_area_boards_share_the_platform_prefix() {
        // `ensure_election_audit_boards` names the boards of elections and of areas
        // (`upsert_area`) with `get_election_board`.
        assert_eq!(get_election_board(TENANT, ELECTION, "dev"), ELECTION_BOARD);
        assert!(ELECTION_BOARD.starts_with(&prefix()), "{}", prefix());
        assert!(get_election_board(TENANT, AREA, "dev").starts_with(&prefix()));
    }

    #[test]
    fn an_empty_open_election_or_area_board_is_expected() {
        assert_eq!(
            classify(ELECTION_BOARD, LogState::Open, 0),
            AuditedLog::PlatformBoard
        );
        for id in [AREA, GONE] {
            let board = get_election_board(TENANT, id, "dev");
            assert_eq!(
                classify(&board, LogState::Open, 0),
                AuditedLog::PlatformBoard,
                "{board}"
            );
        }
    }

    #[test]
    fn a_platform_board_with_records_is_unexpected() {
        // Nothing writes to these boards: every electoral-log entry goes to the
        // event's board.
        assert_eq!(
            classify(ELECTION_BOARD, LogState::Open, 1),
            AuditedLog::Unexpected
        );
    }

    #[test]
    fn a_sealed_log_is_a_continued_log() {
        // An imported event's logs keep the source event's names, e.g. the board of
        // an election whose ID the import replaced.
        let imported = get_election_board(TENANT, GONE, "dev");
        assert_eq!(
            classify(&imported, LogState::Sealed, 0),
            AuditedLog::Continued
        );
        assert_eq!(
            classify("anything", LogState::Sealed, 7),
            AuditedLog::Continued
        );
    }

    #[test]
    fn an_open_log_not_named_as_a_platform_board_is_unexpected() {
        let other_tenant =
            get_election_board("11111111-2222-4333-8444-555555555555", ELECTION, "dev");
        let other_slug = get_election_board(TENANT, ELECTION, "prod");
        let other_event_board = format!(
            "{}event{}",
            &prefix()[..prefix().len() - "election".len()],
            "d9fda03a69304f2fbee84f113fdb7b62"
        );
        for name in [
            other_tenant.as_str(),
            other_slug.as_str(),
            other_event_board.as_str(),
            &format!("{}{}", prefix(), "not-a-uuid"),
            &format!("{}{}", prefix(), "d9fda03a69304f2fbee84f113fdb7b6"),
            &format!("{}{}", prefix(), "d9fda03a69304f2fbee84f113fdb7b62aa"),
            &format!("{}{}", prefix(), "z9fda03a69304f2fbee84f113fdb7b62"),
        ] {
            assert_eq!(
                classify(name, LogState::Open, 0),
                AuditedLog::Unexpected,
                "{name}"
            );
        }
    }

    #[test]
    fn checkpoints_published_of_a_platform_board_are_findings() {
        // The platform publishes no checkpoints of election or area boards, so one
        // that names such a board is reported, not checked against a key it names.
        let (checkpoints, findings) = check_log_publications(
            AuditedLog::PlatformBoard,
            ELECTION_BOARD,
            &[publication(ELECTION_BOARD, 3)],
        );
        assert!(checkpoints.is_empty());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("size 3"), "{findings:?}");
        assert!(
            findings[0].contains("publishes no checkpoints"),
            "{findings:?}"
        );
        // Without publications there is nothing to report.
        let (checkpoints, findings) =
            check_log_publications(AuditedLog::PlatformBoard, ELECTION_BOARD, &[]);
        assert!(checkpoints.is_empty() && findings.is_empty());
        // Control: a continued log's publication goes through the signature checks.
        let (_, findings) = check_log_publications(
            AuditedLog::Continued,
            ELECTION_BOARD,
            &[publication(ELECTION_BOARD, 3)],
        );
        assert!(
            findings
                .iter()
                .all(|f| !f.contains("publishes no checkpoints")),
            "{findings:?}"
        );
    }
}

#[cfg(test)]
mod periodic_checkpoint_tests {
    use super::*;

    fn stored(board: &str, uid: &str, size: i64) -> PublishedCheckpoint {
        PublishedCheckpoint {
            board_name: board.into(),
            log_uid: uuid::Uuid::parse_str(uid).unwrap(),
            tree_size: size,
            root: "00".into(),
            reason: "PERIODIC".into(),
            signer_pk: String::new(),
            signature: String::new(),
        }
    }

    #[test]
    fn the_sweeper_records_exactly_the_stored_checkpoints_its_log_lacks_oldest_first() {
        const UID: &str = "7b0a8a3e-6c55-4f43-9e93-0d6f5ad4d2a1";
        // Attempt 8: the record of the checkpoint at size 43 was refused while
        // entries waited; 30, 46 and 48 were recorded.
        let checkpoints = vec![
            stored("board", UID, 48),
            stored("board", UID, 30),
            stored("board", UID, 43),
            stored("board", UID, 46),
            stored("other-board", UID, 7),
        ];
        let id = |size: i64| checkpoint_record_delivery_id(&stored("board", UID, size));
        assert_eq!(id(43), format!("electoral-log-checkpoint:{UID}:43"));
        let delivered: HashSet<String> = [id(30), id(46), id(48)].into();
        let missing: Vec<i64> = unrecorded_checkpoints(&checkpoints, "board", &delivered)
            .iter()
            .map(|checkpoint| checkpoint.tree_size)
            .collect();
        assert_eq!(
            missing,
            vec![43],
            "only the lost record; not another board's"
        );
        // Nothing recorded yet: all of this board's, oldest first.
        let missing: Vec<i64> = unrecorded_checkpoints(&checkpoints, "board", &HashSet::new())
            .iter()
            .map(|checkpoint| checkpoint.tree_size)
            .collect();
        assert_eq!(missing, vec![30, 43, 46, 48]);
        // All recorded: nothing to do (idempotent).
        let all: HashSet<String> = [id(30), id(43), id(46), id(48)].into();
        assert!(unrecorded_checkpoints(&checkpoints, "board", &all).is_empty());
    }

    #[test]
    fn the_interval_defaults_and_rejects_invalid_values() {
        assert_eq!(
            parse_checkpoint_interval(None).unwrap(),
            DEFAULT_CHECKPOINT_INTERVAL_SECS
        );
        assert_eq!(
            parse_checkpoint_interval(Some(" ")).unwrap(),
            DEFAULT_CHECKPOINT_INTERVAL_SECS
        );
        assert_eq!(parse_checkpoint_interval(Some("60")).unwrap(), 60);
        for invalid in ["0", "-5", "5m", "1.5"] {
            let error = parse_checkpoint_interval(Some(invalid))
                .unwrap_err()
                .to_string();
            assert!(error.contains(CHECKPOINT_INTERVAL_ENV), "{error}");
        }
    }

    #[test]
    fn a_log_is_checkpointed_only_when_it_grew() {
        assert_eq!(periodic_check(0, None).unwrap(), PeriodicCheck::UpToDate);
        assert_eq!(periodic_check(5, None).unwrap(), PeriodicCheck::Due);
        assert_eq!(periodic_check(5, Some(5)).unwrap(), PeriodicCheck::UpToDate);
        assert_eq!(
            periodic_check(6, Some(5)).unwrap(),
            PeriodicCheck::NewestRecordDecides
        );
        assert_eq!(periodic_check(7, Some(5)).unwrap(), PeriodicCheck::Due);
    }

    #[test]
    fn a_log_smaller_than_its_last_checkpoint_is_an_error() {
        let error = periodic_check(4, Some(5)).unwrap_err().to_string();
        assert!(error.contains("rolled back"), "{error}");
    }
}
