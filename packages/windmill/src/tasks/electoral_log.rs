// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::postgres::tenant::get_tenant_by_id_if_exist;
use crate::services::audit_log_writer::{emit_monitoring_line, monitoring_line};
use crate::services::celery_app::Queue;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::database::get_queue_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_board::{ensure_tenant_log, get_board_client};
use crate::services::electoral_log_dead_letter::{
    dead_letter_events, dead_letter_message, DeadLetterStage,
};
use crate::services::electoral_log_deferral::{
    bind_to_rows, in_wait_order, log_round, put_back, scope_of, stop_waiting, waiting_events,
    waiting_ids,
    Deferral, DeferralBackoff, WaitReason, Waiting, WaitingEvent,
};
use crate::services::monitoring::login_counter::{
    count_login_attempts_apart, login_attempt, LoginAttempt,
};
use crate::services::tenant_log::{
    names_no_election_event, tenant_activity_message, TenantActivity,
};
use crate::services::users::get_user_area_id;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use celery::task::Task;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::adapters::events::{owner_of_board, BoardOwner};
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::{
    AuditLogEntry, AuditStatementKind, ErrorMessageString, EventIdString, KeycloakEventTypeString,
};
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use electoral_log::signer::{is_signer_unavailable, SignerUnavailable};
use electoral_log::{ElectoralLogMessage, LogEntry};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use tracing::{event, info, instrument};
use uuid::Uuid;
use voting_crypto::RecordDeserialize;
use voting_crypto::RistrettoCtx;

/// Classifies the type of an incoming log event.
///
/// Serializes as a plain string for wire compatibility:
/// - `Internal`            → `"internal"`
/// - `KeycloakEvent(s)`   → `s` (the raw Keycloak event type, e.g. `"LOGIN"`)
/// - `TenantActivity`      → `"tenant_activity"`
#[derive(Clone, Debug, PartialEq)]
pub enum LogMessageType {
    Internal,
    KeycloakEvent(String),
    /// A request an administrator made outside any election event; the body is a
    /// `TenantActivity` as JSON. It goes to the tenant's log.
    TenantActivity,
}

/// Event type of the tenant-log entry of a message Keycloak sent to an account.
pub const TENANT_MESSAGE_SENT_EVENT: &str = "MESSAGE_SENT";

/// Wire name of `LogMessageType::TenantActivity`. Keycloak's event types are upper
/// case, so none is taken for it.
const TENANT_ACTIVITY_MESSAGE_TYPE: &str = "tenant_activity";

impl Serialize for LogMessageType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            LogMessageType::Internal => serializer.serialize_str("internal"),
            LogMessageType::KeycloakEvent(event_type) => serializer.serialize_str(event_type),
            LogMessageType::TenantActivity => {
                serializer.serialize_str(TENANT_ACTIVITY_MESSAGE_TYPE)
            }
        }
    }
}

impl<'de> Deserialize<'de> for LogMessageType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "internal" => LogMessageType::Internal,
            TENANT_ACTIVITY_MESSAGE_TYPE => LogMessageType::TenantActivity,
            _ => LogMessageType::KeycloakEvent(s),
        })
    }
}

/// Represents the typed content of a log event body.
///
/// Serializes as a plain string for wire compatibility:
/// - `Communications(msg)` → `"communications <msg>"`
/// - `Plain(s)`            → `s`
#[derive(Clone, Debug, PartialEq)]
pub enum LogEventBody {
    /// Body from a send-template action; contains the template message.
    Communications(String),
    /// Body from a standard Keycloak event; typically the error field or "null".
    Plain(String),
}

impl LogEventBody {
    pub fn as_raw(&self) -> String {
        match self {
            LogEventBody::Communications(msg) => format!("communications {}", msg),
            LogEventBody::Plain(s) => s.clone(),
        }
    }
}

impl Serialize for LogEventBody {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_raw())
    }
}

impl<'de> Deserialize<'de> for LogEventBody {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(if let Some(msg) = s.strip_prefix("communications ") {
            LogEventBody::Communications(msg.trim().to_string())
        } else {
            LogEventBody::Plain(s)
        })
    }
}

/// Represents an incoming log event.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LogEventInput {
    #[serde(default)]
    pub delivery_id: Option<String>,
    pub election_event_id: String,
    pub message_type: LogMessageType,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub tenant_id: String,
    pub body: LogEventBody,
    /// When Keycloak saw the event, in milliseconds since the Unix epoch.
    /// Absent from messages sent before the listener said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_time_ms: Option<i64>,
    /// Authentication metadata without network attribution: client, error
    /// outcome and authentication method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// A Keycloak admin event, sent with [`KEYCLOAK_ADMIN_EVENT`] as the
    /// message type: the operation, the resource, its new representation
    /// and who made it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_event: Option<serde_json::Value>,
    /// Set on an event put back to wait for its signer (or behind an
    /// earlier event of its log that waits): how long it has waited
    /// (`services::electoral_log_deferral`). Producers never set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deferral: Option<Deferral>,
}

/// The message type of a Keycloak admin event.
pub const KEYCLOAK_ADMIN_EVENT: &str = "KEYCLOAK_ADMIN_EVENT";
/// Keycloak's event type of a sign-in.
const SIGN_IN_EVENT: &str = "LOGIN";
/// What the type of a sign-in's step ends with.
const SIGN_IN_STEP_SUFFIX: &str = "_STEP";
/// What the listener sends for a user id or a username it does not have.
const ABSENT: &str = "null";

/// Enqueue the electoral log event.
/// This task is routed to the durable electoral_log_event_queue, which only the dispatcher reads.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn enqueue_electoral_log_event(input: LogEventInput) -> Result<()> {
    // By calling this task, the event is enqueued into the electoral_log_event_queue.
    Ok(())
}

/// Process a batch of electoral log events.
///
/// Events that can never be stored as they are (an unknown election event, a malformed
/// body, no signing key) go to the dead-letter queue, and the rest of the batch is
/// appended, one transaction per board. Events whose signer cannot sign now wait,
/// durably and without a retry budget (`services::electoral_log_deferral`), with the
/// later events of their log behind them. Infrastructure failures fail the batch so that
/// it is retried; delivery IDs make the retry idempotent. The last retry dead-letters the
/// batch's other events instead of dropping them; the waiting ones are never
/// dead-lettered for running out of retries.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(
    bind = true,
    max_retries = 10,
    max_retry_delay = 60,
    retry_for_unexpected = true,
    acks_late = true
)]
pub async fn process_electoral_log_events_batch(
    task: &Self,
    events: Vec<LogEventInput>,
) -> Result<()> {
    match store_batch(&events).await {
        Ok(()) => Ok(()),
        Err(StoreFailure { error, waiting })
            if retries_exhausted(task.request().retries, task.max_retries()) =>
        {
            let reason = format!("retries exhausted: {error:#}");
            let set_aside = exhausted_events(&events, &waiting);
            // Events that were already waiting are never dead-lettered for this,
            // even if the run failed before it could put them back.
            let set_aside = match keep_waiting_after_last_retry(set_aside, &reason).await {
                Ok(rest) => rest,
                Err(keep_error) => {
                    tracing::error!(
                        "Could not keep the waiting electoral-log events after the last retry; they are dead-lettered instead: {keep_error:#}"
                    );
                    exhausted_events(&events, &waiting)
                }
            };
            tracing::error!(
                "Dead-lettering {} electoral-log events after the last retry ({} wait for their signer): {error:#}",
                set_aside.len(),
                events.len() - set_aside.len()
            );
            if !set_aside.is_empty() {
                let set_aside: Vec<(&LogEventInput, &str)> = set_aside
                    .into_iter()
                    .map(|event| (event, reason.as_str()))
                    .collect();
                dead_letter_events(&set_aside, DeadLetterStage::Batch).await?;
            }
            Ok(())
        }
        Err(failure) => Err(failure.error.into()),
    }
}

/// The events of a batch that the last retry dead-letters: all but those that
/// wait, durably, for their signer.
fn exhausted_events<'a>(
    events: &'a [LogEventInput],
    waiting: &HashSet<String>,
) -> Vec<&'a LogEventInput> {
    events
        .iter()
        .filter(|event| {
            event
                .delivery_id
                .as_ref()
                .map_or(true, |id| !waiting.contains(id))
        })
        .collect()
}

/// A batch that failed, and the delivery IDs of its events that wait durably for
/// their signer (put back on the event queue before it failed).
#[derive(Debug)]
struct StoreFailure {
    error: anyhow::Error,
    waiting: HashSet<String>,
}

impl From<anyhow::Error> for StoreFailure {
    fn from(error: anyhow::Error) -> Self {
        StoreFailure {
            error,
            waiting: HashSet::new(),
        }
    }
}

/// How far ahead of now an event's time may be (clock skew).
const MAX_EVENT_FUTURE_SECS: u64 = 300;
/// How long before an event first waited Keycloak may say it saw it.
const MAX_EVENT_LAG_SECS: u64 = 600;

/// When an election event's Keycloak event happened, for its statement: now,
/// for an event stored as it comes (as always); for one that waited for its
/// signer, when Keycloak saw it, if that is neither after now (beyond clock
/// skew) nor long before it first waited, and otherwise when it first waited.
fn statement_time(input: &LogEventInput, now: u64) -> u64 {
    let Some(deferral) = input.deferral else {
        return now;
    };
    let since = u64::try_from(deferral.since_ms / 1000).unwrap_or(now).min(now);
    input
        .event_time_ms
        .and_then(|millis| u64::try_from(millis / 1000).ok())
        .filter(|at| {
            *at <= now.saturating_add(MAX_EVENT_FUTURE_SECS)
                && at.saturating_add(MAX_EVENT_LAG_SECS) >= since
                && *at <= since.saturating_add(MAX_EVENT_FUTURE_SECS)
        })
        .unwrap_or(since)
}

/// After the last retry, of the events to dead-letter, those that were already
/// waiting (`marked`: they have a row in the waiting table, which only workers
/// write; a deferral in the message proves nothing) are kept to wait again
/// instead; the others are returned to be dead-lettered.
fn kept_after_last_retry<'a>(
    events: Vec<&'a LogEventInput>,
    marked: &HashSet<String>,
) -> (Vec<&'a LogEventInput>, Vec<&'a LogEventInput>) {
    events.into_iter().partition(|event| {
        event
            .delivery_id
            .as_ref()
            .is_some_and(|id| marked.contains(id))
    })
}

/// Puts back, to wait again, the events of a batch whose last retry failed
/// before (or without) putting them back, if they were already waiting; returns
/// the others, to be dead-lettered.
async fn keep_waiting_after_last_retry<'a>(
    events: Vec<&'a LogEventInput>,
    error: &str,
) -> anyhow::Result<Vec<&'a LogEventInput>> {
    let ids: Vec<String> = events
        .iter()
        .filter_map(|event| event.delivery_id.clone())
        .collect();
    let mut client: DbClient = get_queue_pool()
        .await
        .get()
        .await
        .context("Error getting a connection for the electoral-log event queue")?;
    let marked = waiting_ids(&**client, &ids).await?;
    let (keep, rest) = kept_after_last_retry(events, &marked);
    if !keep.is_empty() {
        let waiting: Vec<WaitingEvent> = keep
            .iter()
            .map(|event| WaitingEvent {
                event: (*event).clone(),
                reason: WaitReason::SignerUnavailable,
                error: format!("kept to wait again after the last retry: {error}"),
            })
            .collect();
        let backoff = DeferralBackoff::from_env().unwrap_or_default();
        let now_ms = chrono::Utc::now().timestamp_millis();
        let tx = client
            .transaction()
            .await
            .context("Error starting the electoral-log put-back transaction")?;
        put_back(
            &*tx,
            Queue::ElectoralLogEvent.queue_name(),
            &waiting,
            &Waiting::default(),
            &backoff,
            now_ms,
        )
        .await?;
        tx.commit()
            .await
            .context("Error committing the electoral-log events that wait")?;
        tracing::warn!(
            kept = keep.len(),
            "Electoral-log events that were waiting are kept to wait again after the last retry"
        );
    }
    Ok(rest)
}

/// Whether a failed run was the last one Celery would make.
fn retries_exhausted(retries: u32, max_retries: Option<u32>) -> bool {
    max_retries.is_some_and(|max| retries >= max)
}

/// Build the records of a batch, dead-letter the events that cannot be stored, put
/// back those that wait for their signer, and append the rest board by board.
async fn store_batch(events: &[LogEventInput]) -> std::result::Result<(), StoreFailure> {
    let backoff = DeferralBackoff::from_env()?;
    let event_queue = Queue::ElectoralLogEvent.queue_name();
    let mut queue_client: DbClient = get_queue_pool()
        .await
        .get()
        .await
        .context("Error getting a connection for the electoral-log event queue")?;
    // The entries already waiting in this batch's scopes: its events stay behind them.
    let mut scopes: Vec<String> = events.iter().map(scope_of).collect();
    scopes.sort();
    scopes.dedup();
    let waiting = waiting_events(&**queue_client, &scopes).await?;
    // Each event's deferral is its own row's, if it has one (never its message's),
    // and it doesn't wait behind its own row.
    let (bound, mut waiting) = bind_to_rows(events, waiting);
    let events = &bound[..];
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting DB pool for batch processing")?;
    let mut hasura_tx = hasura_db_client
        .transaction()
        .await
        .context("Error starting Hasura transaction")?;
    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .context("Error getting keycloak DB pool for batch processing")?;
    let keycloak_tx = keycloak_db_client
        .transaction()
        .await
        .context("Error starting keycloak transaction")?;

    // Entries an action wrote to its outbox and the relay hasn't moved yet come
    // first too: the batch's events of those logs wait behind them.
    let now_ms = chrono::Utc::now().timestamp_millis();
    for (delivery_id, scope, created_ms, seq) in
        crate::services::electoral_log_outbox::unrelayed_rows(&*hasura_tx, &scopes).await?
    {
        waiting.add(
            scope.clone(),
            crate::services::electoral_log_deferral::WaitKey {
                since_ms: created_ms,
                order: u32::try_from(seq).unwrap_or(u32::MAX),
                delivery_id: delivery_id.clone(),
            },
        );
        // Due once relayed (within a beat).
        waiting.add_due(scope, delivery_id, now_ms + 5_000);
    }
    crate::services::privacy_signin::allocate_registered_voters(&hasura_tx, &keycloak_tx, events)
        .await?;
    crate::services::privacy_signin::retain_queued_signin_network(&hasura_tx, events).await?;
    let built = build_batch_behind(
        &mut DbLookups {
            hasura: &hasura_tx,
            keycloak: &keycloak_tx,
            queue: &**queue_client,
        },
        events,
        &waiting,
    )
    .await?;
    count_login_attempts_apart(&mut hasura_tx, &built.login_attempts).await;
    hasura_tx
        .commit()
        .await
        .context("Error committing Hasura transaction")?;

    if !built.dead_letters.is_empty() {
        let mut set_aside = Vec::with_capacity(built.dead_letters.len());
        for (event, reason) in &built.dead_letters {
            tracing::error!(
                delivery_id = event.delivery_id.as_deref().unwrap_or_default(),
                election_event_id = event.election_event_id.as_str(),
                "Dead-lettering an electoral-log event: {reason}"
            );
            set_aside.push((event, reason.as_str()));
        }
        dead_letter_events(&set_aside, DeadLetterStage::Batch).await?;
        // A dead-lettered event no longer holds its log.
        let ids: Vec<String> = built
            .dead_letters
            .iter()
            .filter_map(|(event, _)| event.delivery_id.clone())
            .collect();
        stop_waiting(&**queue_client, &ids).await?;
    }
    stop_waiting(&**queue_client, &built.settled).await?;

    // The events that wait are put back first: whatever fails next, they are kept.
    let mut kept = HashSet::new();
    if !built.waiting.is_empty() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let tx = queue_client
            .transaction()
            .await
            .context("Error starting the electoral-log put-back transaction")?;
        let summary =
            put_back(&*tx, event_queue, &built.waiting, &waiting, &backoff, now_ms).await?;
        tx.commit()
            .await
            .context("Error committing the electoral-log events that wait")?;
        log_round(&summary, &waiting, &backoff, now_ms);
        kept = built
            .waiting
            .iter()
            .filter_map(|waiting| waiting.event.delivery_id.clone())
            .collect();
    }

    // Append every board before failing, so one failing board does not hold back the
    // others; a retry appends the failed boards again, idempotently.
    let client = get_board_client().await?;
    let mut failures = built.deferred;
    let mut stored_ids: HashSet<String> = HashSet::new();
    for (board, messages) in built.by_board {
        match client.append(&board, &messages).await {
            Ok(stored) => {
                emit_monitoring_lines(&board, &messages, &stored);
                // Stored: its entries no longer hold their log.
                let ids = built.delivered.get(&board).cloned().unwrap_or_default();
                stored_ids.extend(ids.iter().cloned());
                if let Err(error) = stop_waiting(&**queue_client, &ids).await {
                    failures.push(format!("{board}: {error:#}"));
                }
            }
            Err(error) => {
                tracing::error!("Error appending electoral-log batch for board {board}: {error:?}");
                failures.push(format!("{board}: {error:#}"));
            }
        }
    }
    // A log whose head was just stored, with nothing waiting for the signer
    // any more, drains its backlog now, in order.
    let released = crate::services::electoral_log_deferral::released_scopes(
        events,
        &stored_ids,
        &built.waiting,
    );
    if let Err(error) =
        crate::services::electoral_log_deferral::release_scopes(&**queue_client, event_queue, &released)
            .await
    {
        tracing::warn!("Error releasing the electoral-log entries waiting behind a stored one: {error:#}");
    }
    if !failures.is_empty() {
        return Err(StoreFailure {
            error: anyhow!(
                "Error appending electoral-log batches: {}",
                failures.join("; ")
            ),
            waiting: kept,
        });
    }
    Ok(())
}

/// Whether monitoring collects a record: every record of a tenant's log, and
/// the Keycloak admin events of an election event's.
fn is_monitored(board: &str, message: &ElectoralLogMessage) -> bool {
    matches!(owner_of_board(board), Ok(BoardOwner::Tenant(_)))
        || message.statement_kind == AuditStatementKind::KeycloakAdminEvent.to_string()
}

/// Writes the monitoring line of each record the board stored now; a record
/// it held already had its line when it was first stored.
fn emit_monitoring_lines(
    board: &str,
    messages: &[LogEntry],
    stored: &[electoral_log::StoredRecord],
) {
    for record in stored {
        let entry = messages
            .iter()
            .find(|entry| entry.delivery_id == record.delivery_id);
        if let Some(entry) = entry.filter(|entry| is_monitored(board, &entry.message)) {
            emit_monitoring_line(&monitoring_line(board, Some(record.id), &entry.message));
        }
    }
}

/// Why an event of a batch could not be turned into log records.
#[derive(Debug)]
enum EventFailure {
    /// The event can never be stored as it is; it goes to the dead-letter queue.
    Permanent(anyhow::Error),
    /// A database or other infrastructure failure; the whole batch is retried.
    Transient(anyhow::Error),
    /// The board's signer could not be loaded or refused (not an outage): the
    /// event waits for a retry while the batch's other events are stored.
    /// Nothing is dropped: the batch fails after storing the rest, so it is
    /// retried (stored events are not stored twice: delivery IDs); after the
    /// last retry the event goes to the dead-letter queue, whose reason names
    /// each deferred delivery, for an operator to replay or discard.
    Deferred(anyhow::Error),
    /// The board's signer cannot sign now ([`SignerUnavailable`]: an external
    /// signer whose key service is down): the event waits, durably and for as
    /// long as it takes, and is never dead-lettered for it.
    Unavailable(anyhow::Error),
}

/// How an event fails when making its record failed: a signer that cannot
/// sign right now ([`SignerUnavailable`], an external signer whose key service
/// is down) makes the event wait; any other error is the event's, and it is
/// dead-lettered.
fn failure(error: anyhow::Error) -> EventFailure {
    if is_signer_unavailable(&error) {
        EventFailure::Unavailable(error)
    } else {
        EventFailure::Permanent(error)
    }
}

