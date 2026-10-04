// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Electoral log entries of signing steps. A step is staged in the
//! transaction that makes it, as two outbox rows under one step id, and the
//! outbox worker posts them in order after the commit.
//!
//! A pass over an event:
//! 1. Takes a snapshot: under the event's signing lock (the one every
//!    signing step takes, see `lock_signing_event`), waiting at most
//!    [`SNAPSHOT_LOCK_TIMEOUT`] for the steps being written, it reads the
//!    last unposted id and commits. Every entry up to that id is committed;
//!    any later step queues a higher one. An event whose first unposted
//!    entry failed recently is left alone until its backoff ends.
//! 2. Loads the signing keys the entries need, in transactions of its own.
//! 3. Posts the entries up to the snapshot, in id order (the order the steps
//!    committed), holding only the event's worker lock, so signing steps
//!    never wait for the board. Each entry is a board delivery with a stable
//!    id, `sha256("signing:{step_id}:{event_type}")`, whose receipt the board
//!    keeps in the same transaction: a retry after a lost answer or a lost
//!    commit writes nothing twice. The first failure stops the event, so
//!    nothing is posted out of order.

use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::signing::{
    fetch_unposted_signing_log_outbox_rows, insert_signing_log_outbox,
    list_signing_log_outbox_events, lock_signing_event, mark_signing_log_outbox_failed,
    mark_signing_log_outbox_posted, try_lock_signing_log_worker, unposted_signing_log_outbox_span,
    unposted_signing_log_outbox_users, NewSigningLogOutboxEntry, SigningLogOutboxRow,
    SigningLogOutboxUser,
};
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_board_client;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{
    EventIdString, SigningLogEntry, SigningStatementKind, Timestamp,
};
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use electoral_log::{BoardClient, ElectoralLogMessage};
use immudb_rs::TxMode;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// The person a step is logged for. For an expiry it is the requester.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub user_id: String,
    pub username: String,
}

/// What the SYSTEM entry of a step reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemOutcome {
    Info,
    Error,
}

impl SystemOutcome {
    fn log_type(self) -> StatementLogType {
        match self {
            SystemOutcome::Info => StatementLogType::INFO,
            SystemOutcome::Error => StatementLogType::ERROR,
        }
    }
}

/// Where a step happened. Post = `election_id`, country = `area_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogScope {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct LogStep {
    pub kind: SigningStatementKind,
    pub user: Actor,
    pub system: SystemOutcome,
    pub scope: LogScope,
    /// The entries' short English description, built by the server.
    pub description: String,
    /// The entries' `details_json`.
    pub details: Value,
}

/// Queues the USER and the SYSTEM entry of `step` in the caller's
/// transaction, under one new step id, which it returns. The outbox row's
/// body holds the description and the details. It takes the event's signing
/// lock, which the caller should already hold (see [`lock_signing_event`]),
/// so that a step staged without it still queues in commit order.
#[instrument(skip(hasura_transaction, step), fields(kind = %step.kind), err)]
pub async fn stage(hasura_transaction: &Transaction<'_>, step: &LogStep) -> Result<Uuid> {
    lock_signing_event(
        hasura_transaction,
        step.scope.tenant_id,
        step.scope.election_event_id,
    )
    .await?;
    let step_id = Uuid::new_v4();
    // One time for both entries of the step.
    let occurred_at: DateTime<Utc> = hasura_transaction
        .query_one("SELECT clock_timestamp()", &[])
        .await
        .context("Error reading the time of the signing step")?
        .try_get(0)?;
    let body = json!({
        "description": step.description,
        "details": step.details,
    });
    let entry = |event_type, log_type, user: Option<&Actor>| NewSigningLogOutboxEntry {
        tenant_id: step.scope.tenant_id,
        election_event_id: step.scope.election_event_id,
        step_id,
        statement_kind: step.kind,
        event_type,
        log_type,
        user_id: user.map(|user| user.user_id.clone()),
        username: user.map(|user| user.username.clone()),
        election_id: step.scope.election_id,
        area_id: step.scope.area_id,
        body: body.clone(),
        occurred_at,
    };
    insert_signing_log_outbox(
        hasura_transaction,
        &entry(
            StatementEventType::USER,
            StatementLogType::INFO,
            Some(&step.user),
        ),
    )
    .await?;
    insert_signing_log_outbox(
        hasura_transaction,
        &entry(StatementEventType::SYSTEM, step.system.log_type(), None),
    )
    .await?;
    Ok(step_id)
}