/// How an event fails when its board's signer could not be loaded: it waits
/// if the signer is only unavailable, and is retried a bounded number of
/// times otherwise (a refused signer).
fn signer_failure(reason: &str, unavailable: bool) -> EventFailure {
    if unavailable {
        EventFailure::Unavailable(anyhow::Error::new(SignerUnavailable::new(reason)))
    } else {
        EventFailure::Deferred(anyhow!("{reason}"))
    }
}

/// Loads the board's signer into the batch's cache, once per batch. A lookup
/// that fails is remembered for the batch, with whether the signer is only
/// unavailable, so the board's other events do not ask again.
async fn load_signer<L: BatchLookups + Send>(
    lookups: &mut L,
    cache: &mut BatchCache,
    input: &LogEventInput,
    board: &str,
) -> std::result::Result<(), EventFailure> {
    let key = (input.tenant_id.clone(), input.election_event_id.clone());
    if let Some((reason, unavailable)) = cache.failed_signers.get(&key) {
        return Err(signer_failure(reason, *unavailable));
    }
    if !cache.signers.contains_key(&key) {
        match lookups
            .signer(&input.tenant_id, &input.election_event_id, board)
            .await
        {
            Ok(signer) => {
                cache.signers.insert(key, signer);
            }
            Err(error) => {
                let unavailable = is_signer_unavailable(&error);
                let reason = format!("{error:#}");
                let failure = signer_failure(&reason, unavailable);
                cache.failed_signers.insert(key, (reason, unavailable));
                return Err(failure);
            }
        }
    }
    Ok(())
}

/// The records of a batch, grouped by board, the events that cannot be stored,
/// and the sign-in attempts of the events stored, each under the SHA-256 of its
/// delivery ID (the monitoring counters' receipts take a 64-digit hex ID).
#[derive(Default)]
struct BuiltBatch {
    by_board: HashMap<String, Vec<LogEntry>>,
    dead_letters: Vec<(LogEventInput, String)>,
    login_attempts: Vec<(String, LoginAttempt)>,
    /// Events whose board's signer could not be loaded: retried, without holding
    /// back the rest.
    deferred: Vec<String>,
    /// Events that wait for their signer, or behind an earlier event of their log
    /// that does: put back on the event queue.
    waiting: Vec<WaitingEvent>,
    /// The delivery IDs of the events whose records go to each board.
    delivered: HashMap<String, Vec<String>>,
    /// The delivery IDs of the events built with no record to store.
    settled: Vec<String>,
}

/// The records of one event, and the sign-in attempt it is, if any.
struct BuiltEvent {
    entries: Vec<(String, LogEntry)>,
    login_attempt: Option<LoginAttempt>,
}

/// The ID a sign-in attempt is counted under: the SHA-256 of its delivery ID.
fn login_counter_delivery_id(delivery_id: &str) -> String {
    hex::encode(Sha256::digest(delivery_id.as_bytes()))
}

/// What building a batch needs from the databases. Lookups are made once per
/// election event and batch.
#[async_trait::async_trait]
trait BatchLookups {
    /// The electoral-log board of an election event, or `None` if the event does not
    /// exist or has no board.
    async fn board(
        &mut self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> anyhow::Result<Option<String>>;

    /// The area of a voter, if the voter has one.
    async fn user_area(
        &mut self,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
    ) -> anyhow::Result<Option<String>>;

    /// The system signer of a board, or `None` if the event has no protocol-manager key.
    async fn signer(
        &mut self,
        tenant_id: &str,
        election_event_id: &str,
        board: &str,
    ) -> anyhow::Result<Option<ElectoralLog>>;

    /// The board of the tenant's log and its system signer. The log is created
    /// the first time.
    async fn tenant_log(
        &mut self,
        tenant_id: &str,
    ) -> anyhow::Result<Option<(String, ElectoralLog)>>;

    /// The client whose service account Step uses on Keycloak's admin API.
    fn service_client(&self) -> String;

    /// The pending statement a worker queued under this delivery ID, if any
    /// (`services::electoral_log_pending`).
    async fn pending_statement(
        &mut self,
        delivery_id: &str,
    ) -> anyhow::Result<Option<crate::services::electoral_log_pending::StoredPending>>;

    /// The tenant's pending-statement MAC key, if it has one (vault).
    async fn pending_mac_key(&mut self, tenant_id: &str) -> anyhow::Result<Option<[u8; 32]>>;

    /// Whether the election event's board already holds a record under this
    /// delivery ID.
    async fn is_stored(
        &mut self,
        election_event_id: &str,
        board: &str,
        delivery_id: &str,
    ) -> anyhow::Result<bool>;

    /// Whether the tenant's log announces `public_key` (base64) as the signing
    /// key of administrator `user_id`: an `AdminPublicKey` entry for them,
    /// signed by the tenant log's system key.
    async fn admin_key_announced(
        &mut self,
        tenant_id: &str,
        user_id: &str,
        public_key: &str,
    ) -> anyhow::Result<bool>;
}

struct DbLookups<'a, 'b> {
    hasura: &'a Transaction<'b>,
    keycloak: &'a Transaction<'b>,
    /// The task-queue database, where the waiting table is.
    queue: &'a tokio_postgres::Client,
}

#[async_trait::async_trait]
impl BatchLookups for DbLookups<'_, '_> {
    async fn board(
        &mut self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> anyhow::Result<Option<String>> {
        Ok(
            get_election_event_by_id_if_exist(self.hasura, tenant_id, election_event_id)
                .await
                .context("Error getting election event")?
                .and_then(|event| get_election_event_board(event.bulletin_board_reference)),
        )
    }

    async fn user_area(
        &mut self,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
    ) -> anyhow::Result<Option<String>> {
        let realm = get_event_realm(tenant_id, election_event_id);
        get_user_area_id(self.keycloak, &realm, user_id)
            .await
            .context("Error getting user area id")
    }

    async fn signer(
        &mut self,
        tenant_id: &str,
        election_event_id: &str,
        board: &str,
    ) -> anyhow::Result<Option<ElectoralLog>> {
        // A board without any key is skipped, as before; one with a key is
        // signed through its identity (software or external), never skipped.
        let Some(system) = crate::services::electoral_log_board::get_board_signer_if_any(
            self.hasura,
            tenant_id,
            election_event_id,
            board,
        )
        .await
        .context("Error reading the board's signer")?
        else {
            return Ok(None);
        };
        Ok(Some(ElectoralLog::for_system_with_signer(board, system)))
    }

    async fn tenant_log(
        &mut self,
        tenant_id: &str,
    ) -> anyhow::Result<Option<(String, ElectoralLog)>> {
        // No log, and no database, is made for a tenant that does not exist.
        if get_tenant_by_id_if_exist(self.hasura, tenant_id)
            .await
            .context("Error getting the tenant")?
            .is_none()
        {
            return Ok(None);
        }
        let log = ensure_tenant_log(self.hasura, tenant_id)
            .await
            .context("Error getting the tenant log")?;
        let signer = ElectoralLog::for_system_with_signing_key(&log.board, &log.system_key);
        Ok(Some((log.board, signer)))
    }

    fn service_client(&self) -> String {
        std::env::var("KEYCLOAK_ADMIN_CLIENT_ID").unwrap_or_default()
    }

    async fn pending_statement(
        &mut self,
        delivery_id: &str,
    ) -> anyhow::Result<Option<crate::services::electoral_log_pending::StoredPending>> {
        crate::services::electoral_log_pending::load(self.queue, delivery_id).await
    }

    async fn pending_mac_key(&mut self, tenant_id: &str) -> anyhow::Result<Option<[u8; 32]>> {
        crate::services::vault::pending_mac_key(self.hasura, tenant_id).await
    }

    async fn is_stored(
        &mut self,
        election_event_id: &str,
        board: &str,
        delivery_id: &str,
    ) -> anyhow::Result<bool> {
        Ok(
            crate::services::electoral_log_board::get_event_store(election_event_id)
                .await?
                .delivered(board, delivery_id)
                .await?
                .is_some(),
        )
    }

    async fn admin_key_announced(
        &mut self,
        tenant_id: &str,
        user_id: &str,
        public_key: &str,
    ) -> anyhow::Result<bool> {
        use electoral_log::domain::{Filter, LogQuery, SqlCompOperators, TextColumn};
        use electoral_log::messages::statement::StatementType;
        let log = ensure_tenant_log(self.hasura, tenant_id)
            .await
            .context("Error getting the tenant log")?;
        let system_pk = voting_crypto::signing::VerificationKey::from_sk(&log.system_key)?;
        let mut query = LogQuery::default();
        query.filters.push(Filter::Text(
            TextColumn::StatementKind,
            SqlCompOperators::Equal,
            StatementType::AdminPublicKey.to_string(),
        ));
        query.filters.push(Filter::Text(
            TextColumn::UserId,
            SqlCompOperators::Equal,
            user_id.to_string(),
        ));
        let entries = get_board_client()
            .await?
            .query(&log.board, &query)
            .await
            .context("Error reading the tenant log's administrator keys")?;
        Ok(entries
            .iter()
            .any(|entry| announces_admin_key(entry, tenant_id, user_id, public_key, &system_pk)))
    }
}

/// Whether a tenant-log entry announces `public_key` as administrator
/// `user_id`'s key: an `AdminPublicKey` statement of that tenant and user,
/// whose signatures verify with the tenant log's system key.
fn announces_admin_key(
    entry: &ElectoralLogMessage,
    tenant_id: &str,
    user_id: &str,
    public_key: &str,
    system_pk: &voting_crypto::signing::VerificationKey,
) -> bool {
    use electoral_log::messages::statement::StatementBody;
    let Ok(message) = Message::record_deserialize(&entry.message) else {
        return false;
    };
    if message.verify(system_pk).is_err() {
        return false;
    }
    matches!(
        &message.statement.body,
        StatementBody::AdminPublicKey(tenant, Some(user), key)
            if tenant.0 == tenant_id && user == user_id && key.0 == public_key
    )
}

/// Lookups already made for this batch, by (tenant, election event), and the
/// tenants' logs by tenant.
#[derive(Default)]
struct BatchCache {
    boards: HashMap<(String, String), Option<String>>,
    signers: HashMap<(String, String), Option<ElectoralLog>>,
    /// Boards whose signer failed to load in this batch, why, and whether it is
    /// only unavailable.
    failed_signers: HashMap<(String, String), (String, bool)>,
    tenant_logs: HashMap<String, Option<(String, ElectoralLog)>>,
}

#[cfg(test)]
async fn build_batch<L: BatchLookups + Send>(
    lookups: &mut L,
    events: &[LogEventInput],
) -> anyhow::Result<BuiltBatch> {
    build_batch_behind(lookups, events, &Waiting::default()).await
}

/// Build a batch's records behind the events already `waiting`: an event waits
/// behind any event of its scope that started waiting before it, in the queue or
/// earlier in this batch, so a log's events keep their order.
async fn build_batch_behind<L: BatchLookups + Send>(
    lookups: &mut L,
    events: &[LogEventInput],
    waiting: &Waiting,
) -> anyhow::Result<BuiltBatch> {
    let mut cache = BatchCache::default();
    let mut built = BuiltBatch::default();
    // Scopes with an event that waits in this batch, and why.
    let mut held: HashMap<String, String> = HashMap::new();
    for original in in_wait_order(events) {
        let input = crate::services::privacy_signin::sanitized_event(original);
        let scope = scope_of(&input);
        // An event without a delivery ID is dead-lettered by `build_event`, never held.
        let has_delivery_id = input.delivery_id.as_deref().is_some_and(|id| !id.is_empty());
        let ahead = held.get(&scope).cloned().or_else(|| {
            waiting
                .ahead_of(&input)
                .map(|key| format!("delivery {:?} waits for its signer", key.delivery_id))
        });
        let ahead = ahead.filter(|_| has_delivery_id);
        if let Some(ahead) = ahead {
            built.waiting.push(WaitingEvent {
                event: input.clone(),
                reason: WaitReason::Held,
                error: format!("held behind an earlier event of its log: {ahead}"),
            });
            continue;
        }
        match build_event(lookups, &mut cache, &input).await {
            Ok(event) => {
                let delivery = input.delivery_id.clone().unwrap_or_default();
                if event.entries.is_empty() {
                    built.settled.push(delivery.clone());
                }
                for (board, entry) in event.entries {
                    let delivered = built.delivered.entry(board.clone()).or_default();
                    if !delivered.contains(&delivery) {
                        delivered.push(delivery.clone());
                    }
                    built.by_board.entry(board).or_default().push(entry);
                }
                if let (Some(attempt), Some(delivery_id)) =
                    (event.login_attempt, input.delivery_id.as_deref())
                {
                    built
                        .login_attempts
                        .push((login_counter_delivery_id(delivery_id), attempt));
                }
            }
            Err(EventFailure::Permanent(error)) => {
                built
                    .dead_letters
                    .push((input.clone(), format!("{error:#}")));
            }
            Err(EventFailure::Deferred(error)) => {
                tracing::error!(
                    delivery_id = input.delivery_id.as_deref().unwrap_or_default(),
                    "Deferring an electoral-log event: {error:#}"
                );
                built
                    .deferred
                    .push(format!("delivery {:?}: {error:#}", input.delivery_id));
            }
            Err(EventFailure::Unavailable(error)) => {
                let error = format!("{error:#}");
                tracing::warn!(
                    delivery_id = input.delivery_id.as_deref().unwrap_or_default(),
                    "An electoral-log event waits for its signer: {error}"
                );
                // Later events of its log wait behind it, without asking the signer.
                held.insert(scope, error.clone());
                built.waiting.push(WaitingEvent {
                    event: input.clone(),
                    reason: WaitReason::SignerUnavailable,
                    error,
                });
            }
            Err(EventFailure::Transient(error)) => {
                return Err(error.context(format!(
                    "Error building the record of delivery {:?}",
                    input.delivery_id
                )));
            }
        }
    }
    Ok(built)
}

async fn build_event<L: BatchLookups + Send>(
    lookups: &mut L,
    cache: &mut BatchCache,
    input: &LogEventInput,
) -> std::result::Result<BuiltEvent, EventFailure> {
    use EventFailure::{Permanent, Transient};

    let delivery_id = input
        .delivery_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| Permanent(anyhow!("Missing electoral-log delivery ID")))?;
    Uuid::parse_str(&input.tenant_id)
        .context("Invalid tenant ID")
        .map_err(failure)?;
    // The tenant realm's events and the requests made outside an election
    // event name none.
    if names_no_election_event(&input.election_event_id) {
        return build_tenant_event(lookups, cache, input, delivery_id, None).await;
    }
    Uuid::parse_str(&input.election_event_id)
        .context("Invalid election event ID")
        .map_err(failure)?;

    let key = (input.tenant_id.clone(), input.election_event_id.clone());
    if !cache.boards.contains_key(&key) {
        let board = lookups
            .board(&input.tenant_id, &input.election_event_id)
            .await
            .map_err(Transient)?;
        cache.boards.insert(key.clone(), board);
    }
    let Some(board) = cache.boards.get(&key).cloned().flatten() else {
        let reason = format!(
            "Election event {} does not exist or has no electoral-log board",
            input.election_event_id
        );
        return match input.message_type {
            LogMessageType::Internal | LogMessageType::TenantActivity => {
                Err(Permanent(anyhow!(reason)))
            }
            LogMessageType::KeycloakEvent(_) => {
                build_tenant_event(lookups, cache, input, delivery_id, Some(reason)).await
            }
        };
    };
    if input.message_type == LogMessageType::KeycloakEvent(KEYCLOAK_ADMIN_EVENT.to_string()) {
        return build_event_realm_admin_event(lookups, cache, input, delivery_id, board).await;
    }

    let mut entries = Vec::new();
    let mut attempt = None;
    let message = match &input.message_type {
        LogMessageType::TenantActivity => {
            return Err(Permanent(anyhow!(
                "Tenant activity belongs to the tenant's log, not to election event {}",
                input.election_event_id
            )));
        }
        LogMessageType::Internal
            if crate::services::electoral_log_pending::pending_ref(&input.body.as_raw())
                .is_some() =>
        {
            // A reference to an unsigned statement a worker queued in the waiting
            // table. Checked before anything is signed; one that fails is poison
            // (dead-lettered, never signed).
            let stored = lookups.pending_statement(delivery_id).await.map_err(Transient)?;
            // No row: if the board already holds this delivery, it was stored
            // (and its row removed) by an earlier run: settled, not poison.
            if stored.is_none()
                && lookups
                    .is_stored(&input.election_event_id, &board, delivery_id)
                    .await
                    .map_err(Transient)?
            {
                return Ok(BuiltEvent {
                    entries: vec![],
                    login_attempt: None,
                });
            }
            // A refused statement's content goes with its dead letter (its row is
            // removed then), for an operator to see and, if right, queue again.
            let statement_text = stored
                .as_ref()
                .map(|stored| stored.statement.to_string().chars().take(4_000).collect::<String>())
                .unwrap_or_else(|| "no row: forged, or already stored and removed".to_string());
            let mac_key = lookups
                .pending_mac_key(&input.tenant_id)
                .await
                .map_err(Transient)?;
            let pending = crate::services::electoral_log_pending::PendingStatement::read(
                input,
                stored,
                mac_key,
                electoral_log::timestamp(),
            )
            .with_context(|| format!("Refused pending statement {statement_text}"))
            .map_err(Permanent)?;
            // An administrator's signature is attested only for the key their
            // tenant log announces for them, never for a key the row names alone.
            if let Some((user_id, public_key)) = pending.statement.admin() {
                let announced = lookups
                    .admin_key_announced(&input.tenant_id, user_id, public_key)
                    .await
                    .map_err(Transient)?;
                if !announced {
                    // Not (yet) announced: the entry waits, never attested
                    // meanwhile (its relay announces a genuine key first). An
                    // operator may cancel it (`cancel_waiting_entry`).
                    tracing::error!(
                        alert = "electoral-log-admin-key-unannounced",
                        delivery_id,
                        scope = scope_of(input).as_str(),
                        user_id,
                        waiting_since_ms = input.deferral.map(|deferral| deferral.since_ms),
                        "An administrator-signed entry waits: the tenant log doesn't announce its key"
                    );
                    return Err(EventFailure::Unavailable(anyhow!(
                        "the tenant log doesn't announce administrator {user_id}'s key yet: the entry waits"
                    )));
                }
            }
            load_signer(lookups, cache, input, &board).await?;
            let signer = cache
                .signers
                .get(&key)
                .and_then(Option::as_ref)
                .ok_or_else(|| {
                    Permanent(anyhow!(
                        "Election event {} has no protocol-manager key to sign its pending statement",
                        input.election_event_id
                    ))
                })?;
            let message = pending
                .sign(&signer.sd)
                .context("Error signing a pending statement")
                .map_err(failure)?;
            let stored = ElectoralLogMessage::try_from(&message).map_err(failure)?;
            // Under its queued delivery ID, as the producer's own post would be.
            return Ok(BuiltEvent {
                entries: vec![(
                    board,
                    LogEntry {
                        delivery_id: delivery_id.to_string(),
                        message: stored,
                    },
                )],
                login_attempt: None,
            });
        }
        LogMessageType::Internal => deserialize_str::<ElectoralLogMessage>(&input.body.as_raw())
            .context("Error parsing the body as an ElectoralLogMessage")
            .map_err(failure)?,
        LogMessageType::KeycloakEvent(event_type) => {
            let user_id = input
                .user_id
                .clone()
                .unwrap_or_else(|| "unknown_user".into());
            let user_area_id = lookups
                .user_area(&input.tenant_id, &input.election_event_id, &user_id)
                .await
                .map_err(Transient)?;
            attempt = login_attempt(input, user_area_id.as_deref(), chrono::Utc::now());
            load_signer(lookups, cache, input, &board).await?;
            if cache.signers.get(&key).is_some_and(Option::is_none) {
                let reason = format!(
                    "Election event {} has no protocol-manager key",
                    input.election_event_id
                );
                return build_tenant_event(lookups, cache, input, delivery_id, Some(reason)).await;
            }
            let signer = cache
                .signers
                .get(&key)
                .and_then(Option::as_ref)
                .ok_or_else(|| Transient(anyhow!("The signer of a board was not loaded")))?;

            // An event that waited for its signer says when it happened, not
            // when it was finally signed.
            let at = statement_time(input, electoral_log::timestamp());
            if let LogEventBody::Communications(ref template_body) = input.body {
                let send_template_msg = electoral_log::with_timestamp(at, || {
                    signer.build_send_template_message(
                        Some(template_body.clone()),
                        input.election_event_id.clone(),
                        Some(user_id.clone()),
                        input.username.clone(),
                        None,
                        user_area_id.clone(),
                    )
                })
                .context("Error building send template message")
                .map_err(failure)?;
                entries.push((
                    board.clone(),
                    LogEntry {
                        delivery_id: format!("{delivery_id}:communication"),
                        message: send_template_msg,
                    },
                ));
            }

            electoral_log::with_timestamp(at, || {
                signer.build_keycloak_event_message(
                    input.election_event_id.clone(),
                    event_type.clone(),
                    input.body.as_raw(),
                    Some(user_id),
                    input.username.clone(),
                    user_area_id,
                )
            })
            .context("Error building keycloak event message")
            .map_err(failure)?
        }
    };
    entries.push((
        board,
        LogEntry {
            delivery_id: format!("{delivery_id}:event"),
            message,
        },
    ));
    Ok(BuiltEvent {
        entries,
        login_attempt: attempt,
    })
}

/// Who made a Keycloak admin event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum AdminEventAuthor {
    /// Step, through its service account: the change an admin made in the
    /// Admin Portal, as Keycloak saw it.
    ServiceAccount,
    /// Anyone else, in Keycloak's console or through its API.
    Operator,
}