// Posting

/// How many entries one pass posts per event.
pub const OUTBOX_BATCH: i64 = 200;

/// Deliveries per board transaction, as the electoral log dispatcher writes
/// them.
pub const DELIVERIES_PER_TRANSACTION: usize = 16;

/// How long a snapshot waits for the signing steps being written before it
/// leaves the event to a later pass.
pub const SNAPSHOT_LOCK_TIMEOUT: Duration = Duration::from_secs(2);

/// The longest one event's pass may take; the other events go on after it.
pub const EVENT_TIME_LIMIT: Duration = Duration::from_secs(60);

/// Failed attempts after which an entry is reported as an error, once.
pub const ALERT_ATTEMPTS: i32 = 5;

/// The longest wait between two attempts at a failing entry, in seconds.
const MAX_BACKOFF_SECONDS: i64 = 600;

/// The longest `last_error` kept, in characters.
const MAX_ERROR_CHARS: usize = 1000;

/// A copy of `value` whose objects list their keys in order, so its text
/// is the same whatever order it was read in.
fn sorted(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            Value::Object(
                keys.into_iter()
                    .map(|key| (key.clone(), sorted(&object[key])))
                    .collect::<Map<String, Value>>(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

fn description(row: &SigningLogOutboxRow) -> Result<&str> {
    row.body
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("signing log row {} has no description", row.id))
}

fn details(row: &SigningLogOutboxRow) -> Result<Value> {
    row.body
        .get("details")
        .map(sorted)
        .ok_or_else(|| anyhow!("signing log row {} has no details", row.id))
}

/// Only USER entries name their person.
fn person(row: &SigningLogOutboxRow) -> (Option<&str>, Option<&str>) {
    match row.event_type {
        StatementEventType::USER => (row.user_id.as_deref(), row.username.as_deref()),
        StatementEventType::SYSTEM => (None, None),
    }
}

/// The entry an outbox row posts. Kind, event type and log type are the
/// row's; the description and details (keys in order) come from its body.
pub fn outbox_entry(row: &SigningLogOutboxRow) -> Result<SigningLogEntry> {
    Ok(SigningLogEntry {
        kind: row.statement_kind,
        event_type: row.event_type.clone(),
        log_type: row.log_type.clone(),
        description: description(row)?.to_string(),
        details_json: serde_json::to_string(&details(row)?)?,
        step_id: row.step_id.to_string(),
    })
}

/// The signed message of an outbox row. Its statement timestamp is when the
/// step happened. Only USER entries name their person.
pub fn outbox_message(row: &SigningLogOutboxRow, sd: &SigningData) -> Result<Message> {
    let timestamp = Timestamp::try_from(row.occurred_at.timestamp())
        .with_context(|| format!("signing log row {} happened before 1970", row.id))?;
    let (user_id, username) = person(row);
    Message::signing_message(
        EventIdString(row.election_event_id.to_string()),
        outbox_entry(row)?,
        timestamp,
        sd,
        user_id.map(str::to_string),
        username.map(str::to_string),
        row.election_id.map(|id| id.to_string()),
        row.area_id.map(|id| id.to_string()),
    )
}

/// The board row of an outbox row.
pub fn outbox_board_message(
    row: &SigningLogOutboxRow,
    sd: &SigningData,
) -> Result<ElectoralLogMessage> {
    ElectoralLogMessage::try_from(&outbox_message(row, sd)?)
}

/// The board delivery id of a step's entry: the same for every attempt.
pub fn delivery_id(step_id: Uuid, event_type: &StatementEventType) -> String {
    hex::encode(Sha256::digest(
        format!("signing:{step_id}:{event_type}").as_bytes(),
    ))
}

/// What an entry says, apart from its signature: the board refuses a
/// delivery id reused for anything else.
#[derive(Serialize)]
struct DeliveryPayload<'a> {
    kind: String,
    event_type: String,
    log_type: String,
    step_id: Uuid,
    election_event_id: Uuid,
    election_id: Option<Uuid>,
    area_id: Option<Uuid>,
    user_id: Option<&'a str>,
    username: Option<&'a str>,
    occurred_at: String,
    description: &'a str,
    details: Value,
}

/// The SHA-256 of what the row's entry says, not of its signed bytes, which
/// change with every signature.
pub fn payload_hash(row: &SigningLogOutboxRow) -> Result<String> {
    let (user_id, username) = person(row);
    let payload = DeliveryPayload {
        kind: row.statement_kind.to_string(),
        event_type: row.event_type.to_string(),
        log_type: row.log_type.to_string(),
        step_id: row.step_id,
        election_event_id: row.election_event_id,
        election_id: row.election_id,
        area_id: row.area_id,
        user_id,
        username,
        occurred_at: row.occurred_at.to_rfc3339_opts(SecondsFormat::Micros, true),
        description: description(row)?,
        details: details(row)?,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

/// One entry, ready for the board.
#[derive(Debug, Clone)]
pub struct SigningLogDelivery {
    pub row_id: i64,
    pub delivery_id: String,
    pub payload_hash: String,
    pub message: ElectoralLogMessage,
}

pub fn outbox_delivery(row: &SigningLogOutboxRow, sd: &SigningData) -> Result<SigningLogDelivery> {
    Ok(SigningLogDelivery {
        row_id: row.id,
        delivery_id: delivery_id(row.step_id, &row.event_type),
        payload_hash: payload_hash(row)?,
        message: outbox_board_message(row, sd)?,
    })
}

/// When an entry that failed `attempts` times, the last at
/// `last_attempt_at`, may be tried again: 2^attempts seconds later, at most
/// ten minutes.
pub fn retry_after(attempts: i32, last_attempt_at: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = 2_i64
        .saturating_pow(attempts.clamp(0, 30) as u32)
        .min(MAX_BACKOFF_SECONDS);
    last_attempt_at + chrono::Duration::seconds(seconds)
}

/// The keys that sign an event's entries.
pub trait SigningLogKeys: Send + Sync {
    /// Loads the keys the entries of `users` (and the SYSTEM entries) need,
    /// each in a transaction of its own on `client`, and returns the event's
    /// board. Called before the posting transaction opens.
    fn prepare(
        &mut self,
        client: &mut Client,
        tenant_id: Uuid,
        election_event_id: Uuid,
        users: &[SigningLogOutboxUser],
    ) -> impl Future<Output = Result<String>> + Send;

    /// The signing data of `row`'s entry.
    fn signing_data(&self, row: &SigningLogOutboxRow) -> Result<&SigningData>;
}

/// The board the worker posts to.
pub trait SigningLogBoard: Send {
    /// Writes `deliveries` to `board` in one transaction, in order. A
    /// delivery whose receipt the board holds is not written again (`false`);
    /// a delivery id reused for other content fails the transaction.
    fn deliver(
        &mut self,
        board: &str,
        deliveries: &[SigningLogDelivery],
    ) -> impl Future<Output = Result<Vec<bool>>> + Send;
}

/// What one pass over an event did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OutboxPass {
    /// Entries now marked posted, including those the board already had.
    pub posted: usize,
    /// Entries a lost answer or commit had already put on the board.
    pub already_on_board: usize,
    /// The row that failed, which stopped the pass.
    pub failed: Option<i64>,
}

/// An event's snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxSnapshot {
    /// Nothing to post.
    Empty,
    /// A signing step held the event past the lock timeout.
    Busy,
    /// The first entry failed; the event waits until then.
    Backoff(DateTime<Utc>),
    /// Post the entries up to this id.
    Ready(i64),
}

/// What a pass over an event came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassOutcome {
    Posted(OutboxPass),
    Empty,
    /// A signing step or another worker held the event.
    Busy,
    Backoff(DateTime<Utc>),
}

/// The error chain, cut to `MAX_ERROR_CHARS`.
fn error_text(error: &anyhow::Error) -> String {
    format!("{error:#}").chars().take(MAX_ERROR_CHARS).collect()
}

fn is_lock_timeout(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<tokio_postgres::Error>())
        .any(|error| error.code() == Some(&tokio_postgres::error::SqlState::LOCK_NOT_AVAILABLE))
}

/// Takes the event's snapshot in `hasura_transaction`, which the caller
/// commits at once to let signing steps go on. It waits at most
/// `lock_timeout` for the steps being written (then the transaction fails
/// with a lock timeout).
#[instrument(skip(hasura_transaction), err)]
pub async fn outbox_snapshot(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    lock_timeout: Duration,
) -> Result<OutboxSnapshot> {
    hasura_transaction
        .batch_execute(&format!(
            "SET LOCAL lock_timeout = '{}ms'",
            lock_timeout.as_millis()
        ))
        .await
        .context("Error setting the snapshot's lock timeout")?;
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let Some(span) =
        unposted_signing_log_outbox_span(hasura_transaction, tenant_id, election_event_id).await?
    else {
        return Ok(OutboxSnapshot::Empty);
    };
    if let Some(last_attempt_at) = span.last_attempt_at {
        let until = retry_after(span.attempts, last_attempt_at);
        if until > span.now {
            return Ok(OutboxSnapshot::Backoff(until));
        }
    }
    Ok(OutboxSnapshot::Ready(span.last_id))
}