/// Prefix Keycloak gives the username of a client's service account.
const SERVICE_ACCOUNT_PREFIX: &str = "service-account-";

/// Realm where Step's admin account lives when it signs in with a password.
const ADMIN_ACCOUNT_REALM: &str = "master";

/// Who made an admin event. Step acts on Keycloak's admin API as
/// `service_client` (`KEYCLOAK_ADMIN_CLIENT_ID`): the service account of that
/// client, or the master realm's user of that name, which is how
/// `KeycloakAdminClient` signs in (a password grant through `admin-cli`).
pub fn admin_event_author(admin_event: &Value, service_client: &str) -> AdminEventAuthor {
    let actor = &admin_event["actor"];
    let username = actor["username"].as_str().unwrap_or_default();
    let is_service = !service_client.is_empty()
        && (username.eq_ignore_ascii_case(&format!("{SERVICE_ACCOUNT_PREFIX}{service_client}"))
            || (username.eq_ignore_ascii_case(service_client)
                && actor["realm"].as_str() == Some(ADMIN_ACCOUNT_REALM)));
    if is_service {
        AdminEventAuthor::ServiceAccount
    } else {
        AdminEventAuthor::Operator
    }
}

/// A user id or username of the listener, which sends `"null"` for none.
fn present(value: &Option<String>) -> Option<String> {
    value
        .clone()
        .filter(|value| !value.is_empty() && value != ABSENT)
}

/// The record of a Keycloak admin event, signed by the log's system key: a
/// SYSTEM entry when Step's service account made it, a USER entry otherwise.
fn admin_event_message(
    signer: &ElectoralLog,
    statement_event: &str,
    input: &LogEventInput,
    delivery_id: &str,
    author: AdminEventAuthor,
) -> anyhow::Result<ElectoralLogMessage> {
    let admin_event = input
        .admin_event
        .as_ref()
        .ok_or_else(|| anyhow!("A Keycloak admin event without its details"))?;
    let mut details = admin_event.clone();
    details["made_by"] = json!(author.to_string());
    let text = |field: &str| admin_event[field].as_str().unwrap_or_default().to_string();
    let description = format!(
        "Keycloak: {} {} {}",
        text("operation"),
        text("resource_type"),
        text("resource_path")
    )
    .trim_end()
    .to_string();
    let timestamp = input
        .event_time_ms
        .and_then(|ms| u64::try_from(ms / 1000).ok())
        .unwrap_or_else(electoral_log::timestamp);
    let message = Message::audit_message(
        EventIdString(statement_event.to_string()),
        AuditLogEntry {
            kind: AuditStatementKind::KeycloakAdminEvent,
            event_type: match author {
                AdminEventAuthor::ServiceAccount => StatementEventType::SYSTEM,
                AdminEventAuthor::Operator => StatementEventType::USER,
            },
            log_type: if admin_event["error"].is_null() {
                StatementLogType::INFO
            } else {
                StatementLogType::ERROR
            },
            description,
            details_json: details.to_string(),
            entry_id: delivery_id.to_string(),
        },
        timestamp,
        &signer.sd,
        present(&input.user_id),
        present(&input.username),
        None,
        None,
    )?;
    ElectoralLogMessage::try_from(&message)
}

/// The type a tenant realm's event is logged under. An authenticator reports
/// the steps of a sign-in (a code sent, a code checked) as sign-ins of their
/// own before Keycloak reports the sign-in, which is the one that carries the
/// consent: the steps are logged as `<TYPE>_STEP`, so one sign-in reads as
/// one `LOGIN`.
fn tenant_event_type(event_type: &str, details: &Value) -> String {
    let reported = &details["details"];
    let is_step = event_type == SIGN_IN_EVENT
        && !reported["authenticator_class_name"].is_null()
        && reported["consent"].is_null();
    if is_step {
        format!("{event_type}{SIGN_IN_STEP_SUFFIX}")
    } else {
        event_type.to_string()
    }
}

/// The records of an event that belongs in the tenant's log: an event of the
/// tenant realm, or one that names an election event it cannot be placed in
/// (`unplaced` says why), which becomes an ERROR entry.
async fn build_tenant_event<L: BatchLookups + Send>(
    lookups: &mut L,
    cache: &mut BatchCache,
    input: &LogEventInput,
    delivery_id: &str,
    unplaced: Option<String>,
) -> std::result::Result<BuiltEvent, EventFailure> {
    use EventFailure::{Permanent, Transient};

    let event_type = match &input.message_type {
        LogMessageType::Internal => {
            return Err(Permanent(anyhow!(
                "An internal record needs an election event"
            )));
        }
        LogMessageType::TenantActivity => None,
        LogMessageType::KeycloakEvent(event_type) => Some(event_type),
    };
    if !cache.tenant_logs.contains_key(&input.tenant_id) {
        let log = lookups
            .tenant_log(&input.tenant_id)
            .await
            .map_err(Transient)?;
        cache.tenant_logs.insert(input.tenant_id.clone(), log);
    }
    let (board, signer) = cache
        .tenant_logs
        .get(&input.tenant_id)
        .and_then(Option::as_ref)
        .ok_or_else(|| Permanent(anyhow!("Tenant {} does not exist", input.tenant_id)))?;
    // When the producer saw the event; an event sent before producers said is
    // stamped as it is stored.
    let occurred_at = input
        .event_time_ms
        .and_then(|millis| u64::try_from(millis / 1000).ok())
        .unwrap_or_else(electoral_log::timestamp);

    let message = if let Some(event_type) = event_type
        .filter(|_| matches!(input.body, LogEventBody::Communications(_)) && unplaced.is_none())
    {
        // Keycloak reports a message it sent, such as a sign-in code, as an
        // event of the step that sent it, with the message as its body. The
        // entry says a message was sent and leaves its text out.
        let message = Message::keycloak_user_event_at(
            EventIdString(input.tenant_id.clone()),
            KeycloakEventTypeString(TENANT_MESSAGE_SENT_EVENT.to_string()),
            ErrorMessageString(json!({ "error": null, "step": event_type }).to_string()),
            occurred_at,
            present(&input.user_id),
            present(&input.username),
            &signer.sd,
            None,
        )
        .context("Error building the message-sent entry")
        .map_err(failure)?;
        ElectoralLogMessage::try_from(&message).map_err(failure)?
    } else if let Some(event_type) = event_type {
        build_tenant_keycloak_message(
            signer,
            event_type,
            input,
            delivery_id,
            &unplaced,
            occurred_at,
            &lookups.service_client(),
        )?
    } else {
        let activity = deserialize_str::<TenantActivity>(&input.body.as_raw())
            .context("Error parsing the body as a TenantActivity")
            .map_err(failure)?;
        tenant_activity_message(
            signer,
            &input.tenant_id,
            &activity,
            occurred_at,
            present(&input.user_id),
            present(&input.username),
        )
        .context("Error building the tenant activity message")
        .map_err(failure)?
    };
    Ok(BuiltEvent {
        entries: vec![(
            board.clone(),
            LogEntry {
                delivery_id: format!("{delivery_id}:event"),
                message,
            },
        )],
        login_attempt: None,
    })
}

/// The entry of a Keycloak event in the tenant's log: an admin event, or a
/// user event of the tenant realm or one that could not be placed.
fn build_tenant_keycloak_message(
    signer: &ElectoralLog,
    event_type: &str,
    input: &LogEventInput,
    delivery_id: &str,
    unplaced: &Option<String>,
    occurred_at: u64,
    service_client: &str,
) -> std::result::Result<ElectoralLogMessage, EventFailure> {
    let message = if event_type == KEYCLOAK_ADMIN_EVENT && unplaced.is_none() {
        let author = input
            .admin_event
            .as_ref()
            .map(|event| admin_event_author(event, service_client))
            .unwrap_or(AdminEventAuthor::Operator);
        admin_event_message(signer, &input.tenant_id, input, delivery_id, author)
            .map_err(failure)?
    } else {
        let mut details = input.details.clone().unwrap_or_else(|| {
            let error = Some(input.body.as_raw()).filter(|error| error != ABSENT);
            json!({ "error": error })
        });
        if let Some(reason) = unplaced {
            details["unplaced"] = json!({
                "election_event_id": input.election_event_id,
                "reason": reason,
            });
            if details["error"].is_null() {
                details["error"] = json!("event_not_placed");
            }
        }
        let message = Message::keycloak_user_event_at(
            EventIdString(input.tenant_id.clone()),
            KeycloakEventTypeString(tenant_event_type(event_type, &details)),
            ErrorMessageString(details.to_string()),
            occurred_at,
            present(&input.user_id),
            present(&input.username),
            &signer.sd,
            None,
        )
        .context("Error building keycloak event message")
        .map_err(failure)?;
        ElectoralLogMessage::try_from(&message).map_err(failure)?
    };
    Ok(message)
}

/// The record of an admin event on an election event's realm. Step's own
/// changes there (voters, their credentials) are recorded by what made them,
/// so only the changes made outside Step are.
async fn build_event_realm_admin_event<L: BatchLookups + Send>(
    lookups: &mut L,
    cache: &mut BatchCache,
    input: &LogEventInput,
    delivery_id: &str,
    board: String,
) -> std::result::Result<BuiltEvent, EventFailure> {
    let author = input
        .admin_event
        .as_ref()
        .map(|event| admin_event_author(event, &lookups.service_client()))
        .unwrap_or(AdminEventAuthor::Operator);
    if author == AdminEventAuthor::ServiceAccount {
        return Ok(BuiltEvent {
            entries: vec![],
            login_attempt: None,
        });
    }
    let key = (input.tenant_id.clone(), input.election_event_id.clone());
    load_signer(lookups, cache, input, &board).await?;
    let Some(signer) = cache.signers.get(&key).and_then(Option::as_ref) else {
        let reason = format!(
            "Election event {} has no protocol-manager key",
            input.election_event_id
        );
        return build_tenant_event(lookups, cache, input, delivery_id, Some(reason)).await;
    };
    let message = admin_event_message(signer, &input.election_event_id, input, delivery_id, author)
        .map_err(failure)?;
    Ok(BuiltEvent {
        entries: vec![(
            board,
            LogEntry {
                delivery_id: format!("{delivery_id}:event"),
                message,
            },
        )],
        login_attempt: None,
    })
}

/// Environment variable with the maximum number of events in one dispatcher batch.
pub const BATCH_SIZE_ENV: &str = "ELECTORAL_LOG_BATCH_SIZE";
/// Environment variable with the maximum payload bytes in one dispatcher batch.
pub const BATCH_MAX_BYTES_ENV: &str = "ELECTORAL_LOG_BATCH_MAX_BYTES";
/// Events per batch when `ELECTORAL_LOG_BATCH_SIZE` is not set.
pub const DEFAULT_BATCH_SIZE: usize = 1_000;
/// Payload bytes per batch when `ELECTORAL_LOG_BATCH_MAX_BYTES` is not set (16 MiB). A
/// batch task is one PGMQ message, so this also bounds the size of that message.
pub const DEFAULT_BATCH_MAX_BYTES: usize = 16 * 1024 * 1024;

/// How much the dispatcher puts in one batch task. A batch closes when it reaches
/// either limit; a single message larger than `max_bytes` still forms a batch on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchLimits {
    pub max_events: usize,
    pub max_bytes: usize,
}

impl BatchLimits {
    /// Read the limits from the environment. An unset or empty variable takes its
    /// default; any other value must be a positive integer.
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_values(
            std::env::var(BATCH_SIZE_ENV).ok().as_deref(),
            std::env::var(BATCH_MAX_BYTES_ENV).ok().as_deref(),
        )
    }

    fn from_values(max_events: Option<&str>, max_bytes: Option<&str>) -> anyhow::Result<Self> {
        let max_events = parse_limit(BATCH_SIZE_ENV, max_events, DEFAULT_BATCH_SIZE)?;
        anyhow::ensure!(
            i32::try_from(max_events).is_ok(),
            "{BATCH_SIZE_ENV} is too large for a PGMQ read, got {max_events}"
        );
        Ok(Self {
            max_events,
            max_bytes: parse_limit(BATCH_MAX_BYTES_ENV, max_bytes, DEFAULT_BATCH_MAX_BYTES)?,
        })
    }

    /// Whether a batch holding `events` messages with `bytes` payload bytes is full.
    pub fn is_full(&self, events: usize, bytes: usize) -> bool {
        events >= self.max_events || bytes >= self.max_bytes
    }
}

fn parse_limit(name: &str, value: Option<&str>, default: usize) -> anyhow::Result<usize> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(default),
        Some(value) => {
            let limit: usize = value
                .parse()
                .with_context(|| format!("{name} must be a positive integer, got {value:?}"))?;
            anyhow::ensure!(
                limit > 0,
                "{name} must be a positive integer, got {value:?}"
            );
            Ok(limit)
        }
    }
}

/// Dispatcher: repeatedly promotes batches of raw events from the electoral_log_event_queue to
/// processing tasks. Each handoff is one PostgreSQL transaction: the batch task is enqueued,
/// messages that cannot be parsed are dead-lettered and the batch's raw events are deleted
/// together, or not at all. Normal Celery consumers do not read the raw-event queue.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
/// Asks a worker to dispatch the electoral log's waiting events now (after an
/// action's entry was relayed). Fire and forget, bounded: beat dispatches
/// every few seconds anyway.
pub fn kick_electoral_log_batch_dispatcher() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let send = async {
            crate::services::celery_app::try_get_celery_app()
                .await?
                .send_task(electoral_log_batch_dispatcher::new())
                .await?;
            anyhow::Ok(())
        };
        match tokio::time::timeout(std::time::Duration::from_secs(5), send).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!("Error kicking the electoral-log dispatcher: {error:#}"),
            Err(_) => tracing::warn!("Kicking the electoral-log dispatcher timed out"),
        }
    })
}

#[celery::task(time_limit = 30, max_retries = 0, expires = 1)]
pub async fn electoral_log_batch_dispatcher() -> Result<()> {
    let limits = BatchLimits::from_env()?;
    let pool = get_queue_pool().await;
    // Waiting rows whose copy was lost hold their log: alerted on each run.
    match pool.get().await {
        Ok(client) => {
            if let Err(error) = crate::services::electoral_log_deferral::report_orphans(
                &**client,
                Queue::ElectoralLogEvent.queue_name(),
                ORPHAN_GRACE_SECS,
            )
            .await
            {
                tracing::warn!("Error looking for orphan electoral-log waiting rows: {error:#}");
            }
        }
        Err(error) => tracing::warn!("Error connecting to look for orphan waiting rows: {error}"),
    }
    dispatch_electoral_log_batches(pool.as_ref(), &DispatchQueues::default(), limits).await
}

/// How long past due a waiting row may be without its copy on the queue (a batch
/// still retrying it) before it is reported as an orphan.
const ORPHAN_GRACE_SECS: i32 = 900;

/// The queues a dispatcher run reads from and writes to.
struct DispatchQueues {
    events: String,
    batches: String,
    dead_letters: String,
}

impl Default for DispatchQueues {
    fn default() -> Self {
        DispatchQueues {
            events: Queue::ElectoralLogEvent.queue_name().into(),
            batches: Queue::ElectoralLogBatch.queue_name().into(),
            dead_letters: Queue::ElectoralLogDeadLetter.queue_name().into(),
        }
    }
}