/// [`outbox_snapshot`] in a transaction of its own: `Busy` when a signing
/// step held the event past `lock_timeout`.
#[instrument(skip(client), err)]
pub async fn take_outbox_snapshot(
    client: &mut Client,
    tenant_id: Uuid,
    election_event_id: Uuid,
    lock_timeout: Duration,
) -> Result<OutboxSnapshot> {
    let transaction = client
        .transaction()
        .await
        .context("Error starting the signing log snapshot")?;
    match outbox_snapshot(&transaction, tenant_id, election_event_id, lock_timeout).await {
        Err(error) if is_lock_timeout(&error) => Ok(OutboxSnapshot::Busy),
        Err(error) => Err(error),
        Ok(snapshot) => {
            transaction
                .commit()
                .await
                .context("Error ending the signing log snapshot")?;
            Ok(snapshot)
        }
    }
}

/// How much of a snapshot one posting transaction takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboxBatch {
    /// The snapshot: entries after it wait for a later pass.
    pub up_to_id: i64,
    /// At most this many entries.
    pub rows: i64,
    /// Deliveries per board transaction.
    pub per_transaction: usize,
}

async fn fail(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: i64,
    error: &anyhow::Error,
) -> Result<()> {
    let attempts =
        mark_signing_log_outbox_failed(hasura_transaction, id, &error_text(error)).await?;
    if attempts == ALERT_ATTEMPTS {
        error!(%tenant_id, %election_event_id, id, attempts,
            "A signing log entry keeps failing; the event's log is held up: {error:#}");
    } else {
        warn!(%tenant_id, %election_event_id, id, attempts,
            "Error posting a signing log entry: {error:#}");
    }
    Ok(())
}