async fn dispatch_electoral_log_batches(
    pool: &deadpool_postgres::Pool,
    queues: &DispatchQueues,
    limits: BatchLimits,
) -> Result<()> {
    let source = queues.events.as_str();
    let target = queues.batches.as_str();
    let dead_letters = queues.dead_letters.as_str();
    let max_events = i32::try_from(limits.max_events)
        .with_context(|| format!("{BATCH_SIZE_ENV} is too large for a PGMQ read"))?;
    loop {
        let mut client = pool
            .get()
            .await
            .context("Error obtaining PGMQ batch connection")?;
        let tx = client.transaction().await?;
        tx.batch_execute("SET LOCAL statement_timeout = '10s'")
            .await?;
        // pgmq.read retains row locks until commit, including while building the batch.
        let mut rows = tx
            .query(
                "SELECT msg_id, message, COALESCE(octet_length(message::text), 0) AS bytes \
                 FROM pgmq.read($1, 60, $2)",
                &[&source, &max_events],
            )
            .await?;
        if rows.is_empty() {
            break;
        }
        rows.sort_by_key(|row| row.get::<_, i64>("msg_id"));

        let mut events = Vec::with_capacity(rows.len());
        let mut taken: Vec<i64> = Vec::with_capacity(rows.len());
        let mut returned: Vec<i64> = Vec::new();
        let mut batch_bytes = 0usize;
        for row in rows {
            let id: i64 = row.get("msg_id");
            if limits.is_full(taken.len(), batch_bytes) {
                returned.push(id);
                continue;
            }
            let bytes: i32 = row.get("bytes");
            batch_bytes = batch_bytes.saturating_add(usize::try_from(bytes).unwrap_or_default());
            taken.push(id);
            let message = row
                .get::<_, Option<serde_json::Value>>("message")
                .unwrap_or(serde_json::Value::Null);
            // A message that cannot be parsed is dead-lettered, so it cannot block the queue.
            match parse_queued_event(message.clone()) {
                Ok(event) => events.push(crate::services::privacy_signin::sanitized_event(&event)),
                Err(_error) => {
                    tracing::error!(
                        message_id = id,
                        "Dead-lettering an electoral-log message that cannot be parsed"
                    );
                    dead_letter_message(
                        &*tx,
                        dead_letters,
                        &json!({"unparseable_event": true, "original_message_id": id}),
                        DeadLetterStage::Dispatcher,
                        "Invalid Celery envelope; payload omitted for privacy",
                    )
                    .await?;
                }
            }
        }
        info!(
            "dispatching a batch of {len} events, {batch_bytes} bytes (limits: {max_events} events, {max_bytes} bytes)",
            len = taken.len(),
            max_events = limits.max_events,
            max_bytes = limits.max_bytes,
        );

        if !events.is_empty() {
            let message = celery::protocol::Message::try_from(
                process_electoral_log_events_batch::new(events),
            )
            .context("Error encoding electoral-log batch")?;
            pgmq_broker::send(&*tx, target, &message)
                .await
                .context("Error enqueueing electoral-log batch")?;
        }
        tx.query("SELECT pgmq.delete($1, $2::bigint[])", &[&source, &taken])
            .await?;
        if !returned.is_empty() {
            // Messages beyond the batch limits are visible again for the next batch.
            tx.query(
                "SELECT pgmq.set_vt($1, $2::bigint[], 0)",
                &[&source, &returned],
            )
            .await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

/// Parse an event-queue message: the Celery envelope of an `enqueue_electoral_log_event`
/// task.
fn parse_queued_event(message: serde_json::Value) -> anyhow::Result<LogEventInput> {
    let message = pgmq_broker::decode(message).context("Error decoding the Celery envelope")?;
    anyhow::ensure!(
        message.headers.task == enqueue_electoral_log_event::NAME,
        "Unexpected raw-event task {:?}",
        message.headers.task
    );
    parse_event_body(&message.raw_body, Some(message.headers.id.as_str()))
}

/// Parse the body of an event-queue message, whose second element carries the event as
/// `input`.
fn parse_event_body(body: &[u8], celery_id: Option<&str>) -> anyhow::Result<LogEventInput> {
    let message: serde_json::Value =
        serde_json::from_slice(body).context("Error parsing Celery message as JSON")?;
    let input = message
        .as_array()
        .context("Invalid message format: expected a JSON array")?
        .get(1)
        .context("Invalid message format: expected an array with at least 2 elements")?
        .get("input")
        .context("Missing 'input' field in message payload")?;
    let mut event: LogEventInput = serde_json::from_value(input.clone())
        .context("Error deserializing LogEventInput from input field")?;
    retain_delivery_id(&mut event, celery_id)?;
    Ok(event)
}

/// Keycloak supplies the Celery task ID; internal producers persist an explicit ID.
fn retain_delivery_id(input: &mut LogEventInput, celery_id: Option<&str>) -> anyhow::Result<()> {
    if input.delivery_id.is_none() {
        input.delivery_id = Some(
            celery_id
                .context("Missing original Celery delivery ID")?
                .to_owned(),
        );
    }
    anyhow::ensure!(
        input
            .delivery_id
            .as_deref()
            .is_some_and(|id| !id.is_empty()),
        "Empty electoral-log delivery ID"
    );
    Ok(())
}

#[cfg(test)]
mod batch_limit_tests {
    use super::*;

    #[test]
    fn unset_or_empty_values_take_the_defaults() {
        let defaults = BatchLimits {
            max_events: DEFAULT_BATCH_SIZE,
            max_bytes: DEFAULT_BATCH_MAX_BYTES,
        };
        assert_eq!(BatchLimits::from_values(None, None).unwrap(), defaults);
        assert_eq!(
            BatchLimits::from_values(Some(""), Some("  ")).unwrap(),
            defaults
        );
        assert_eq!(DEFAULT_BATCH_MAX_BYTES, 16_777_216);
    }

    #[test]
    fn set_values_override_the_defaults() {
        let limits = BatchLimits::from_values(Some(" 250 "), Some("1048576")).unwrap();
        assert_eq!(limits.max_events, 250);
        assert_eq!(limits.max_bytes, 1_048_576);
    }

    #[test]
    fn invalid_values_are_rejected_with_the_variable_name() {
        for bad in ["0", "-1", "ten", "1.5", "99999999999999999999999"] {
            let error = BatchLimits::from_values(Some(bad), None).unwrap_err();
            assert!(error.to_string().contains(BATCH_SIZE_ENV), "{bad}: {error}");
            let error = BatchLimits::from_values(None, Some(bad)).unwrap_err();
            assert!(
                error.to_string().contains(BATCH_MAX_BYTES_ENV),
                "{bad}: {error}"
            );
        }
        let error = BatchLimits::from_values(Some("3000000000"), None).unwrap_err();
        assert!(error.to_string().contains(BATCH_SIZE_ENV), "{error}");
    }

    /// Mirrors the dispatcher loop: messages are taken until a limit is reached.
    fn split(limits: &BatchLimits, sizes: &[usize]) -> Vec<Vec<usize>> {
        let mut batches = Vec::new();
        let mut rest = sizes.iter().copied().peekable();
        while rest.peek().is_some() {
            let (mut batch, mut bytes) = (Vec::new(), 0usize);
            while !limits.is_full(batch.len(), bytes) {
                let Some(size) = rest.next() else { break };
                bytes += size;
                batch.push(size);
            }
            batches.push(batch);
        }
        batches
    }

    #[test]
    fn batches_close_at_the_event_limit() {
        let limits = BatchLimits {
            max_events: 3,
            max_bytes: usize::MAX,
        };
        assert_eq!(
            split(&limits, &[1; 7]),
            vec![vec![1, 1, 1], vec![1, 1, 1], vec![1]]
        );
    }

    #[test]
    fn batches_close_once_the_byte_limit_is_reached() {
        let limits = BatchLimits {
            max_events: 100,
            max_bytes: 10,
        };
        assert_eq!(
            split(&limits, &[4, 4, 4, 4, 2, 9]),
            vec![vec![4, 4, 4], vec![4, 2, 9]]
        );
    }

    #[test]
    fn an_oversized_message_forms_a_batch_on_its_own() {
        let limits = BatchLimits {
            max_events: 100,
            max_bytes: 10,
        };
        assert_eq!(split(&limits, &[50, 3]), vec![vec![50], vec![3]]);
    }
}

#[cfg(test)]
mod delivery_tests {
    use super::*;

    #[test]
    fn original_delivery_identity_survives_dispatch_and_task_retry() {
        let mut input = LogEventInput {
            delivery_id: None,
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            message_type: LogMessageType::KeycloakEvent("LOGIN".into()),
            user_id: None,
            username: None,
            body: LogEventBody::Plain("null".into()),
            event_time_ms: None,
            details: None,
            admin_event: None,
            deferral: None,
        };
        assert!(retain_delivery_id(&mut input, None).is_err());
        retain_delivery_id(&mut input, Some("original-id")).unwrap();
        let encoded = serde_json::to_vec(&input).unwrap();
        let mut retry: LogEventInput = serde_json::from_slice(&encoded).unwrap();
        retain_delivery_id(&mut retry, Some("retry-id")).unwrap();
        assert_eq!(retry.delivery_id.as_deref(), Some("original-id"));
        retry.delivery_id = Some(String::new());
        assert!(retain_delivery_id(&mut retry, None).is_err());
        assert_eq!(
            process_electoral_log_events_batch::DEFAULTS.acks_late,
            Some(true)
        );
        assert_eq!(
            process_electoral_log_events_batch::DEFAULTS.max_retries,
            Some(10)
        );
        assert_eq!(
            process_electoral_log_events_batch::DEFAULTS.max_retry_delay,
            Some(60)
        );
        assert_eq!(
            process_electoral_log_events_batch::DEFAULTS.retry_for_unexpected,
            Some(true)
        );
    }
}

#[cfg(test)]
mod batch_build_tests {
    use super::*;
    use crate::services::electoral_log_dead_letter::event_message;
    use electoral_log::messages::statement::{Statement, StatementBody};
    use crate::services::electoral_log_deferral::{plan, WaitKey};
    use crate::services::electoral_log_pending::StoredPending;
    use electoral_log::signer::{MessageSigner, SharedSigner, SignerUnavailable};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use voting_crypto::signing::{Signature, SigningKey, VerificationKey};
    use voting_crypto::RecordDeserialize;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const EVENT: &str = "fdd21db2-dd68-4974-90eb-7f2750b2b5df";
    const DELETED_EVENT: &str = "6f0c5c2e-3f43-4a5e-9d55-0d6a3e1f7b21";
    const KEYLESS_EVENT: &str = "2b8e4a1d-7c6f-4e93-b0a2-5d9c8f1e3a47";
    const UNKNOWN_TENANT: &str = "5a3c1f0e-9b7d-4c2a-8e6f-1d0b9a8c7e65";
    const REFUSED_SIGNER_EVENT: &str = "7d1e9c3b-2a4f-4b8e-9c6d-3e5f7a9b1c2d";
    const DOWN_SIGNER_EVENT: &str = "c4a7e2f1-8b3d-4e6a-9f0c-1b2d3e4f5a6b";
    /// An election event whose signer can't even be loaded: its key service is down.
    const DOWN_LOOKUP_EVENT: &str = "e8b1c3d5-6f7a-4b9c-8d2e-0f1a2b3c4d5e";

    /// How a loaded signer fails when it is asked to sign.
    #[derive(Clone, Copy)]
    enum SignerOutage {
        /// Its key service is down: try again later.
        Unavailable,
        /// It refuses to sign (another key): final.
        Refused,
        /// It signs, but the answer is lost on the way back while
        /// `FakeLookups::lost_answers` is above zero (the service may have
        /// signed: an uncertain outcome).
        LostAnswers,
    }

    /// A signer that is loaded (its public key is known) but cannot sign.
    struct FailingSigner {
        key: SigningKey,
        outage: SignerOutage,
        lost_answers: Arc<AtomicUsize>,
        sign_calls: Arc<AtomicUsize>,
    }

    impl MessageSigner for FailingSigner {
        fn public_key(&self) -> anyhow::Result<VerificationKey> {
            VerificationKey::from_sk(&self.key)
        }

        fn sign(&self, bytes: &[u8]) -> anyhow::Result<Signature> {
            self.sign_calls.fetch_add(1, Ordering::SeqCst);
            match self.outage {
                SignerOutage::Unavailable => Err(SignerUnavailable::new(
                    "the key service is unreachable: error sending request",
                )
                .into()),
                SignerOutage::Refused => Err(anyhow!(
                    "the key service refused: the key does not match (refused)"
                )),
                SignerOutage::LostAnswers => {
                    let signature = self.key.sign(bytes)?;
                    let lost = self
                        .lost_answers
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                            left.checked_sub(1)
                        })
                        .is_ok();
                    if lost {
                        Err(SignerUnavailable::new(
                            "the key service is unreachable: connection reset before the answer",
                        )
                        .into())
                    } else {
                        Ok(signature)
                    }
                }
            }
        }
    }

    struct FakeLookups {
        signing_key: SigningKey,
        unreachable_event: Option<&'static str>,
        unreachable_tenant: Option<&'static str>,
        /// Election events and tenants whose signer loads but cannot sign.
        failing_signers: HashMap<&'static str, SignerOutage>,
        /// Election events whose signer can't be loaded: its key service is down.
        unavailable_lookups: HashSet<&'static str>,
        /// Answers `SignerOutage::LostAnswers` signers still lose.
        lost_answers: Arc<AtomicUsize>,
        /// Signing calls the failing signers got.
        sign_calls: Arc<AtomicUsize>,
        /// Pending statements as workers wrote them, by delivery ID.
        pending: HashMap<String, StoredPending>,
        /// Administrator keys the tenant log announces: (user ID, key).
        announced: HashSet<(String, String)>,
        /// Delivery IDs the boards already hold.
        stored_deliveries: HashSet<String>,
        board_calls: usize,
        signer_calls: usize,
        tenant_log_calls: usize,
    }

    impl FakeLookups {
        fn new() -> Self {
            Self {
                signing_key: SigningKey::generate().unwrap(),
                unreachable_event: None,
                unreachable_tenant: None,
                failing_signers: HashMap::new(),
                unavailable_lookups: HashSet::new(),
                lost_answers: Arc::new(AtomicUsize::new(0)),
                sign_calls: Arc::new(AtomicUsize::new(0)),
                pending: HashMap::new(),
                announced: HashSet::new(),
                stored_deliveries: HashSet::new(),
                board_calls: 0,
                signer_calls: 0,
                tenant_log_calls: 0,
            }
        }

        /// The signer of an election event or tenant that cannot sign, if it is one.
        fn failing_signer(&self, id: &str) -> Option<SharedSigner> {
            let outage = *self.failing_signers.get(id)?;
            Some(Arc::new(FailingSigner {
                key: self.signing_key.clone(),
                outage,
                lost_answers: self.lost_answers.clone(),
                sign_calls: self.sign_calls.clone(),
            }))
        }
    }

    #[async_trait::async_trait]
    impl BatchLookups for FakeLookups {
        async fn board(
            &mut self,
            _tenant_id: &str,
            election_event_id: &str,
        ) -> anyhow::Result<Option<String>> {
            self.board_calls += 1;
            if self.unreachable_event == Some(election_event_id) {
                return Err(anyhow!("connection reset"));
            }
            Ok((election_event_id != DELETED_EVENT).then(|| format!("board-{election_event_id}")))
        }

        async fn user_area(
            &mut self,
            _tenant_id: &str,
            _election_event_id: &str,
            _user_id: &str,
        ) -> anyhow::Result<Option<String>> {
            Ok(None)
        }

        async fn signer(
            &mut self,
            _tenant_id: &str,
            election_event_id: &str,
            board: &str,
        ) -> anyhow::Result<Option<ElectoralLog>> {
            self.signer_calls += 1;
            if self.unavailable_lookups.contains(election_event_id) {
                return Err(anyhow::Error::new(SignerUnavailable::new(
                    "the key service is unreachable: error sending request",
                ))
                .context("Error reading the board's signer"));
            }
            if election_event_id == REFUSED_SIGNER_EVENT {
                return Err(anyhow!(
                    "this external board has no key: its creation failed and must be repeated (refused)"
                ));
            }
            if let Some(signer) = self.failing_signer(election_event_id) {
                return Ok(Some(ElectoralLog::for_system_with_signer(board, signer)));
            }
            Ok((election_event_id != KEYLESS_EVENT)
                .then(|| ElectoralLog::for_system_with_signing_key(board, &self.signing_key)))
        }

        async fn tenant_log(
            &mut self,
            tenant_id: &str,
        ) -> anyhow::Result<Option<(String, ElectoralLog)>> {
            self.tenant_log_calls += 1;
            if self.unreachable_tenant == Some(tenant_id) {
                return Err(anyhow!("connection reset"));
            }
            let board = format!("tenant-board-{tenant_id}");
            let signer = match self.failing_signer(tenant_id) {
                Some(signer) => ElectoralLog::for_system_with_signer(&board, signer),
                None => ElectoralLog::for_system_with_signing_key(&board, &self.signing_key),
            };
            Ok((tenant_id != UNKNOWN_TENANT).then_some((board, signer)))
        }

        fn service_client(&self) -> String {
            "step".to_string()
        }

        async fn pending_statement(
            &mut self,
            delivery_id: &str,
        ) -> anyhow::Result<Option<StoredPending>> {
            Ok(self.pending.get(delivery_id).cloned())
        }

        async fn pending_mac_key(&mut self, _tenant_id: &str) -> anyhow::Result<Option<[u8; 32]>> {
            Ok(Some(TEST_MAC_KEY))
        }

        async fn is_stored(
            &mut self,
            _election_event_id: &str,
            _board: &str,
            delivery_id: &str,
        ) -> anyhow::Result<bool> {
            Ok(self.stored_deliveries.contains(delivery_id))
        }

        async fn admin_key_announced(
            &mut self,
            _tenant_id: &str,
            user_id: &str,
            public_key: &str,
        ) -> anyhow::Result<bool> {
            Ok(self
                .announced
                .contains(&(user_id.to_string(), public_key.to_string())))
        }
    }

    fn keycloak_event(delivery_id: Option<&str>, election_event_id: &str) -> LogEventInput {
        LogEventInput {
            delivery_id: delivery_id.map(str::to_string),
            election_event_id: election_event_id.into(),
            message_type: LogMessageType::KeycloakEvent("LOGIN".into()),
            user_id: Some("voter".into()),
            username: Some("voter".into()),
            tenant_id: TENANT.into(),
            body: LogEventBody::Plain("{}".into()),
            event_time_ms: None,
            details: None,
            admin_event: None,
            deferral: None,
        }
    }

    fn internal_event(delivery_id: &str, body: String) -> LogEventInput {
        LogEventInput {
            message_type: LogMessageType::Internal,
            user_id: None,
            username: None,
            body: LogEventBody::Plain(body),
            ..keycloak_event(Some(delivery_id), EVENT)
        }
    }

    fn internal_message() -> String {
        serde_json::to_string(&ElectoralLogMessage {
            id: 0,
            created: 0,
            sender_pk: "pk".into(),
            statement_timestamp: 0,
            statement_kind: "kind".into(),
            message: vec![1, 2, 3],
            version: "1".into(),
            user_id: None,
            username: None,
            election_id: None,
            area_id: None,
            ballot_id: None,
        })
        .unwrap()
    }

    fn delivery_ids(entries: &[LogEntry]) -> Vec<&str> {
        entries
            .iter()
            .map(|entry| entry.delivery_id.as_str())
            .collect()
    }

    fn dead_letter_ids(built: &BuiltBatch) -> Vec<Option<&str>> {
        built
            .dead_letters
            .iter()
            .map(|(event, _)| event.delivery_id.as_deref())
            .collect()
    }

    #[tokio::test]
    async fn a_board_whose_signer_fails_is_deferred_without_holding_back_other_events() {
        let mut lookups = FakeLookups::new();
        let events = vec![
            keycloak_event(Some("ok-1"), EVENT),
            keycloak_event(Some("refused-1"), REFUSED_SIGNER_EVENT),
            keycloak_event(Some("ok-2"), EVENT),
            keycloak_event(Some("refused-2"), REFUSED_SIGNER_EVENT),
        ];
        let built = build_batch(&mut lookups, &events).await.unwrap();
        // The other event's entries are built, in order.
        let stored: Vec<&str> = built
            .by_board
            .values()
            .flat_map(|entries| delivery_ids(entries))
            .collect();
        assert_eq!(
            delivery_ids(&built.by_board[&format!("board-{EVENT}")]),
            vec!["ok-1:event", "ok-2:event"]
        );
        assert!(
            !stored.iter().any(|id| id.starts_with("refused")),
            "{stored:?}"
        );
        // The refused board's events wait for a retry: neither stored as
        // something else, nor dead-lettered, nor skipped.
        assert!(built.dead_letters.is_empty());
        assert_eq!(built.deferred.len(), 2, "{:?}", built.deferred);
        assert!(built
            .deferred
            .iter()
            .all(|reason| reason.contains("refused")));
        assert!(
            !built
                .by_board
                .keys()
                .any(|board| board.starts_with("tenant-board")),
            "not moved to the tenant log"
        );
        // The failing signer is asked once per batch.
        assert_eq!(lookups.signer_calls, 2, "one lookup per event board");
    }

    #[tokio::test]
    async fn bad_events_are_dead_lettered_and_the_rest_is_stored() {
        let mut communication = keycloak_event(Some("d2"), EVENT);
        communication.body = LogEventBody::Communications("hello".into());
        let mut invalid_tenant = keycloak_event(Some("d6"), EVENT);
        invalid_tenant.tenant_id = "not-a-uuid".into();
        let events = vec![
            keycloak_event(Some("d1"), EVENT),
            communication,
            internal_event("d3", internal_message()),
            internal_event("d4", "not a log message".into()),
            keycloak_event(Some("d5"), DELETED_EVENT),
            invalid_tenant,
            keycloak_event(None, EVENT),
            keycloak_event(Some(""), EVENT),
            keycloak_event(Some("d7"), KEYLESS_EVENT),
            keycloak_event(Some("d8"), EVENT),
        ];

        let mut lookups = FakeLookups::new();
        let built = build_batch(&mut lookups, &events).await.unwrap();

        assert_eq!(built.by_board.len(), 2);
        assert_eq!(
            delivery_ids(&built.by_board[&format!("board-{EVENT}")]),
            vec![
                "d1:event",
                "d2:communication",
                "d2:event",
                "d3:event",
                "d8:event"
            ]
        );
        // Unreadable messages are set aside.
        assert_eq!(
            dead_letter_ids(&built),
            vec![Some("d4"), Some("d6"), None, Some("")]
        );
        let reasons: Vec<&str> = built
            .dead_letters
            .iter()
            .map(|(_, reason)| reason.as_str())
            .collect();
        assert!(reasons[0].contains("ElectoralLogMessage"), "{}", reasons[0]);
        assert!(reasons[1].contains("Invalid tenant ID"), "{}", reasons[1]);
        assert!(reasons[2].contains("delivery ID"), "{}", reasons[2]);
        assert!(reasons[3].contains("delivery ID"), "{}", reasons[3]);

        // Events that can't be placed in their election event's log are ERROR
        // entries of the tenant's log.
        let unplaced = &built.by_board[&format!("tenant-board-{TENANT}")];
        assert_eq!(delivery_ids(unplaced), vec!["d5:event", "d7:event"]);
        let (statement, text) = keycloak_statement(&unplaced[0]);
        assert_eq!(statement.head.log_type, StatementLogType::ERROR);
        assert_eq!(statement.head.description, "LOGIN {}");
        assert_eq!(text["unplaced"]["election_event_id"], DELETED_EVENT);
        assert!(text["unplaced"]["reason"]
            .as_str()
            .unwrap()
            .contains("does not exist"));
        let (_, text) = keycloak_statement(&unplaced[1]);
        assert!(text["unplaced"]["reason"]
            .as_str()
            .unwrap()
            .contains("protocol-manager key"));
    }

    /// The statement of a Keycloak user event record, and its text as JSON.
    fn keycloak_statement(entry: &LogEntry) -> (Statement, Value) {
        let message = Message::record_deserialize(&entry.message.message).unwrap();
        let StatementBody::KeycloakUserEvent(text, _) = &message.statement.body else {
            panic!("not a Keycloak user event");
        };
        let text = serde_json::from_str(&text.0).unwrap();
        (message.statement, text)
    }

    fn tenant_event(delivery_id: &str, event_type: &str, details: Value) -> LogEventInput {
        LogEventInput {
            message_type: LogMessageType::KeycloakEvent(event_type.into()),
            user_id: Some("admin-id".into()),
            username: Some("admin".into()),
            body: LogEventBody::Plain("null".into()),
            details: Some(details),
            ..keycloak_event(Some(delivery_id), "")
        }
    }

    fn admin_event(delivery_id: &str, election_event_id: &str, username: &str) -> LogEventInput {
        LogEventInput {
            message_type: LogMessageType::KeycloakEvent(KEYCLOAK_ADMIN_EVENT.into()),
            user_id: Some("actor-id".into()),
            username: Some(username.into()),
            body: LogEventBody::Plain("null".into()),
            event_time_ms: Some(1_778_511_600_123),
            admin_event: Some(json!({
                "operation": "CREATE",
                "resource_type": "REALM_ROLE_MAPPING",
                "resource_path": "users/42/role-mappings/realm",
                "representation": [{"name": "tenant-write"}],
                "error": null,
                "actor": {"user_id": "actor-id", "username": username,
                    "client_id": "security-admin-console", "realm": "master",
                    "ip_address": "10.0.0.7"},
            })),
            ..keycloak_event(Some(delivery_id), election_event_id)
        }
    }

    fn audit_entry(entry: &LogEntry) -> (Message, AuditLogEntry, Value) {
        let message = Message::record_deserialize(&entry.message.message).unwrap();
        let StatementBody::Audit(audit) = &message.statement.body else {
            panic!("not an audit entry");
        };
        let audit = audit.clone();
        let details = serde_json::from_str(&audit.details_json).unwrap();
        (message, audit, details)
    }

    fn tenant_activity(delivery_id: &str, body: &str) -> LogEventInput {
        LogEventInput {
            election_event_id: String::new(),
            message_type: LogMessageType::TenantActivity,
            user_id: Some("admin-id".into()),
            username: Some("admin".into()),
            body: LogEventBody::Plain(body.into()),
            event_time_ms: Some(1_800_000_000_123),
            ..keycloak_event(Some(delivery_id), EVENT)
        }
    }

    const DELETED_AN_EVENT: &str = r#"{"action":"delete-election-event","description":"Deleted an election event","outcome":"succeeded","status":200}"#;

    #[tokio::test]
    async fn requests_and_sent_messages_of_no_election_event_go_to_their_tenants_log() {
        let mut sign_in = keycloak_event(Some("t2"), "null");
        sign_in.message_type = LogMessageType::KeycloakEvent("LOGIN_ERROR".into());
        sign_in.body = LogEventBody::Plain("invalid_user_credentials".into());
        sign_in.user_id = None;
        sign_in.event_time_ms = Some(1_799_999_000_500);
        let mut unknown_tenant = tenant_activity("t5", DELETED_AN_EVENT);
        unknown_tenant.tenant_id = UNKNOWN_TENANT.into();
        let mut for_an_event = tenant_activity("t6", DELETED_AN_EVENT);
        for_an_event.election_event_id = EVENT.into();
        let mut untimed = tenant_activity("t8", DELETED_AN_EVENT);
        untimed.event_time_ms = None;
        let mut code_sent = keycloak_event(Some("t9"), "");
        code_sent.message_type = LogMessageType::KeycloakEvent("CODE_TO_TOKEN".into());
        code_sent.body = LogEventBody::Communications("Your code is 123456".into());
        let events = vec![
            tenant_activity("t1", DELETED_AN_EVENT),
            sign_in,
            keycloak_event(Some("d1"), EVENT),
            tenant_activity("t4", "not an activity"),
            unknown_tenant,
            for_an_event,
            untimed,
            code_sent,
        ];

        let mut lookups = FakeLookups::new();
        let before = electoral_log::timestamp() as i64;
        let built = build_batch(&mut lookups, &events).await.unwrap();

        assert_eq!(
            delivery_ids(&built.by_board[&format!("board-{EVENT}")]),
            vec!["d1:event"]
        );
        let stored = &built.by_board[&format!("tenant-board-{TENANT}")];
        assert_eq!(
            delivery_ids(stored),
            vec!["t1:event", "t2:event", "t8:event", "t9:event"]
        );
        assert_eq!(stored[0].message.statement_kind, "TenantAdminAction");
        assert_eq!(stored[0].message.statement_timestamp, 1_800_000_000);
        assert_eq!(stored[0].message.user_id.as_deref(), Some("admin-id"));
        // A tenant realm's event keeps the time Keycloak saw it.
        assert_eq!(stored[1].message.statement_kind, "KeycloakUserEvent");
        assert_eq!(stored[1].message.statement_timestamp, 1_799_999_000);
        assert!(stored[2].message.statement_timestamp >= before);
        // A sent message is an entry of its own, without the message's text.
        let sent = Message::record_deserialize(&stored[3].message.message).unwrap();
        assert_eq!(sent.statement.head.description, "MESSAGE_SENT");
        assert_eq!(sent.statement.head.log_type, StatementLogType::INFO);
        assert!(!sent.to_string().contains("123456"));
        assert_eq!(lookups.tenant_log_calls, 2);

        assert_eq!(
            dead_letter_ids(&built),
            vec![Some("t4"), Some("t5"), Some("t6")]
        );
        let reasons: Vec<&str> = built
            .dead_letters
            .iter()
            .map(|(_, reason)| reason.as_str())
            .collect();
        assert!(reasons[0].contains("TenantActivity"), "{}", reasons[0]);
        assert!(reasons[1].contains("does not exist"), "{}", reasons[1]);
        assert!(reasons[2].contains("tenant's log"), "{}", reasons[2]);
    }

    fn waiting_ids(built: &BuiltBatch) -> Vec<(&str, WaitReason)> {
        built
            .waiting
            .iter()
            .map(|waiting| {
                (
                    waiting.event.delivery_id.as_deref().unwrap_or_default(),
                    waiting.reason,
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn each_kind_of_event_waits_while_its_signer_is_unavailable() {
        let mut communication = keycloak_event(Some("c1"), DOWN_SIGNER_EVENT);
        communication.body = LogEventBody::Communications("hello".into());
        let mut sent = tenant_event("t3", "SEND_CODE", json!({"error": null}));
        sent.body = LogEventBody::Communications("your code".into());
        // Each alone, so that each is built (not held behind another).
        let cases = vec![
            (keycloak_event(Some("k1"), DOWN_SIGNER_EVENT), "Error building keycloak event message"),
            (communication, "Error building send template message"),
            (admin_event("a1", DOWN_SIGNER_EVENT, "operator"), "the key service is unreachable"),
            (tenant_event("t1", "LOGIN", json!({"error": null})), "Error building keycloak event message"),
            (tenant_activity("t2", DELETED_AN_EVENT), "Error building the tenant activity message"),
            (sent, "Error building the message-sent entry"),
            (keycloak_event(Some("t4"), DELETED_EVENT), "Error building keycloak event message"),
        ];
        for (event, context) in cases {
            let mut lookups = FakeLookups::new();
            lookups
                .failing_signers
                .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
            lookups
                .failing_signers
                .insert(TENANT, SignerOutage::Unavailable);
            let delivery = event.delivery_id.clone().unwrap();
            let built = build_batch(&mut lookups, &[event]).await.unwrap();
            assert!(built.dead_letters.is_empty(), "{delivery}: {:?}", dead_letter_ids(&built));
            assert!(built.deferred.is_empty(), "{delivery}: bounded retries are not used");
            assert!(built.by_board.is_empty(), "{delivery}");
            assert_eq!(
                waiting_ids(&built),
                vec![(delivery.as_str(), WaitReason::SignerUnavailable)]
            );
            let error = &built.waiting[0].error;
            assert!(error.contains(context), "{delivery}: {error}");
            assert!(error.contains("the key service is unreachable"), "{delivery}: {error}");
        }
    }

    #[tokio::test]
    async fn later_events_of_a_log_wait_behind_one_whose_signer_is_unavailable() {
        let events = vec![
            keycloak_event(Some("down-1"), DOWN_SIGNER_EVENT),
            keycloak_event(Some("ok-1"), EVENT),
            keycloak_event(Some("down-2"), DOWN_SIGNER_EVENT),
            admin_event("down-3", DOWN_SIGNER_EVENT, "operator"),
        ];
        let mut lookups = FakeLookups::new();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);

        let built = build_batch(&mut lookups, &events).await.unwrap();

        assert!(built.dead_letters.is_empty());
        assert!(built.deferred.is_empty());
        assert_eq!(
            waiting_ids(&built),
            vec![
                ("down-1", WaitReason::SignerUnavailable),
                ("down-2", WaitReason::Held),
                ("down-3", WaitReason::Held),
            ]
        );
        assert!(built.waiting[1].error.contains("held behind"));
        assert_eq!(
            delivery_ids(&built.by_board[&format!("board-{EVENT}")]),
            vec!["ok-1:event"]
        );
        // The signer was asked once: the held events did not call the key service.
        assert_eq!(lookups.sign_calls.load(Ordering::SeqCst), 1);
        assert_eq!(lookups.signer_calls, 2, "one lookup per board");
    }

    #[tokio::test]
    async fn an_event_waits_behind_an_earlier_one_of_its_log_in_the_queue() {
        let earlier = LogEventInput {
            deferral: Some(Deferral {
                rounds: 3,
                since_ms: 1_000,
                order: 0,
            }),
            ..keycloak_event(Some("earlier"), EVENT)
        };
        let mut waiting = Waiting::default();
        waiting.add(scope_of(&earlier), WaitKey::of(&earlier));
        let events = vec![
            keycloak_event(Some("new"), EVENT),
            keycloak_event(Some("other-log"), KEYLESS_EVENT),
            // A copy of the waiting event is not held behind itself.
            earlier.clone(),
        ];
        let mut lookups = FakeLookups::new();

        let built = build_batch_behind(&mut lookups, &events, &waiting)
            .await
            .unwrap();

        assert_eq!(waiting_ids(&built), vec![("new", WaitReason::Held)]);
        assert!(built.waiting[0].error.contains("\"earlier\""), "{}", built.waiting[0].error);
        assert_eq!(
            delivery_ids(&built.by_board[&format!("board-{EVENT}")]),
            vec!["earlier:event"]
        );
    }

    #[tokio::test]
    async fn a_signer_that_cannot_be_loaded_now_waits_and_a_refused_one_is_retried_then_dead_lettered() {
        let mut lookups = FakeLookups::new();
        lookups.unavailable_lookups.insert(DOWN_LOOKUP_EVENT);
        let events = vec![
            keycloak_event(Some("k1"), DOWN_LOOKUP_EVENT),
            admin_event("a1", REFUSED_SIGNER_EVENT, "operator"),
            keycloak_event(Some("k2"), REFUSED_SIGNER_EVENT),
            keycloak_event(Some("ok"), EVENT),
        ];

        let built = build_batch(&mut lookups, &events).await.unwrap();

        // Unavailable at lookup: waits (typed, not rebuilt from text).
        assert_eq!(waiting_ids(&built), vec![("k1", WaitReason::SignerUnavailable)]);
        assert!(built.waiting[0].error.contains("the key service is unreachable"));
        // Refused at lookup, on the admin-event path as on the others: bounded
        // retries of the batch, then the dead-letter queue (not the whole batch failing).
        assert_eq!(built.deferred.len(), 2, "{:?}", built.deferred);
        assert!(built.deferred.iter().all(|reason| reason.contains("(refused)")));
        assert!(built.dead_letters.is_empty());
        assert_eq!(
            delivery_ids(&built.by_board[&format!("board-{EVENT}")]),
            vec!["ok:event"]
        );
        // A failed lookup is remembered for the batch.
        assert_eq!(lookups.signer_calls, 3);
    }

    #[tokio::test]
    async fn a_signer_that_refuses_to_sign_is_permanent() {
        let events = vec![
            keycloak_event(Some("k1"), DOWN_SIGNER_EVENT),
            tenant_activity("t1", DELETED_AN_EVENT),
        ];
        let mut lookups = FakeLookups::new();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Refused);
        lookups.failing_signers.insert(TENANT, SignerOutage::Refused);

        let built = build_batch(&mut lookups, &events).await.unwrap();

        assert!(built.deferred.is_empty(), "{:?}", built.deferred);
        assert!(built.waiting.is_empty());
        assert_eq!(dead_letter_ids(&built), vec![Some("k1"), Some("t1")]);
        for (_, reason) in &built.dead_letters {
            assert!(reason.contains("(refused)"), "{reason}");
        }
    }

    /// The tenants' pending-statement MAC key, in these tests.
    const TEST_MAC_KEY: [u8; 32] = [7; 32];

    /// A pending statement as a worker queues it: its row and its reference event.
    fn queue_pending(
        lookups: &mut FakeLookups,
        pending: &crate::services::electoral_log_pending::PendingStatement,
    ) -> LogEventInput {
        let delivery_id = pending.origin.delivery_id();
        lookups.pending.insert(
            delivery_id.clone(),
            StoredPending {
                statement: serde_json::to_value(pending).unwrap(),
                scope: pending.scope(),
                created_ms: i64::try_from(electoral_log::timestamp()).unwrap() * 1000,
                mac: Some(
                    crate::services::electoral_log_pending::pending_mac(
                        &TEST_MAC_KEY,
                        &delivery_id,
                        pending,
                    )
                    .unwrap(),
                ),
            },
        );
        pending.reference_event(&delivery_id).unwrap()
    }

    #[tokio::test]
    async fn a_trusted_pending_statement_waits_then_is_stored_and_a_forged_one_is_never_signed() {
        use crate::services::electoral_log_pending::{
            PendingOrigin, PendingStatement, PendingStatementBody,
        };
        let problems = |text: &str| PendingStatementBody::EnrollmentProblems {
            problems: text.to_string(),
        };
        let mut lookups = FakeLookups::new();
        let mut durable = Durable::default();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
        // A reference forged on the queue (as Keycloak's role could write): no
        // worker wrote its row.
        let forged_id = PendingOrigin::EnrollmentWindows.delivery_id();
        let forged = PendingStatement::new(TENANT, DOWN_SIGNER_EVENT, problems("forged"))
            .reference_event(&forged_id)
            .unwrap();
        // A worker's row of another election event, referred to from this one.
        let mut foreign = durable.queue_pending(&mut lookups,
            &PendingStatement::new(TENANT, EVENT, problems("foreign")),
        );
        foreign.election_event_id = DOWN_SIGNER_EVENT.into();
        let foreign_id = foreign.delivery_id.clone().unwrap();
        let trusted = durable.queue_pending(&mut lookups,
            &PendingStatement::new(TENANT, DOWN_SIGNER_EVENT, problems("two posts")),
        );
        let trusted_id = trusted.delivery_id.clone().unwrap();

        durable
            .round(&mut lookups, vec![forged, foreign, trusted])
            .await;

        // The foreign one is poison: dead-lettered, never signed. (The trusted
        // one, queued first, waits; the forged reference waits behind it in its
        // scope, and is refused when its turn comes.)
        assert_eq!(durable.dead_letters, vec![foreign_id.clone()]);
        assert_eq!(lookups.sign_calls.load(Ordering::SeqCst), 1, "only the trusted one");
        assert!(durable.board(DOWN_SIGNER_EVENT).is_empty());
        // The trusted one is stored under its own ID; the forged one is refused,
        // never signed.
        lookups.failing_signers.remove(DOWN_SIGNER_EVENT);
        for _ in 0..3 {
            durable.round(&mut lookups, vec![]).await;
        }
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), vec![trusted_id.as_str()]);
        assert_eq!(durable.dead_letters, vec![foreign_id, forged_id]);
        assert!(durable.rows.is_empty(), "no row is left to hold the log");
    }

    #[tokio::test]
    async fn an_admin_signed_entry_is_attested_only_for_the_key_its_tenant_log_announces() {
        use crate::services::electoral_log::admin_signed_certificate_event;
        use electoral_log::messages::newtypes::CertificateAuthEventAction;
        let admin = SigningKey::generate().unwrap();
        let impostor = SigningKey::generate().unwrap();
        let mut lookups = FakeLookups::new();
        let mut durable = Durable::default();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
        let announced = VerificationKey::from_sk(&admin).unwrap().to_b64_string().unwrap();
        lookups
            .announced
            .insert(("admin-id".to_string(), announced));
        let entry = |key: &SigningKey, user: &str| {
            admin_signed_certificate_event(
                key,
                TENANT,
                DOWN_SIGNER_EVENT,
                CertificateAuthEventAction::Delete,
                vec!["CN=one".into()],
                user,
                None,
            )
            .unwrap()
        };
        // The genuine entry first; then a key never announced for that
        // administrator, and another administrator's name on the admin's key.
        let trusted = durable.queue_pending(&mut lookups, &entry(&admin, "admin-id"));
        let impostor_event = durable.queue_pending(&mut lookups, &entry(&impostor, "admin-id"));
        let wrong_user = durable.queue_pending(&mut lookups, &entry(&admin, "someone-else"));
        let ids: Vec<String> = [&trusted, &impostor_event, &wrong_user]
            .iter()
            .map(|event| event.delivery_id.clone().unwrap())
            .collect();

        durable
            .round(&mut lookups, vec![trusted, impostor_event, wrong_user])
            .await;
        assert!(durable.dead_letters.is_empty(), "nothing is dropped");
        assert_eq!(lookups.sign_calls.load(Ordering::SeqCst), 1, "only the genuine one is tried");
        // The board's signer is back: the genuine entry is attested and stored;
        // the others wait, never attested (their keys are not announced).
        lookups.failing_signers.remove(DOWN_SIGNER_EVENT);
        for _ in 0..3 {
            durable.round(&mut lookups, vec![]).await;
        }
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), vec![ids[0].as_str()]);
        assert_eq!(durable.rows.len(), 2);
        // An operator cancels them: their references find no row and go to the
        // dead-letter queue; nothing is ever attested for them.
        for id in &ids[1..] {
            durable.rows.remove(id);
            lookups.pending.remove(id);
        }
        for _ in 0..2 {
            durable.round(&mut lookups, vec![]).await;
        }
        assert_eq!(durable.dead_letters, ids[1..].to_vec());
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), vec![ids[0].as_str()]);
    }

    #[test]
    fn a_tenant_log_entry_announces_an_admin_key_only_when_signed_by_its_system_key() {
        use electoral_log::messages::newtypes::{PublicKeyDerB64, TenantIdString};
        use electoral_log::messages::message::SigningData;
        let system = SigningKey::generate().unwrap();
        let other = SigningKey::generate().unwrap();
        let admin_pk = "QUJD".to_string();
        let announce = |key: &SigningKey, user: &str| {
            let sd = SigningData::new(key.clone(), "", key.clone());
            let message = Message::admin_public_key_message(
                TenantIdString(TENANT.into()),
                Some(user.into()),
                None,
                PublicKeyDerB64(admin_pk.clone()),
                &sd,
                None,
                None,
            )
            .unwrap();
            ElectoralLogMessage::try_from(&message).unwrap()
        };
        let system_pk = VerificationKey::from_sk(&system).unwrap();
        assert!(announces_admin_key(&announce(&system, "admin-id"), TENANT, "admin-id", &admin_pk, &system_pk));
        // Another user, another key, another tenant, another signer: no.
        assert!(!announces_admin_key(&announce(&system, "other"), TENANT, "admin-id", &admin_pk, &system_pk));
        assert!(!announces_admin_key(&announce(&system, "admin-id"), TENANT, "admin-id", "WFla", &system_pk));
        assert!(!announces_admin_key(&announce(&system, "admin-id"), UNKNOWN_TENANT, "admin-id", &admin_pk, &system_pk));
        assert!(!announces_admin_key(&announce(&other, "admin-id"), TENANT, "admin-id", &admin_pk, &system_pk));
    }

    #[tokio::test]
    async fn pending_statements_and_events_of_one_log_never_wait_on_each_other() {
        use crate::services::electoral_log_pending::{PendingStatement, PendingStatementBody};
        let problems = |text: &str| PendingStatementBody::EnrollmentProblems {
            problems: text.to_string(),
        };
        let mut lookups = FakeLookups::new();
        let mut durable = Durable::default();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
        let first = durable.queue_pending(
            &mut lookups,
            &PendingStatement::new(TENANT, DOWN_SIGNER_EVENT, problems("first")),
        );
        let second = durable.queue_pending(
            &mut lookups,
            &PendingStatement::new(TENANT, DOWN_SIGNER_EVENT, problems("second")),
        );
        let ids: Vec<String> = [&first, &second]
            .iter()
            .map(|event| event.delivery_id.clone().unwrap())
            .collect();
        durable
            .round(
                &mut lookups,
                vec![first, keycloak_event(Some("k1"), DOWN_SIGNER_EVENT), second],
            )
            .await;
        // A few rounds of outage: the rows keep their keys, nothing deadlocks.
        for _ in 0..5 {
            durable.round(&mut lookups, vec![]).await;
        }
        lookups.failing_signers.remove(DOWN_SIGNER_EVENT);
        for _ in 0..3 {
            durable.round(&mut lookups, vec![]).await;
        }
        // The statements were queued before the event came: they go first, in order.
        let mut expected = ids.clone();
        expected.push("k1:event".to_string());
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), expected);
        assert!(durable.rows.is_empty());
        assert!(durable.dead_letters.is_empty());
    }

    #[tokio::test]
    async fn a_batch_drains_every_ready_event_of_a_log_in_one_round() {
        // The batch's own rows don't hold its events (B3): once the signer is
        // back, all of a log's waiting events are stored in one round.
        let mut lookups = FakeLookups::new();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
        let mut durable = Durable::default();
        let events: Vec<LogEventInput> = (0..20)
            .map(|i| keycloak_event(Some(&format!("d{i:02}")), DOWN_SIGNER_EVENT))
            .collect();
        durable.round(&mut lookups, events).await;
        assert_eq!(durable.rows.len(), 20);
        lookups.failing_signers.remove(DOWN_SIGNER_EVENT);
        durable.round(&mut lookups, vec![]).await;
        assert_eq!(durable.board(DOWN_SIGNER_EVENT).len(), 20);
        assert!(durable.rows.is_empty());
    }

    #[tokio::test]
    async fn an_event_waits_behind_an_unrelayed_outbox_entry_of_its_log() {
        // As store_batch adds them: an outbox row not yet relayed is ahead of
        // the batch's events of its log.
        let mut waiting = Waiting::default();
        let scope = crate::services::electoral_log_deferral::scope_key(TENANT, EVENT);
        waiting.add(
            scope.clone(),
            WaitKey {
                since_ms: 1_000,
                order: 7,
                delivery_id: "pending:certificate-authority:x".into(),
            },
        );
        waiting.add_due(scope, "pending:certificate-authority:x".into(), 5_000);
        let mut lookups = FakeLookups::new();
        let built = build_batch_behind(
            &mut lookups,
            &[keycloak_event(Some("k1"), EVENT), keycloak_event(Some("other"), KEYLESS_EVENT)],
            &waiting,
        )
        .await
        .unwrap();
        assert_eq!(waiting_ids(&built), vec![("k1", WaitReason::Held)]);
    }

    #[test]
    fn a_scope_is_released_once_stored_unless_something_still_waits_for_the_signer() {
        use crate::services::electoral_log_deferral::released_scopes;
        let stored = keycloak_event(Some("s1"), EVENT);
        let held = keycloak_event(Some("h1"), EVENT);
        let down = keycloak_event(Some("d1"), DOWN_SIGNER_EVENT);
        let down_stored = keycloak_event(Some("s2"), DOWN_SIGNER_EVENT);
        let events = vec![stored.clone(), held.clone(), down.clone(), down_stored.clone()];
        let ids: HashSet<String> = ["s1", "s2"].map(String::from).into();
        let waiting = vec![
            WaitingEvent {
                event: held,
                reason: WaitReason::Held,
                error: String::new(),
            },
            WaitingEvent {
                event: down,
                reason: WaitReason::SignerUnavailable,
                error: String::new(),
            },
        ];
        // EVENT's log: stored, only held entries left: released. DOWN's: an
        // entry still waits for the signer: kept to its backoff.
        assert_eq!(
            released_scopes(&events, &ids, &waiting),
            vec![scope_of(&stored)]
        );
    }

    #[tokio::test]
    async fn a_reference_with_no_row_is_settled_if_stored_and_poison_otherwise() {
        use crate::services::electoral_log_pending::{
            PendingOrigin, PendingStatement, PendingStatementBody,
        };
        let statement = PendingStatement::new(
            TENANT,
            EVENT,
            PendingStatementBody::EnrollmentProblems {
                problems: "x".into(),
            },
        );
        let stored_id = PendingOrigin::EnrollmentWindows.delivery_id();
        let forged_id = PendingOrigin::EnrollmentWindows.delivery_id();
        let mut lookups = FakeLookups::new();
        // An earlier run stored it and removed its row; this is a retry.
        lookups.stored_deliveries.insert(stored_id.clone());
        let events = vec![
            statement.reference_event(&stored_id).unwrap(),
            statement.reference_event(&forged_id).unwrap(),
        ];
        let built = build_batch(&mut lookups, &events).await.unwrap();
        assert_eq!(built.settled, vec![stored_id]);
        assert_eq!(dead_letter_ids(&built), vec![Some(forged_id.as_str())]);
        assert!(built.by_board.is_empty());
    }

    #[tokio::test]
    async fn a_refused_ballots_audit_queued_in_an_outage_is_stored_with_its_own_time() {
        // As Harvest (or Windmill) queues it while the board's signer is
        // unavailable: the record, stored after recovery, says when the ballot
        // was refused, not when it was finally signed.
        use crate::services::electoral_log_pending::{PendingStatement, PendingStatementBody};
        use electoral_log::messages::newtypes::PseudonymHash;
        let mut lookups = FakeLookups::new();
        let mut durable = Durable::default();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
        let mut refusal = PendingStatement::new(
            TENANT,
            DOWN_SIGNER_EVENT,
            PendingStatementBody::CastVoteError {
                election_id: Some("election".into()),
                pseudonym: PseudonymHash(voting_crypto::hash::HashWrapper::new(
                    crate::services::insert_cast_vote::hash_voter_id("voter").unwrap(),
                )),
                error: "Ballot acceptance refused".into(),
                voter_id: "voter".into(),
                area_id: "area".into(),
            },
        );
        let refused_at = electoral_log::timestamp() - 120;
        refusal.occurred_at = refused_at;
        let event = durable.queue_pending(&mut lookups, &refusal);
        let delivery_id = event.delivery_id.clone().unwrap();
        durable.round(&mut lookups, vec![event]).await;
        for _ in 0..5 {
            durable.round(&mut lookups, vec![]).await;
        }
        assert!(durable.board(DOWN_SIGNER_EVENT).is_empty(), "nothing stored during the outage");
        lookups.failing_signers.remove(DOWN_SIGNER_EVENT);
        durable.round(&mut lookups, vec![]).await;
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), vec![delivery_id.as_str()]);
        let stored = Message::record_deserialize(&durable.messages[&delivery_id].message).unwrap();
        assert_eq!(stored.statement.head.timestamp, refused_at);
        assert!(matches!(stored.statement.body, StatementBody::CastVoteError(..)));
        stored
            .verify(&VerificationKey::from_sk(&lookups.signing_key).unwrap())
            .unwrap();
    }

    #[tokio::test]
    async fn a_busy_log_drains_in_order_right_after_its_head_is_stored() {
        // A real clock: one round a second, a new event of the same log each
        // second, during a two-minute outage and after it.
        let mut lookups = FakeLookups::new();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
        let mut durable = Durable {
            step_ms: Some(1_000),
            ..Default::default()
        };
        let mut arrived = Vec::new();
        let mut second = 0;
        let mut head_stored_at = None;
        while second < 600 {
            if second == 120 {
                lookups.failing_signers.remove(DOWN_SIGNER_EVENT);
            }
            let fresh = if second < 400 {
                let id = format!("e{second:03}");
                arrived.push(format!("{id}:event"));
                vec![keycloak_event(Some(&id), DOWN_SIGNER_EVENT)]
            } else {
                vec![]
            };
            durable.round(&mut lookups, fresh).await;
            if head_stored_at.is_none() && !durable.board(DOWN_SIGNER_EVENT).is_empty() {
                head_stored_at = Some(second);
            }
            if let Some(stored_at) = head_stored_at {
                // Once the head is stored, the backlog follows within seconds,
                // not after each held event's own backoff: from then on a new
                // event is stored a few seconds after it arrives (it waits behind
                // the one before it), and the log is never more behind than that.
                if second >= stored_at + 3 && second < 400 {
                    assert!(
                        durable.queue.len() <= 3,
                        "second {second}: {} still wait",
                        durable.queue.len()
                    );
                    assert!(durable.board(DOWN_SIGNER_EVENT).len() + 3 >= arrived.len());
                }
            }
            second += 1;
        }
        let stored_at = head_stored_at.expect("the head is stored after recovery");
        // The head waited at most its capped backoff after the recovery.
        assert!(stored_at <= 120 + 300, "head stored at {stored_at}");
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), arrived, "in arrival order, once each");
        assert!(durable.dead_letters.is_empty());
    }

    #[test]
    fn the_last_retry_keeps_events_that_were_waiting_even_if_it_failed_before_putting_them_back() {
        let fresh = keycloak_event(Some("fresh"), EVENT);
        let waited = LogEventInput {
            deferral: Some(Deferral {
                rounds: 3,
                since_ms: 1_000,
                order: 0,
            }),
            ..keycloak_event(Some("waited"), DOWN_SIGNER_EVENT)
        };
        let marked = keycloak_event(Some("marked"), DOWN_SIGNER_EVENT);
        let waiting: HashSet<String> = ["marked".to_string()].into();
        let (keep, dead) = kept_after_last_retry(vec![&fresh, &waited, &marked], &waiting);
        let ids = |events: Vec<&LogEventInput>| -> Vec<String> {
            events
                .into_iter()
                .map(|event| event.delivery_id.clone().unwrap())
                .collect()
        };
        // Only the row counts: a deferral in a message is the queue writer's word.
        assert_eq!(ids(keep), vec!["marked"]);
        assert_eq!(ids(dead), vec!["fresh", "waited"]);
    }

    #[test]
    fn a_waiting_keycloak_event_says_when_keycloak_saw_it_within_bounds() {
        let now = 1_800_000_000u64;
        let mut event = keycloak_event(Some("k"), EVENT);
        event.event_time_ms = Some(1_799_990_000_000);
        // Stored as it comes: now, as always.
        assert_eq!(statement_time(&event, now), now);
        // Waited: when Keycloak saw it.
        event.deferral = Some(Deferral {
            rounds: 4,
            since_ms: 1_799_990_500_000,
            order: 0,
        });
        assert_eq!(statement_time(&event, now), 1_799_990_000);
        // A time in the future, or long before it first waited: when it first waited.
        event.event_time_ms = Some(1_800_009_000_000);
        assert_eq!(statement_time(&event, now), 1_799_990_500);
        event.event_time_ms = Some(1_700_000_000_000);
        assert_eq!(statement_time(&event, now), 1_799_990_500);
        // No event time: when it first waited.
        event.event_time_ms = None;
        assert_eq!(statement_time(&event, now), 1_799_990_500);
    }

    #[test]
    fn a_fail_closed_entry_still_fails_while_the_signer_is_unavailable() {
        // Secret-attribute and password-change entries are built with
        // `build_keycloak_event_message` before their action; it must fail
        // (and stop the action), never queue.
        let lookups = {
            let mut lookups = FakeLookups::new();
            lookups
                .failing_signers
                .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
            lookups
        };
        let log = ElectoralLog::for_system_with_signer(
            "board",
            lookups.failing_signer(DOWN_SIGNER_EVENT).unwrap(),
        );
        let error = log
            .build_keycloak_event_message(
                DOWN_SIGNER_EVENT.into(),
                "VOTER_SECRET_ATTRIBUTE_REVEALED".into(),
                "{}".into(),
                Some("voter".into()),
                None,
                None,
            )
            .unwrap_err();
        assert!(electoral_log::signer::is_signer_unavailable(&error), "{error:#}");
    }

    #[test]
    fn the_last_retry_never_dead_letters_the_events_that_wait() {
        let events = vec![
            keycloak_event(Some("waits-1"), DOWN_SIGNER_EVENT),
            keycloak_event(Some("other"), EVENT),
            keycloak_event(Some("waits-2"), DOWN_SIGNER_EVENT),
        ];
        let waiting: HashSet<String> = ["waits-1", "waits-2"].map(String::from).into();
        let set_aside: Vec<Option<&str>> = exhausted_events(&events, &waiting)
            .into_iter()
            .map(|event| event.delivery_id.as_deref())
            .collect();
        assert_eq!(set_aside, vec![Some("other")]);
        // All waiting: nothing is dead-lettered, however many retries ran out.
        let all: HashSet<String> = ["waits-1", "other", "waits-2"].map(String::from).into();
        assert!(exhausted_events(&events, &all).is_empty());
        // A failure before anything was put back keeps the old behaviour.
        let failure = StoreFailure::from(anyhow!("database down"));
        assert_eq!(exhausted_events(&events, &failure.waiting).len(), 3);
        assert!(retries_exhausted(10, Some(10)));
    }

    /// The event queue and the boards in memory, as `store_batch` uses them: a
    /// round takes the due events (decoded from the queue's own format) and the
    /// new ones, builds them behind those still waiting, puts the waiting ones
    /// back with their next wait (once per delivery ID), and stores the rest
    /// (a board keeps one record per delivery ID).
    #[derive(Default)]
    struct Durable {
        /// Queue messages: when due (ms) and the message.
        queue: Vec<(i64, serde_json::Value)>,
        /// Delivery IDs of each board's records, in order.
        boards: HashMap<String, Vec<String>>,
        dead_letters: Vec<String>,
        now_ms: i64,
        backoff: DeferralBackoff,
        /// How far a round moves the clock; by default the longest wait, so that
        /// every waiting event is due at the next round.
        step_ms: Option<i64>,
        /// The waiting table: by delivery ID, the scope, the wait key and when due.
        rows: HashMap<String, (String, WaitKey, i64)>,
        /// The stored records, by delivery ID.
        messages: HashMap<String, ElectoralLogMessage>,
    }

    impl Durable {
        fn queued(&self) -> Vec<LogEventInput> {
            self.queue
                .iter()
                .map(|(_, message)| parse_queued_event(message.clone()).unwrap())
                .collect()
        }

        async fn round(&mut self, lookups: &mut FakeLookups, fresh: Vec<LogEventInput>) {
            let now_ms = self.now_ms;
            let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.queue)
                .into_iter()
                .partition(|(at, _)| *at <= now_ms);
            self.queue = later;
            let mut events: Vec<LogEventInput> = due
                .into_iter()
                .map(|(_, message)| parse_queued_event(message).unwrap())
                .collect();
            events.extend(fresh);
            // As `store_batch`: the table's rows, the batch's events bound to theirs.
            let mut waiting = Waiting::default();
            for (delivery_id, (scope, key, due_ms)) in &self.rows {
                waiting.add(scope.clone(), key.clone());
                waiting.add_due(scope.clone(), delivery_id.clone(), *due_ms);
            }
            let (events, waiting) = bind_to_rows(&events, waiting);

            let built = build_batch_behind(lookups, &events, &waiting)
                .await
                .unwrap();

            for (event, _) in &built.dead_letters {
                let id = event.delivery_id.clone().unwrap_or_default();
                self.rows.remove(&id);
                self.dead_letters.push(id);
            }
            let queued: Vec<Option<String>> =
                self.queued().into_iter().map(|event| event.delivery_id).collect();
            for (event, deferral, delay) in plan(&built.waiting, &waiting, &self.backoff, now_ms) {
                let id = event.delivery_id.clone().unwrap_or_default();
                let due = now_ms + delay.as_millis() as i64;
                // The row keeps when and where it first waited; only its due time moves.
                let row = self.rows.entry(id.clone()).or_insert_with(|| {
                    (
                        scope_of(&event),
                        WaitKey {
                            since_ms: deferral.since_ms,
                            order: deferral.order,
                            delivery_id: id.clone(),
                        },
                        due,
                    )
                });
                row.2 = due;
                if !queued.contains(&event.delivery_id) {
                    self.queue.push((due, event_message(&event).unwrap()));
                }
            }
            let stored_ids: HashSet<String> = built
                .delivered
                .values()
                .flatten()
                .chain(&built.settled)
                .cloned()
                .collect();
            for id in &stored_ids {
                self.rows.remove(id);
            }
            // As store_batch: a log whose head was stored drains its backlog now.
            let released = crate::services::electoral_log_deferral::released_scopes(
                &events,
                &stored_ids,
                &built.waiting,
            );
            let next = now_ms
                + self
                    .step_ms
                    .unwrap_or(self.backoff.max.as_millis() as i64);
            for (_, (scope, _, due)) in self.rows.iter_mut() {
                if released.contains(scope) {
                    *due = (*due).min(next);
                }
            }
            for (due, message) in self.queue.iter_mut() {
                let event = parse_queued_event(message.clone()).unwrap();
                if released.contains(&scope_of(&event)) {
                    *due = (*due).min(next);
                }
            }
            for (board, entries) in built.by_board {
                let stored = self.boards.entry(board).or_default();
                for entry in entries {
                    if !stored.contains(&entry.delivery_id) {
                        stored.push(entry.delivery_id.clone());
                        self.messages.insert(entry.delivery_id, entry.message);
                    }
                }
            }
            // The next round: by default once every waiting event is due.
            self.now_ms += self
                .step_ms
                .unwrap_or(self.backoff.max.as_millis() as i64);
        }

        /// A pending statement as a worker queues it: its row (in the table and
        /// in `lookups`) and its reference event, to pass to a round.
        fn queue_pending(
            &mut self,
            lookups: &mut FakeLookups,
            pending: &crate::services::electoral_log_pending::PendingStatement,
        ) -> LogEventInput {
            let event = queue_pending(lookups, pending);
            let id = event.delivery_id.clone().unwrap();
            self.rows.insert(
                id.clone(),
                (
                    pending.scope(),
                    WaitKey {
                        since_ms: self.now_ms,
                        // As the table's clock would: one after the other.
                        order: self.rows.len() as u32,
                        delivery_id: id,
                    },
                    self.now_ms,
                ),
            );
            event
        }

        fn board(&self, election_event_id: &str) -> Vec<&str> {
            self.boards
                .get(&format!("board-{election_event_id}"))
                .map(|ids| ids.iter().map(String::as_str).collect())
                .unwrap_or_default()
        }
    }

    #[tokio::test]
    async fn a_long_outage_dead_letters_nothing_and_stores_each_event_once_in_order_after_it() {
        let mut lookups = FakeLookups::new();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
        let mut durable = Durable::default();
        durable
            .round(
                &mut lookups,
                vec![
                    keycloak_event(Some("e1"), DOWN_SIGNER_EVENT),
                    keycloak_event(Some("e2"), DOWN_SIGNER_EVENT),
                    keycloak_event(Some("ok"), EVENT),
                    internal_event("poison", "not a log message".into()),
                    keycloak_event(Some("e3"), DOWN_SIGNER_EVENT),
                ],
            )
            .await;
        // A permanent error in the same batch is dead-lettered at once.
        assert_eq!(durable.dead_letters, vec!["poison"]);
        assert_eq!(durable.board(EVENT), vec!["ok:event"]);

        // Five times the old ten-retry budget.
        for round in 1..=50 {
            let fresh = if round == 7 {
                // A new event of the same log, while the outage lasts.
                vec![keycloak_event(Some("e4"), DOWN_SIGNER_EVENT)]
            } else {
                vec![]
            };
            durable.round(&mut lookups, fresh).await;
        }
        assert_eq!(durable.dead_letters, vec!["poison"], "nothing more dead-lettered");
        assert!(durable.board(DOWN_SIGNER_EVENT).is_empty());
        let queued = durable.queued();
        assert_eq!(queued.len(), 4, "every event still waits, once");
        let e1 = queued
            .iter()
            .find(|event| event.delivery_id.as_deref() == Some("e1"))
            .unwrap();
        // The wait stopped growing at its longest, and the event is still tried.
        let rounds = e1.deferral.unwrap().rounds;
        assert_eq!(rounds, 51);
        assert_eq!(durable.backoff.delay(rounds), durable.backoff.max);
        // Only the first event of the log asked the key service each round.
        assert!(lookups.sign_calls.load(Ordering::SeqCst) <= 51);

        // The key service is back.
        lookups.failing_signers.remove(DOWN_SIGNER_EVENT);
        for _ in 0..3 {
            durable.round(&mut lookups, vec![]).await;
        }
        assert!(durable.queue.is_empty());
        assert_eq!(
            durable.board(DOWN_SIGNER_EVENT),
            vec!["e1:event", "e2:event", "e3:event", "e4:event"]
        );
        assert_eq!(durable.dead_letters, vec!["poison"]);
    }

    #[tokio::test]
    async fn waiting_events_survive_a_worker_restart() {
        let mut lookups = FakeLookups::new();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::Unavailable);
        let mut durable = Durable::default();
        durable
            .round(
                &mut lookups,
                vec![
                    keycloak_event(Some("r1"), DOWN_SIGNER_EVENT),
                    keycloak_event(Some("r2"), DOWN_SIGNER_EVENT),
                ],
            )
            .await;
        durable.round(&mut lookups, vec![]).await;
        // The worker stops: only the queue's rows remain.
        let rows = serde_json::to_string(&durable.queue).unwrap();
        // The waiting table is durable too.
        let table = durable.rows.clone();
        let now_ms = durable.now_ms;
        drop(durable);
        drop(lookups);

        // A new worker, with nothing in memory, and the key service back.
        let mut lookups = FakeLookups::new();
        let mut durable = Durable {
            queue: serde_json::from_str(&rows).unwrap(),
            rows: table,
            now_ms,
            ..Default::default()
        };
        assert_eq!(durable.queued().len(), 2);
        durable.round(&mut lookups, vec![]).await;
        assert!(durable.queue.is_empty());
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), vec!["r1:event", "r2:event"]);
        assert!(durable.dead_letters.is_empty());
    }

    #[tokio::test]
    async fn a_signing_whose_answer_was_lost_is_repeated_and_stored_once() {
        let mut lookups = FakeLookups::new();
        lookups
            .failing_signers
            .insert(DOWN_SIGNER_EVENT, SignerOutage::LostAnswers);
        lookups.lost_answers.store(1, Ordering::SeqCst);
        let mut durable = Durable::default();
        let event = keycloak_event(Some("u1"), DOWN_SIGNER_EVENT);
        // The service signs, but the answer is lost: the event waits.
        durable.round(&mut lookups, vec![event.clone()]).await;
        assert!(durable.board(DOWN_SIGNER_EVENT).is_empty());
        assert_eq!(durable.queued().len(), 1, "it waits");
        // Signed again (Ed25519 is deterministic: the same signature) and stored.
        durable.round(&mut lookups, vec![]).await;
        assert!(durable.queue.is_empty());
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), vec!["u1:event"]);
        // The batch is delivered again (an uncertain commit, at-least-once
        // delivery): nothing is stored twice.
        durable.round(&mut lookups, vec![event]).await;
        assert_eq!(durable.board(DOWN_SIGNER_EVENT), vec!["u1:event"]);
        assert!(durable.dead_letters.is_empty());
        assert_eq!(lookups.sign_calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn a_tenant_log_that_cannot_be_opened_fails_the_whole_batch() {
        let events = vec![tenant_activity("t1", DELETED_AN_EVENT)];
        let mut lookups = FakeLookups::new();
        lookups.unreachable_tenant = Some(TENANT);

        let error = match build_batch(&mut lookups, &events).await {
            Ok(_) => panic!("a lookup failure must fail the batch"),
            Err(error) => format!("{error:#}"),
        };

        assert!(error.contains("t1"), "{error}");
        assert!(error.contains("connection reset"), "{error}");
    }

    #[test]
    fn tenant_activity_has_a_wire_name_no_keycloak_event_takes() {
        assert_eq!(
            serde_json::to_string(&LogMessageType::TenantActivity).unwrap(),
            r#""tenant_activity""#
        );
        assert_eq!(
            serde_json::from_str::<LogMessageType>(r#""tenant_activity""#).unwrap(),
            LogMessageType::TenantActivity
        );
        assert_eq!(
            serde_json::from_str::<LogMessageType>(r#""TENANT_ACTIVITY""#).unwrap(),
            LogMessageType::KeycloakEvent("TENANT_ACTIVITY".into())
        );
    }

    #[tokio::test]
    async fn the_tenant_realms_events_go_to_the_tenant_log() {
        let mut unknown = tenant_event(
            "d2",
            "LOGIN_ERROR",
            json!({"error": "user_not_found", "client_id": "admin-portal",
                "ip_address": "10.0.0.7", "unknown_account": true}),
        );
        unknown.user_id = Some("null".into());
        unknown.username = Some("null".into());
        let events = vec![
            tenant_event(
                "d1",
                "LOGIN",
                json!({"error": null, "client_id": "admin-portal", "ip_address": "10.0.0.7"}),
            ),
            unknown,
            tenant_event(
                "d3",
                "USER_DISABLED_BY_TEMPORARY_LOCKOUT",
                json!({"error": null}),
            ),
            internal_event("d4", internal_message()),
        ];
        let mut events = events;
        events[3].election_event_id = String::new();

        let mut lookups = FakeLookups::new();
        let built = build_batch(&mut lookups, &events).await.unwrap();

        assert_eq!((lookups.tenant_log_calls, lookups.board_calls), (1, 0));
        assert_eq!(built.by_board.len(), 1);
        let entries = &built.by_board[&format!("tenant-board-{TENANT}")];
        assert_eq!(
            delivery_ids(entries),
            vec!["d1:event", "d2:event", "d3:event"]
        );
        let (signed_in, text) = keycloak_statement(&entries[0]);
        assert_eq!(signed_in.head.event.0, TENANT);
        assert_eq!(signed_in.head.log_type, StatementLogType::INFO);
        assert_eq!(signed_in.head.description, "LOGIN");
        assert!(text.get("ip_address").is_none());
        assert_eq!(entries[0].message.user_id.as_deref(), Some("admin-id"));
        assert_eq!(entries[0].message.username.as_deref(), Some("admin"));
        // The name typed for an unknown account is not recorded.
        let (failed, text) = keycloak_statement(&entries[1]);
        assert_eq!(failed.head.log_type, StatementLogType::ERROR);
        assert_eq!(failed.head.description, "LOGIN_ERROR user_not_found");
        assert_eq!(text["unknown_account"], true);
        assert_eq!(entries[1].message.user_id, None);
        assert_eq!(entries[1].message.username, None);
        let (locked, _) = keycloak_statement(&entries[2]);
        assert_eq!(locked.head.log_type, StatementLogType::ERROR);
        // An internal message belongs to an election event.
        assert_eq!(dead_letter_ids(&built), vec![Some("d4")]);
        assert!(built.login_attempts.is_empty());
    }

    #[test]
    fn the_steps_of_a_sign_in_are_not_sign_ins() {
        let step = json!({"details": {"authenticator_class_name": "MessageOTPAuthenticator",
            "action": "send_code via EMAIL"}});
        let signed_in = json!({"details": {"authenticator_class_name": "MessageOTPAuthenticator",
            "consent": "no_consent_required"}});
        assert_eq!(tenant_event_type("LOGIN", &step), "LOGIN_STEP");
        assert_eq!(tenant_event_type("LOGIN", &signed_in), "LOGIN");
        assert_eq!(tenant_event_type("LOGIN", &json!({"details": {}})), "LOGIN");
        assert_eq!(tenant_event_type("LOGIN", &json!({})), "LOGIN");
        assert_eq!(tenant_event_type("LOGIN_ERROR", &step), "LOGIN_ERROR");
    }

    #[tokio::test]
    async fn keycloak_admin_events_say_who_made_them() {
        let events = vec![
            admin_event("d1", "", "ada"),
            admin_event("d2", "", "service-account-step"),
            admin_event("d3", EVENT, "ada"),
            // Step's own changes to an event's realm are not repeated.
            admin_event("d4", EVENT, "service-account-step"),
        ];
        let built = build_batch(&mut FakeLookups::new(), &events).await.unwrap();
        assert!(built.dead_letters.is_empty());
        let tenant = &built.by_board[&format!("tenant-board-{TENANT}")];
        let event = &built.by_board[&format!("board-{EVENT}")];
        assert_eq!(delivery_ids(tenant), vec!["d1:event", "d2:event"]);
        assert_eq!(delivery_ids(event), vec!["d3:event"]);

        let (message, operator, details) = audit_entry(&tenant[0]);
        assert_eq!(operator.kind, AuditStatementKind::KeycloakAdminEvent);
        assert_eq!(operator.event_type, StatementEventType::USER);
        assert_eq!(operator.log_type, StatementLogType::INFO);
        assert_eq!(
            operator.description,
            "Keycloak: CREATE REALM_ROLE_MAPPING users/42/role-mappings/realm"
        );
        assert_eq!(operator.entry_id, "d1");
        assert_eq!(details["made_by"], "OPERATOR");
        assert_eq!(details["representation"][0]["name"], "tenant-write");
        assert_eq!(message.statement.head.timestamp, 1_778_511_600);
        assert_eq!(message.username.as_deref(), Some("ada"));

        let (_, service, details) = audit_entry(&tenant[1]);
        assert_eq!(service.event_type, StatementEventType::SYSTEM);
        assert_eq!(details["made_by"], "SERVICE_ACCOUNT");

        let (message, in_event, _) = audit_entry(&event[0]);
        assert_eq!(in_event.event_type, StatementEventType::USER);
        assert_eq!(message.statement.head.event.0, EVENT);
    }

    /// Step signs in to Keycloak's admin API as the master realm's user named
    /// by `KEYCLOAK_ADMIN_CLIENT_ID` (through `admin-cli`), so that user's
    /// changes are Step's own; the same name in another realm is not.
    #[test]
    fn an_admin_event_by_steps_master_realm_account_is_steps_own() {
        let event = |username: &str, realm: Value| json!({"actor": {"username": username, "client_id": "admin-cli", "realm": realm}});
        assert_eq!(
            admin_event_author(&event("Admin", json!("master")), "admin"),
            AdminEventAuthor::ServiceAccount
        );
        for (username, realm, client) in [
            (
                "admin",
                json!("tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5"),
                "admin",
            ),
            ("admin", Value::Null, "admin"),
            ("ada", json!("master"), "admin"),
            ("", json!("master"), ""),
        ] {
            assert_eq!(
                admin_event_author(&event(username, realm.clone()), client),
                AdminEventAuthor::Operator,
                "{username} {realm} {client}"
            );
        }
    }

    #[test]
    fn an_admin_event_is_steps_own_only_from_its_service_account() {
        let event = |username: Value| json!({"actor": {"username": username}});
        assert_eq!(
            admin_event_author(&event(json!("Service-Account-Step")), "step"),
            AdminEventAuthor::ServiceAccount
        );
        for (username, client) in [
            (json!("ada"), "step"),
            (json!("service-account-other"), "step"),
            (json!("service-account-"), ""),
            (Value::Null, "step"),
        ] {
            assert_eq!(
                admin_event_author(&event(username.clone()), client),
                AdminEventAuthor::Operator,
                "{username} {client}"
            );
        }
        let mut failed = admin_event("d1", "", "ada");
        failed.admin_event.as_mut().unwrap()["error"] = json!("forbidden");
        failed.event_time_ms = None;
        let signer =
            ElectoralLog::for_system_with_signing_key("board", &SigningKey::generate().unwrap());
        let message =
            admin_event_message(&signer, TENANT, &failed, "d1", AdminEventAuthor::Operator)
                .unwrap();
        let record = LogEntry {
            delivery_id: String::new(),
            message,
        };
        assert_eq!(audit_entry(&record).1.log_type, StatementLogType::ERROR);
        failed.admin_event = None;
        assert!(
            admin_event_message(&signer, TENANT, &failed, "d1", AdminEventAuthor::Operator)
                .is_err()
        );
    }

    #[tokio::test]
    async fn lookups_are_made_once_per_election_event() {
        let events = vec![
            keycloak_event(Some("d1"), EVENT),
            keycloak_event(Some("d2"), EVENT),
            keycloak_event(Some("d3"), DELETED_EVENT),
            keycloak_event(Some("d4"), DELETED_EVENT),
            keycloak_event(Some("d5"), KEYLESS_EVENT),
            keycloak_event(Some("d6"), KEYLESS_EVENT),
        ];

        let mut lookups = FakeLookups::new();
        let built = build_batch(&mut lookups, &events).await.unwrap();

        assert_eq!(lookups.board_calls, 3);
        assert_eq!(lookups.signer_calls, 2);
        // The four events that can't be placed share one tenant log lookup.
        assert_eq!(lookups.tenant_log_calls, 1);
        assert!(built.dead_letters.is_empty());
        assert_eq!(built.by_board[&format!("tenant-board-{TENANT}")].len(), 4);
    }

    #[tokio::test]
    async fn the_sign_in_attempts_of_stored_events_count_under_hashed_delivery_ids() {
        let mut timed = keycloak_event(Some("d1"), EVENT);
        timed.event_time_ms = Some(1_778_511_600_123);
        let events = vec![
            timed,
            internal_event("d2", internal_message()),
            keycloak_event(Some("d3"), KEYLESS_EVENT),
            keycloak_event(Some("d4"), DELETED_EVENT),
        ];

        let mut lookups = FakeLookups::new();
        let built = build_batch(&mut lookups, &events).await.unwrap();

        assert_eq!(built.login_attempts.len(), 1);
        let (delivery_id, attempt) = &built.login_attempts[0];
        assert_eq!(delivery_id, &login_counter_delivery_id("d1"));
        assert!(
            delivery_id.len() == 64
                && delivery_id
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{delivery_id}"
        );
        assert_ne!(delivery_id, &login_counter_delivery_id("d3"));
        assert_eq!(attempt.event_type, "LOGIN");
        assert_eq!(
            attempt.bucket_start,
            crate::services::monitoring::login_counter::bucket_start(
                chrono::DateTime::from_timestamp_millis(1_778_511_600_123).unwrap()
            )
        );
    }

    #[tokio::test]
    async fn a_lookup_failure_fails_the_whole_batch() {
        let events = vec![
            keycloak_event(Some("d1"), EVENT),
            keycloak_event(Some("d2"), KEYLESS_EVENT),
        ];
        let mut lookups = FakeLookups::new();
        lookups.unreachable_event = Some(KEYLESS_EVENT);

        let error = match build_batch(&mut lookups, &events).await {
            Ok(_) => panic!("a lookup failure must fail the batch"),
            Err(error) => format!("{error:#}"),
        };

        assert!(error.contains("d2"), "{error}");
        assert!(error.contains("connection reset"), "{error}");
    }

    #[test]
    fn only_the_last_run_dead_letters_the_batch() {
        assert!(!retries_exhausted(0, Some(10)));
        assert!(!retries_exhausted(9, Some(10)));
        assert!(retries_exhausted(10, Some(10)));
        assert!(!retries_exhausted(1_000, None));
    }

    #[test]
    fn dead_lettered_events_parse_back_unchanged() {
        let mut event = keycloak_event(Some("d1"), EVENT);
        event.body = LogEventBody::Communications("hello".into());
        let message = event_message(&event).unwrap();

        let parsed = parse_queued_event(message).unwrap();

        assert_eq!(
            serde_json::to_value(&parsed).unwrap(),
            serde_json::to_value(&event).unwrap()
        );
    }

    #[tokio::test]
    async fn scanovate_contract_reaches_the_signed_event_board_and_ui_body() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../keycloak-extensions/custom-event-listener/src/test/resources/scanovate-failure-contract.json"
        )).unwrap();
        let input = parse_queued_event(fixture["queue_envelope"].clone()).unwrap();
        assert_eq!(input.tenant_id, "00000000-0000-4000-8000-000000000001");
        assert_eq!(
            input.election_event_id,
            "00000000-0000-4000-8000-000000000002"
        );
        let board = format!("board-{}", input.election_event_id);
        let mut lookups = FakeLookups::new();
        let batch = build_batch(&mut lookups, &[input]).await.unwrap();
        assert!(batch.dead_letters.is_empty());
        assert_eq!(batch.by_board.len(), 1);
        let entries = &batch.by_board[&board];
        assert_eq!(entries.len(), 1);
        let stored = &entries[0].message;
        assert_eq!(
            stored.user_id.as_deref(),
            Some("00000000-0000-4000-8000-000000000003")
        );
        assert_eq!(stored.username.as_deref(), Some("synthetic-voter"));
        let decoded = Message::record_deserialize(&stored.message).unwrap();
        let actual_body = serde_json::to_value(&decoded.statement.body).unwrap();
        let actual_details: Value =
            serde_json::from_str(actual_body["KeycloakUserEvent"][0].as_str().unwrap()).unwrap();
        let expected_details: Value = serde_json::from_str(
            fixture["statement_body"]["KeycloakUserEvent"][0]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(actual_details, expected_details);
        assert_eq!(actual_body["KeycloakUserEvent"][1], "REGISTER_ERROR");
        assert_eq!(decoded.statement.head.log_type, StatementLogType::ERROR);
        assert_eq!(
            actual_details["details"]["scanovate_check_barcode_valid"],
            "missing"
        );
        assert_eq!(
            actual_details["details"]["scanovate_occurred_at"],
            "2026-05-11T15:00:00.123Z"
        );
        assert_eq!(lookups.tenant_log_calls, 0);
    }

    /// Keycloak's publisher produces this envelope; its tests check the same fixture.
    #[test]
    fn keycloak_envelopes_parse_into_events() {
        let envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../../../keycloak-extensions/custom-event-listener/src/test/resources/electoral-log-event-envelope.json"
        ))
        .unwrap();
        let event = parse_queued_event(envelope).unwrap();
        assert_eq!(
            event,
            LogEventInput {
                delivery_id: Some("9d5e2c1b-contract-delivery".into()),
                election_event_id: "6f1c3a6e-2a61-4d6b-9a52-3f0f8a3c2b10".into(),
                message_type: LogMessageType::KeycloakEvent("LOGIN".into()),
                user_id: Some("0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09".into()),
                username: Some("voter@example.com".into()),
                tenant_id: TENANT.into(),
                body: LogEventBody::Plain("null".into()),
                event_time_ms: Some(1_778_511_600_123),
                details: None,
                admin_event: None,
                deferral: None,
            }
        );
    }

    #[test]
    fn malformed_messages_are_rejected() {
        for body in [
            b"not json".to_vec(),
            b"{}".to_vec(),
            b"[[]]".to_vec(),
            b"[[], {}]".to_vec(),
            serde_json::to_vec(&serde_json::json!([[], {"input": {"tenant_id": TENANT}}])).unwrap(),
            serde_json::to_vec(&serde_json::json!([[], {"input": keycloak_event(None, EVENT)}]))
                .unwrap(),
        ] {
            assert!(
                parse_event_body(&body, None).is_err(),
                "{}",
                String::from_utf8_lossy(&body)
            );
        }

        let mut other_task = event_message(&keycloak_event(Some("d1"), EVENT)).unwrap();
        other_task["headers"]["task"] = "process_electoral_log_events_batch".into();
        for message in [
            serde_json::json!({"invalid": true}),
            serde_json::Value::Null,
            other_task,
        ] {
            assert!(parse_queued_event(message.clone()).is_err(), "{message}");
        }
    }
}

#[cfg(test)]
mod pgmq_tests {
    use super::*;
    use crate::services::electoral_log_dead_letter::{ERROR_HEADER, STAGE_HEADER};
    use celery::broker::{Broker, BrokerBuilder};
    use pgmq_broker::setup::{setup, Installation};
    use pgmq_broker::PgmqBrokerBuilder;
    use std::sync::Arc;

    /// The environment of the disposable test database, shared with pgmq-broker's tests.
    const TEST_ENVIRONMENT: &str = "pgmq-test";

    fn test_pool() -> Arc<deadpool_postgres::Pool> {
        let config = deadpool_postgres::Config {
            url: Some(std::env::var("PGMQ_TEST_DATABASE_URL").expect("PGMQ_TEST_DATABASE_URL")),
            ..Default::default()
        };
        Arc::new(
            config
                .create_pool(
                    Some(deadpool_postgres::Runtime::Tokio1),
                    tokio_postgres::NoTls,
                )
                .unwrap(),
        )
    }

    /// Queues of their own for one test, created as the database setup creates them.
    async fn test_queues(pool: &deadpool_postgres::Pool, prefix: &str) -> DispatchQueues {
        let id = &uuid::Uuid::new_v4().simple().to_string()[..12];
        let queues = DispatchQueues {
            events: format!("{prefix}_events_{id}"),
            batches: format!("{prefix}_batches_{id}"),
            dead_letters: format!("{prefix}_dead_letters_{id}"),
        };
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        setup(
            &tx,
            &Installation {
                environment: TEST_ENVIRONMENT,
                queues: &[&queues.events, &queues.batches, &queues.dead_letters],
                roles: None,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        queues
    }

    fn event(election_event_id: &str) -> LogEventInput {
        LogEventInput {
            delivery_id: None,
            election_event_id: election_event_id.into(),
            message_type: LogMessageType::KeycloakEvent("LOGIN_ERROR".into()),
            user_id: None,
            username: None,
            tenant_id: "test-tenant".into(),
            body: LogEventBody::Plain("test".into()),
            event_time_ms: None,
            details: None,
            admin_event: None,
            deferral: None,
        }
    }

    #[tokio::test]
    #[ignore = "requires PGMQ_TEST_DATABASE_URL pointing to a disposable database"]
    async fn waiting_events_are_put_back_once_due_later_and_read_back() {
        use crate::services::electoral_log_deferral::{direct_post_gate, stop_waiting};
        let pool = test_pool();
        let queues = test_queues(&pool, "deferral").await;
        let client = pool.get().await.unwrap();
        // Delivery IDs and scope of their own: the waiting table is shared.
        let unique = uuid::Uuid::new_v4().to_string();
        let w1 = format!("w1-{unique}");
        let waiting = WaitingEvent {
            event: LogEventInput {
                delivery_id: Some(w1.clone()),
                ..event(&unique)
            },
            reason: WaitReason::SignerUnavailable,
            error: "the key service is unreachable: error sending request".into(),
        };
        let scope = scope_of(&waiting.event);
        let backoff = DeferralBackoff::default();
        let now_ms = chrono::Utc::now().timestamp_millis();
        let none = Waiting::default();

        let first = put_back(&**client, &queues.events, &[waiting.clone()], &none, &backoff, now_ms)
            .await
            .unwrap();
        // The batch is retried after an uncertain commit: still one copy.
        let again = put_back(&**client, &queues.events, &[waiting.clone()], &none, &backoff, now_ms)
            .await
            .unwrap();
        assert_eq!((first.put_back, again.put_back, again.already_waiting), (1, 0, 1));
        assert_eq!(queue_length(&client, &queues.events).await, 1);
        // Not due yet: the dispatcher reads nothing.
        let visible = client
            .query("SELECT msg_id FROM pgmq.read($1, 0, 10)", &[&queues.events])
            .await
            .unwrap();
        assert!(visible.is_empty());
        // It is read back from the waiting table, ahead of the later events of its log.
        let read = waiting_events(&**client, &[scope.clone()]).await.unwrap();
        assert_eq!(read.len(), 1);
        assert!(read.due_ms(&scope).unwrap() > now_ms);
        let newer = LogEventInput {
            delivery_id: Some(format!("w2-{unique}")),
            ..waiting.event.clone()
        };
        assert_eq!(read.ahead_of(&newer).unwrap().delivery_id, w1);
        // Its message is the event as producers send it, with how long it waited.
        let row = client
            .query_one(&format!("SELECT message FROM pgmq.q_{}", queues.events), &[])
            .await
            .unwrap();
        let parsed = parse_queued_event(row.get(0)).unwrap();
        assert_eq!(parsed.delivery_id.as_deref(), Some(w1.as_str()));
        assert_eq!(parsed.deferral.unwrap().rounds, 1);

        // A direct post to the scope is refused while it waits; once stored, allowed.
        let mut direct = pool.get().await.unwrap();
        let tx = direct.transaction().await.unwrap();
        let refused = direct_post_gate(&*tx, &scope).await.unwrap_err();
        assert!(
            crate::services::electoral_log_deferral::is_scope_waiting(&refused),
            "{refused:#}"
        );
        tx.rollback().await.unwrap();
        stop_waiting(&**client, &[w1.clone()]).await.unwrap();
        let tx = direct.transaction().await.unwrap();
        direct_post_gate(&*tx, &scope).await.unwrap();
        tx.rollback().await.unwrap();
        drop_queues(&client, &queues).await;
    }

    #[tokio::test]
    #[ignore = "requires PGMQ_TEST_DATABASE_URL pointing to a disposable database"]
    async fn a_direct_post_and_an_entry_starting_to_wait_never_interleave() {
        use crate::services::electoral_log_deferral::direct_post_gate;
        let pool = test_pool();
        let queues = test_queues(&pool, "gate").await;
        let unique = uuid::Uuid::new_v4().to_string();
        let waiting = WaitingEvent {
            event: LogEventInput {
                delivery_id: Some(format!("g1-{unique}")),
                ..event(&unique)
            },
            reason: WaitReason::SignerUnavailable,
            error: "unavailable".into(),
        };
        let scope = scope_of(&waiting.event);
        // Connection A: a direct post passed the gate and is appending (holds the lock).
        let mut a = pool.get().await.unwrap();
        let a_tx = a.transaction().await.unwrap();
        direct_post_gate(&*a_tx, &scope).await.unwrap();
        // Connection B: an entry of the scope starts waiting meanwhile: it waits for A.
        let b_pool = pool.clone();
        let b_queue = queues.events.clone();
        let b_waiting = waiting.clone();
        let b = tokio::spawn(async move {
            let mut b = b_pool.get().await.unwrap();
            let b_tx = b.transaction().await.unwrap();
            put_back(
                &*b_tx,
                &b_queue,
                &[b_waiting],
                &Waiting::default(),
                &DeferralBackoff::default(),
                chrono::Utc::now().timestamp_millis(),
            )
            .await
            .unwrap();
            b_tx.commit().await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(!b.is_finished(), "the entry waits for the direct post's lock");
        // A's append is done: it ends, and only then does B's entry start waiting.
        a_tx.rollback().await.unwrap();
        b.await.unwrap();
        // A later direct post now fails closed.
        let mut c = pool.get().await.unwrap();
        let c_tx = c.transaction().await.unwrap();
        assert!(direct_post_gate(&*c_tx, &scope).await.is_err());
        c_tx.rollback().await.unwrap();
        // Other scopes are not affected.
        let other = pool.get().await.unwrap();
        let mut other = other;
        let other_tx = other.transaction().await.unwrap();
        direct_post_gate(&*other_tx, &format!("{unique}/other")).await.unwrap();
        other_tx.rollback().await.unwrap();
        let client = pool.get().await.unwrap();
        crate::services::electoral_log_deferral::stop_waiting(
            &**client,
            &[format!("g1-{unique}")],
        )
        .await
        .unwrap();
        drop_queues(&client, &queues).await;
    }

    #[tokio::test]
    #[ignore = "requires PGMQ_TEST_DATABASE_URL pointing to a disposable database whose user may create roles"]
    async fn each_role_gets_exactly_its_rights_on_the_waiting_table() {
        use pgmq_broker::setup::Roles;
        use tokio_postgres::error::SqlState;
        let pool = test_pool();
        let id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        let worker = format!("t_worker_{id}");
        let producer = format!("t_producer_{id}");
        let reader = format!("t_reader_{id}");
        let queue = format!("roles_events_{id}");
        let mut client = pool.get().await.unwrap();
        for role in [&worker, &producer, &reader] {
            client
                .batch_execute(&format!("CREATE ROLE {role} NOLOGIN"))
                .await
                .unwrap();
        }
        let tx = client.transaction().await.unwrap();
        setup(
            &tx,
            &Installation {
                environment: TEST_ENVIRONMENT,
                queues: &[&queue],
                roles: Some(&Roles {
                    worker: worker.clone(),
                    producer: producer.clone(),
                    reader: reader.clone(),
                }),
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let scope = format!("{id}/scope");

        // The producer (Harvest) passes the gate: it may read the scope column.
        let tx = client.transaction().await.unwrap();
        tx.batch_execute(&format!("SET LOCAL ROLE {producer}"))
            .await
            .unwrap();
        crate::services::electoral_log_deferral::direct_post_gate(&*tx, &scope)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
        // But it may neither write a row nor read a statement.
        for sql in [
            "INSERT INTO step_queue.electoral_log_waiting (delivery_id, scope, since_ms, ord, due_at) \
             VALUES ('x', 'y', 0, 0, now())",
            "SELECT statement FROM step_queue.electoral_log_waiting",
            "DELETE FROM step_queue.electoral_log_waiting",
        ] {
            let tx = client.transaction().await.unwrap();
            tx.batch_execute(&format!("SET LOCAL ROLE {producer}"))
                .await
                .unwrap();
            let denied = tx.execute(sql, &[]).await.unwrap_err();
            assert_eq!(denied.code(), Some(&SqlState::INSUFFICIENT_PRIVILEGE), "{sql}");
            tx.rollback().await.unwrap();
        }
        // The worker writes rows; the reader reads, never writes.
        let tx = client.transaction().await.unwrap();
        tx.batch_execute(&format!("SET LOCAL ROLE {worker}"))
            .await
            .unwrap();
        tx.execute(
            "INSERT INTO step_queue.electoral_log_waiting (delivery_id, scope, since_ms, ord, due_at) \
             VALUES ($1, $2, 0, 0, now())",
            &[&format!("roles-{id}"), &scope],
        )
        .await
        .unwrap();
        tx.rollback().await.unwrap();
        let tx = client.transaction().await.unwrap();
        tx.batch_execute(&format!("SET LOCAL ROLE {reader}"))
            .await
            .unwrap();
        tx.execute("SELECT statement FROM step_queue.electoral_log_waiting", &[])
            .await
            .unwrap();
        let denied = tx
            .execute("DELETE FROM step_queue.electoral_log_waiting", &[])
            .await
            .unwrap_err();
        assert_eq!(denied.code(), Some(&SqlState::INSUFFICIENT_PRIVILEGE));
        tx.rollback().await.unwrap();

        client
            .query_one("SELECT pgmq.drop_queue($1)", &[&queue])
            .await
            .unwrap();
        for role in [&worker, &producer, &reader] {
            client
                .batch_execute(&format!("DROP OWNED BY {role}; DROP ROLE {role}"))
                .await
                .unwrap();
        }
    }

    /// The plan of `sql`, with sequential scans off.
    async fn plan_of(client: &tokio_postgres::Client, sql: String) -> String {
        client.batch_execute("SET enable_seqscan = off").await.unwrap();
        let plan: Vec<String> = client
            .query(&format!("EXPLAIN {sql}"), &[])
            .await
            .unwrap()
            .iter()
            .map(|row| row.get(0))
            .collect();
        client.batch_execute("RESET enable_seqscan").await.unwrap();
        plan.join("\n")
    }

    #[tokio::test]
    #[ignore = "requires PGMQ_TEST_DATABASE_URL pointing to a disposable database"]
    async fn the_waiting_lookups_use_their_indexes() {
        let pool = test_pool();
        let queues = test_queues(&pool, "indexes").await;
        let client = pool.get().await.unwrap();
        let indexes: Vec<String> = client
            .query(
                "SELECT indexname FROM pg_indexes WHERE schemaname IN ('pgmq', 'step_queue')",
                &[],
            )
            .await
            .unwrap()
            .iter()
            .map(|row| row.get(0))
            .collect();
        assert!(indexes.contains(&format!("q_{}_deferred_delivery", queues.events)), "{indexes:?}");
        assert!(indexes.contains(&"electoral_log_waiting_scope".to_string()), "{indexes:?}");
        // The plans use them (sequential scans off, so a tiny table shows the choice).
        let delivery = plan_of(&client, format!(
            "SELECT 1 FROM pgmq.q_{} WHERE headers->>'x-electoral-log-deferred-scope' IS NOT NULL \
             AND headers->>'x-electoral-log-deferred-delivery' = 'x'",
            queues.events
        ))
        .await;
        assert!(delivery.contains("_deferred_delivery"), "{delivery}");
        let scope = plan_of(
            &client,
            "SELECT 1 FROM step_queue.electoral_log_waiting WHERE scope = ANY(ARRAY['x'])".to_string(),
        )
        .await;
        assert!(scope.contains("electoral_log_waiting_scope"), "{scope}");
        drop_queues(&client, &queues).await;
    }

    fn limits(max_events: usize, max_bytes: usize) -> BatchLimits {
        BatchLimits {
            max_events,
            max_bytes,
        }
    }

    /// Enqueue one raw event per ID, as producers do.
    async fn enqueue_events(
        pool: &Arc<deadpool_postgres::Pool>,
        queues: &DispatchQueues,
        ids: &[&str],
    ) {
        let broker =
            Box::new(PgmqBrokerBuilder::from_pool(pool.clone()).environment(TEST_ENVIRONMENT))
                .declare_queue(&queues.events)
                .build(5)
                .await
                .unwrap();
        for id in ids {
            broker
                .send(
                    &celery::protocol::Message::try_from(enqueue_electoral_log_event::new(event(
                        id,
                    )))
                    .unwrap(),
                    &queues.events,
                )
                .await
                .unwrap();
        }
    }

    async fn queue_length(client: &tokio_postgres::Client, queue: &str) -> i64 {
        client
            .query_one("SELECT queue_length FROM pgmq.metrics($1)", &[&queue])
            .await
            .unwrap()
            .get(0)
    }

    /// The number of events of each batch task waiting in the batch queue, in order.
    async fn batch_sizes(client: &tokio_postgres::Client, target: &str) -> Vec<usize> {
        client
            .query(
                &format!("SELECT message FROM pgmq.q_{target} ORDER BY msg_id"),
                &[],
            )
            .await
            .unwrap()
            .iter()
            .map(|row| {
                let message = pgmq_broker::decode(row.get(0)).unwrap();
                assert_eq!(
                    message.headers.task,
                    process_electoral_log_events_batch::NAME
                );
                let body: serde_json::Value = serde_json::from_slice(&message.raw_body).unwrap();
                body[1]["events"].as_array().unwrap().len()
            })
            .collect()
    }

    async fn drop_queues(client: &tokio_postgres::Client, queues: &DispatchQueues) {
        for queue in [&queues.events, &queues.batches, &queues.dead_letters] {
            client
                .query_one("SELECT pgmq.drop_queue($1)", &[queue])
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "requires PGMQ_TEST_DATABASE_URL pointing to a disposable database"]
    async fn batch_handoff_is_atomic_and_quarantines_invalid_events() {
        let pool = test_pool();
        let queues = test_queues(&pool, "handoff").await;
        enqueue_events(&pool, &queues, &["event-a", "event-b"]).await;
        let client = pool.get().await.unwrap();
        let invalid = serde_json::json!({"invalid": true});
        client
            .query_one("SELECT pgmq.send($1, $2)", &[&queues.events, &invalid])
            .await
            .unwrap();

        client
            .query_one("SELECT pgmq.drop_queue($1)", &[&queues.batches])
            .await
            .unwrap();
        assert!(dispatch_electoral_log_batches(
            &pool,
            &queues,
            limits(10, DEFAULT_BATCH_MAX_BYTES)
        )
        .await
        .is_err());
        assert_eq!(
            queue_length(&client, &queues.events).await,
            3,
            "failed promotion must retain every source event"
        );
        assert_eq!(
            queue_length(&client, &queues.dead_letters).await,
            0,
            "failed promotion must not dead-letter"
        );

        client
            .query_one("SELECT pgmq.create($1)", &[&queues.batches])
            .await
            .unwrap();
        dispatch_electoral_log_batches(&pool, &queues, limits(10, DEFAULT_BATCH_MAX_BYTES))
            .await
            .unwrap();
        assert_eq!(queue_length(&client, &queues.events).await, 0);
        assert_eq!(batch_sizes(&client, &queues.batches).await, vec![2]);
        let dead_letter = client
            .query_one(
                &format!(
                    "SELECT message, headers FROM pgmq.q_{}",
                    queues.dead_letters
                ),
                &[],
            )
            .await
            .unwrap();
        let redacted = dead_letter.get::<_, serde_json::Value>(0);
        assert_eq!(redacted["unparseable_event"], true);
        assert!(redacted.get("original_message_id").is_some());
        let headers: serde_json::Value = dead_letter.get(1);
        assert_eq!(headers[STAGE_HEADER], "dispatcher");
        assert!(headers[ERROR_HEADER]
            .as_str()
            .unwrap()
            .contains("Celery envelope"));
        drop_queues(&client, &queues).await;
    }

    #[tokio::test]
    #[ignore = "requires PGMQ_TEST_DATABASE_URL pointing to a disposable database"]
    async fn batches_close_at_their_limits_and_leave_the_rest_queued() {
        let pool = test_pool();
        let queues = test_queues(&pool, "limits").await;
        enqueue_events(&pool, &queues, &["e1", "e2", "e3", "e4", "e5"]).await;
        let client = pool.get().await.unwrap();

        dispatch_electoral_log_batches(&pool, &queues, limits(2, DEFAULT_BATCH_MAX_BYTES))
            .await
            .unwrap();
        assert_eq!(batch_sizes(&client, &queues.batches).await, vec![2, 2, 1]);

        // A message larger than the byte limit still forms a batch on its own, and the
        // messages read beyond the limit wait for the next batch.
        enqueue_events(&pool, &queues, &["e6", "e7"]).await;
        dispatch_electoral_log_batches(&pool, &queues, limits(10, 1))
            .await
            .unwrap();
        assert_eq!(
            batch_sizes(&client, &queues.batches).await,
            vec![2, 2, 1, 1, 1]
        );
        drop_queues(&client, &queues).await;
    }
}