/// Posts the event's unposted entries within `batch`, in order, and marks
/// them posted in `hasura_transaction`, which the caller commits. `None`
/// while another worker holds the event. The first entry that doesn't read
/// or sign is marked failed and ends the pass; so is the first entry of a
/// board transaction that fails (it is all or nothing).
#[instrument(skip(hasura_transaction, keys, board), err)]
pub async fn post_outbox_snapshot<K: SigningLogKeys, B: SigningLogBoard>(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    keys: &K,
    board: &mut B,
    board_name: &str,
    batch: OutboxBatch,
) -> Result<Option<OutboxPass>> {
    if !try_lock_signing_log_worker(hasura_transaction, tenant_id, election_event_id).await? {
        return Ok(None);
    }
    let rows = fetch_unposted_signing_log_outbox_rows(
        hasura_transaction,
        tenant_id,
        election_event_id,
        batch.up_to_id,
        batch.rows,
    )
    .await?;
    let mut deliveries = Vec::with_capacity(rows.len());
    let mut unprepared = None;
    for (id, row) in rows {
        let delivery = row.and_then(|row| outbox_delivery(&row, keys.signing_data(&row)?));
        match delivery {
            Ok(delivery) => deliveries.push(delivery),
            Err(error) => {
                unprepared = Some((id, error.context("Unreadable signing log entry")));
                break;
            }
        }
    }
    let mut pass = OutboxPass::default();
    for chunk in deliveries.chunks(batch.per_transaction.max(1)) {
        match board.deliver(board_name, chunk).await {
            Ok(written) => {
                for (delivery, new) in chunk.iter().zip(written) {
                    mark_signing_log_outbox_posted(hasura_transaction, delivery.row_id).await?;
                    pass.posted += 1;
                    if !new {
                        pass.already_on_board += 1;
                    }
                }
            }
            Err(error) => {
                let first = chunk.first().map(|delivery| delivery.row_id);
                if let Some(id) = first {
                    fail(hasura_transaction, tenant_id, election_event_id, id, &error).await?;
                    pass.failed = Some(id);
                }
                return Ok(Some(pass));
            }
        }
    }
    if let Some((id, error)) = unprepared {
        fail(hasura_transaction, tenant_id, election_event_id, id, &error).await?;
        pass.failed = Some(id);
    }
    Ok(Some(pass))
}

/// One pass over an event: the snapshot, the keys, then the entries up to
/// the snapshot in a transaction that holds only the event's worker lock.
#[instrument(skip(client, keys, board), err)]
pub async fn post_event_outbox<K: SigningLogKeys, B: SigningLogBoard>(
    client: &mut Client,
    tenant_id: Uuid,
    election_event_id: Uuid,
    keys: &mut K,
    board: &mut B,
) -> Result<PassOutcome> {
    let up_to_id =
        match take_outbox_snapshot(client, tenant_id, election_event_id, SNAPSHOT_LOCK_TIMEOUT)
            .await?
        {
            OutboxSnapshot::Empty => return Ok(PassOutcome::Empty),
            OutboxSnapshot::Busy => return Ok(PassOutcome::Busy),
            OutboxSnapshot::Backoff(until) => return Ok(PassOutcome::Backoff(until)),
            OutboxSnapshot::Ready(up_to_id) => up_to_id,
        };
    let users = {
        let transaction = client.transaction().await?;
        let users =
            unposted_signing_log_outbox_users(&transaction, tenant_id, election_event_id, up_to_id)
                .await?;
        transaction.commit().await?;
        users
    };
    let board_name = keys
        .prepare(client, tenant_id, election_event_id, &users)
        .await?;
    let transaction = client
        .transaction()
        .await
        .context("Error starting the signing log outbox transaction")?;
    let pass = post_outbox_snapshot(
        &transaction,
        tenant_id,
        election_event_id,
        keys,
        board,
        &board_name,
        OutboxBatch {
            up_to_id,
            rows: OUTBOX_BATCH,
            per_transaction: DELIVERIES_PER_TRANSACTION,
        },
    )
    .await?;
    transaction
        .commit()
        .await
        .context("Error committing the signing log outbox transaction")?;
    Ok(match pass {
        Some(pass) => PassOutcome::Posted(pass),
        None => PassOutcome::Busy,
    })
}

/// What a full pass did for one event: its outcome, or the error that
/// rolled it back (a timeout included).
pub type EventOutcome = (Uuid, Uuid, Result<PassOutcome>);

/// One pass over every event with unposted entries, through one board
/// connection. Each event has its own transactions and at most
/// `event_time_limit`: one that fails or is slow doesn't hold up the others.
#[instrument(skip_all, err)]
pub async fn post_signing_log_outboxes<K, F, B>(
    client: &mut Client,
    board: &mut B,
    mut new_keys: F,
    event_time_limit: Duration,
) -> Result<Vec<EventOutcome>>
where
    K: SigningLogKeys,
    F: FnMut(Uuid, Uuid) -> K,
    B: SigningLogBoard,
{
    let events = {
        let transaction = client
            .transaction()
            .await
            .context("Error starting to list the signing log outboxes")?;
        let events = list_signing_log_outbox_events(&transaction).await?;
        transaction.commit().await?;
        events
    };
    let mut outcomes = Vec::with_capacity(events.len());
    for (tenant_id, election_event_id) in events {
        let mut keys = new_keys(tenant_id, election_event_id);
        let outcome = tokio::time::timeout(
            event_time_limit,
            post_event_outbox(client, tenant_id, election_event_id, &mut keys, board),
        )
        .await
        .unwrap_or_else(|_| {
            Err(anyhow!(
                "The event's pass took longer than {event_time_limit:?}"
            ))
        });
        match &outcome {
            Ok(PassOutcome::Posted(pass)) => {
                info!(%tenant_id, %election_event_id, ?pass, "Signing log posted")
            }
            Ok(other) => debug!(%tenant_id, %election_event_id, ?other, "Signing log not posted"),
            Err(error) => warn!(%tenant_id, %election_event_id,
                "Error posting the signing log: {error:#}"),
        }
        outcomes.push((tenant_id, election_event_id, outcome));
    }
    Ok(outcomes)
}

/// Signs with the keys `ElectoralLog` uses: SYSTEM entries with the
/// event's system key (`ElectoralLog::new`), USER entries with their
/// person's admin key (`ElectoralLog::for_admin_user`, which makes it and
/// posts its public key the first time).
#[derive(Default)]
pub struct ElectoralLogKeys {
    system: Option<ElectoralLog>,
    admins: HashMap<String, ElectoralLog>,
}

impl ElectoralLogKeys {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SigningLogKeys for ElectoralLogKeys {
    async fn prepare(
        &mut self,
        client: &mut Client,
        tenant_id: Uuid,
        election_event_id: Uuid,
        users: &[SigningLogOutboxUser],
    ) -> Result<String> {
        let (tenant, event) = (tenant_id.to_string(), election_event_id.to_string());
        let transaction = client.transaction().await?;
        let election_event = get_election_event_by_id(&transaction, &tenant, &event).await?;
        let board = get_election_event_board(election_event.bulletin_board_reference)
            .ok_or_else(|| anyhow!("The election event has no electoral-log board"))?;
        if self.system.is_none() {
            self.system =
                Some(ElectoralLog::new(&transaction, &tenant, Some(&event), &board).await?);
        }
        transaction.commit().await?;
        for user in users {
            if self.admins.contains_key(&user.user_id) {
                continue;
            }
            // Its own transaction: a key made here is kept even if the
            // posting transaction rolls back.
            let transaction = client.transaction().await?;
            let electoral_log = ElectoralLog::for_admin_user(
                &transaction,
                &board,
                &tenant,
                &event,
                &user.user_id,
                user.username.clone(),
                user.election_id.map(|id| vec![id.to_string()]),
                user.area_id.map(|id| id.to_string()),
            )
            .await?;
            transaction.commit().await?;
            self.admins.insert(user.user_id.clone(), electoral_log);
        }
        Ok(board)
    }

    fn signing_data(&self, row: &SigningLogOutboxRow) -> Result<&SigningData> {
        let electoral_log = match (&row.event_type, &row.user_id) {
            (StatementEventType::USER, Some(user_id)) => self.admins.get(user_id),
            (StatementEventType::USER, None) => None,
            (StatementEventType::SYSTEM, _) => self.system.as_ref(),
        };
        electoral_log
            .map(|electoral_log| &electoral_log.sd)
            .ok_or_else(|| anyhow!("No signing key loaded for signing log row {}", row.id))
    }
}

/// The board operations a delivery takes; [`BoardClient`] has them.
pub trait BoardConnection: Send {
    fn open_session(&mut self, board: &str) -> impl Future<Output = Result<()>> + Send;
    fn ensure_receipts(&mut self) -> impl Future<Output = Result<()>> + Send;
    fn begin(&mut self) -> impl Future<Output = Result<String>> + Send;
    fn insert_delivery(
        &mut self,
        transaction_id: &String,
        delivery: &SigningLogDelivery,
    ) -> impl Future<Output = Result<bool>> + Send;
    fn commit(&mut self, transaction_id: &String) -> impl Future<Output = Result<()>> + Send;
    fn close_session(&mut self) -> impl Future<Output = Result<()>> + Send;
}

impl BoardConnection for BoardClient {
    async fn open_session(&mut self, board: &str) -> Result<()> {
        BoardClient::open_session(self, board).await
    }

    async fn ensure_receipts(&mut self) -> Result<()> {
        self.ensure_electoral_log_delivery_receipts().await
    }

    async fn begin(&mut self) -> Result<String> {
        self.new_tx(TxMode::ReadWrite).await
    }

    async fn insert_delivery(
        &mut self,
        transaction_id: &String,
        delivery: &SigningLogDelivery,
    ) -> Result<bool> {
        self.insert_electoral_log_delivery(
            transaction_id,
            &delivery.delivery_id,
            &delivery.payload_hash,
            std::slice::from_ref(&delivery.message),
        )
        .await
    }

    async fn commit(&mut self, transaction_id: &String) -> Result<()> {
        BoardClient::commit(self, transaction_id).await.map(|_| ())
    }

    async fn close_session(&mut self) -> Result<()> {
        BoardClient::close_session(self).await
    }
}

/// Opens a board connection.
pub type Connect<C> =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<C>> + Send>> + Send + Sync>;

/// The electoral log boards, through one connection kept across deliveries
/// (and events). A delivery that fails, or is cut short, drops it; the next
/// one connects again.
pub struct BoardSigningLog<C> {
    connect: Connect<C>,
    connection: Option<C>,
}

impl<C> BoardSigningLog<C> {
    pub fn new(connect: Connect<C>) -> Self {
        BoardSigningLog {
            connect,
            connection: None,
        }
    }
}

impl BoardSigningLog<BoardClient> {
    /// The immudb boards.
    pub fn immudb() -> Self {
        Self::new(Box::new(|| Box::pin(get_board_client())))
    }
}

impl<C: BoardConnection> SigningLogBoard for BoardSigningLog<C> {
    async fn deliver(
        &mut self,
        board: &str,
        deliveries: &[SigningLogDelivery],
    ) -> Result<Vec<bool>> {
        // Taken out while in use: a delivery cut short drops it.
        let mut connection = match self.connection.take() {
            Some(connection) => connection,
            None => (self.connect)().await?,
        };
        let written = async {
            connection.open_session(board).await?;
            connection.ensure_receipts().await?;
            let transaction_id = connection.begin().await?;
            let mut written = Vec::with_capacity(deliveries.len());
            for delivery in deliveries {
                written.push(
                    connection
                        .insert_delivery(&transaction_id, delivery)
                        .await?,
                );
            }
            connection.commit(&transaction_id).await?;
            Ok(written)
        }
        .await;
        let closed = connection.close_session().await;
        if let Err(error) = &closed {
            warn!(%board, "Error closing the electoral log session: {error:#}");
        }
        if written.is_ok() && closed.is_ok() {
            self.connection = Some(connection);
        }
        written
    }
}

#[cfg(test)]
#[path = "log_tests.rs"]
mod tests;
