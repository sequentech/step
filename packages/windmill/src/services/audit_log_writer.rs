// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The audit log writer: posts `audit_log_outbox` to the tenants' logs and the
//! election events' logs.
//!
//! Each log's entries are posted in the order they were queued. An entry is
//! signed with its person's admin key and the log's system key, and delivered
//! under its outbox id, which the log stores once: a pass that fails after
//! posting posts nothing twice when it runs again. Entries wait in the outbox
//! while a log or the worker is down.

use crate::postgres::audit_log_outbox::{
    audit_log_outbox_backlog, fetch_unposted_audit_log_outbox, insert_audit_log_outbox,
    list_audit_log_outbox_targets, mark_audit_log_outbox_failed, mark_audit_log_outbox_posted,
    try_lock_audit_log_worker, AuditLogOutboxRow, AuditLogTarget, NewAuditLogEntry,
};
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log_board::{
    ensure_tenant_log, get_board_client, get_board_signing_key,
};
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{AuditLogEntry, AuditStatementKind, EventIdString};
use electoral_log::messages::statement::{StatementBody, StatementEventType, StatementLogType};
use electoral_log::{ElectoralLogMessage, LogEntry};
use sequent_core::services::keycloak::{get_tenant_realm, KeycloakAdminClient};
use sequent_core::util::version::ENV_VAR_APP_VERSION;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::io::Write;
use std::sync::Mutex;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;
use voting_crypto::signing::SigningKey;
use voting_crypto::RecordDeserialize;

/// Entries one pass posts to a log.
pub const OUTBOX_BATCH: i64 = 200;
/// Prefix of an audit entry's delivery id, before its outbox id.
pub const AUDIT_DELIVERY_PREFIX: &str = "audit:";
/// Longest wait before an entry that keeps failing is tried again.
const MAX_BACKOFF_SECONDS: i64 = 600;
/// Attempts after which a failing entry is reported as an error.
const ALERT_ATTEMPTS: i32 = 5;
/// The `target` of the JSON lines monitoring collects.
pub const MONITORING_TARGET: &str = "audit_log";
/// The `target` of the JSON line with the outbox's waiting time.
pub const MONITORING_OUTBOX_TARGET: &str = "audit_log_outbox";

/// The delivery id of an outbox entry on its log.
pub fn delivery_id(entry_id: Uuid) -> String {
    format!("{AUDIT_DELIVERY_PREFIX}{entry_id}")
}

/// When an entry that failed `attempts` times, the last at `last_attempt_at`,
/// may be tried again: 2^attempts seconds later, at most ten minutes.
pub fn retry_after(attempts: i32, last_attempt_at: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = 2_i64
        .saturating_pow(attempts.clamp(0, 30) as u32)
        .min(MAX_BACKOFF_SECONDS);
    last_attempt_at + chrono::Duration::seconds(seconds)
}

/// Names the people whose entries were queued without a username, as the
/// database trigger queues them: the Hasura session carries only the user id.
pub trait Usernames: Send + Sync {
    fn username(
        &self,
        tenant_id: Uuid,
        user_id: &str,
    ) -> impl Future<Output = Option<String>> + Send;
}

/// Usernames read from the tenant's realm, remembered for the process.
#[derive(Default)]
pub struct KeycloakUsernames {
    known: Mutex<HashMap<(Uuid, String), String>>,
}

impl Usernames for KeycloakUsernames {
    async fn username(&self, tenant_id: Uuid, user_id: &str) -> Option<String> {
        let key = (tenant_id, user_id.to_string());
        if let Some(name) = self.known.lock().ok()?.get(&key) {
            return Some(name.clone());
        }
        let realm = get_tenant_realm(&tenant_id.to_string());
        let user = match KeycloakAdminClient::new().await {
            Ok(client) => client.get_user(&realm, user_id).await,
            Err(error) => Err(error),
        };
        match user {
            Ok(user) => {
                let name = user.username?;
                self.known.lock().ok()?.insert(key, name.clone());
                Some(name)
            }
            Err(error) => {
                warn!(%tenant_id, user_id, "Error reading an account's username: {error:#}");
                None
            }
        }
    }
}

/// The log a pass posts to, and the keys that sign its entries.
pub struct TargetLog {
    pub board: String,
    /// What the signed statements name as their event: the election event,
    /// or the tenant for the tenant's log.
    pub statement_event: String,
    pub system_key: SigningKey,
    pub admin_keys: HashMap<String, SigningKey>,
    /// The entries are of an election event that no longer exists, so they
    /// go to the tenant's log.
    pub rerouted_from: Option<Uuid>,
}

async fn unposted_users(
    hasura_transaction: &Transaction<'_>,
    target: AuditLogTarget,
) -> Result<Vec<(String, Option<String>)>> {
    hasura_transaction
        .query(
            "SELECT user_id, max(username)
             FROM sequent_backend.audit_log_outbox
             WHERE tenant_id = $1 AND election_event_id IS NOT DISTINCT FROM $2
                 AND posted_at IS NULL AND event_type = 'USER' AND user_id IS NOT NULL
             GROUP BY user_id",
            &[&target.tenant_id, &target.election_event_id],
        )
        .await
        .context("Error listing the people with unposted audit log entries")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect()
}

async fn event_board(
    hasura_transaction: &Transaction<'_>,
    target: AuditLogTarget,
) -> Result<Option<String>> {
    let Some(election_event_id) = target.election_event_id else {
        return Ok(None);
    };
    let row = hasura_transaction
        .query_opt(
            "SELECT bulletin_board_reference FROM sequent_backend.election_event
             WHERE tenant_id = $1 AND id = $2",
            &[&target.tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the election event of audit log entries")?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(get_election_event_board(row.try_get(0)?))
}

/// Loads a log's board and keys in a transaction of its own, creating the
/// tenant's log and the admins' keys the first time: a key made here is kept
/// even if the posting transaction rolls back.
#[instrument(skip(client, usernames), err)]
pub async fn prepare_target_log<U: Usernames>(
    client: &mut Client,
    target: AuditLogTarget,
    usernames: &U,
) -> Result<TargetLog> {
    let tenant = target.tenant_id.to_string();
    let transaction = client
        .transaction()
        .await
        .context("Error starting to prepare an audit log")?;
    let mut log = match event_board(&transaction, target).await? {
        Some(board) => {
            let event = target
                .election_event_id
                .map(|id| id.to_string())
                .unwrap_or_default();
            let system_key =
                get_board_signing_key(&transaction, &tenant, Some(&event), &board).await?;
            TargetLog {
                board,
                statement_event: event,
                system_key,
                admin_keys: HashMap::new(),
                rerouted_from: None,
            }
        }
        None => {
            let tenant_log = ensure_tenant_log(&transaction, &tenant).await?;
            TargetLog {
                board: tenant_log.board,
                statement_event: tenant.clone(),
                system_key: tenant_log.system_key,
                admin_keys: HashMap::new(),
                rerouted_from: target.election_event_id,
            }
        }
    };
    for (user_id, username) in unposted_users(&transaction, target).await? {
        // The public key is announced under the name the entries will carry.
        let username = match username {
            Some(name) => Some(name),
            None => usernames.username(target.tenant_id, &user_id).await,
        };
        let key = vault::get_admin_user_signing_key(
            &transaction,
            &tenant,
            &user_id,
            username,
            None,
            None,
        )
        .await?;
        log.admin_keys.insert(user_id, key);
    }
    transaction
        .commit()
        .await
        .context("Error ending the preparation of an audit log")?;
    Ok(log)
}

/// An entry signed for its log.
pub struct SignedEntry {
    pub row: AuditLogOutboxRow,
    pub username: Option<String>,
    pub message: Message,
    pub entry: LogEntry,
}

/// Signs an outbox row for `log`. A USER entry is signed by its person's
/// admin key; a SYSTEM entry, by the log's system key alone.
pub fn sign_row(
    row: AuditLogOutboxRow,
    username: Option<String>,
    log: &TargetLog,
) -> Result<SignedEntry> {
    let sender = match (&row.event_type, &row.user_id) {
        (StatementEventType::USER, Some(user_id)) => Some((
            log.admin_keys
                .get(user_id)
                .ok_or_else(|| anyhow!("No admin key loaded for audit log entry {}", row.id))?,
            username.clone().unwrap_or_else(|| user_id.clone()),
        )),
        _ => None,
    };
    let signing_data = match sender {
        Some((key, name)) => SigningData::new(key.clone(), &name, log.system_key.clone()),
        None => SigningData::new(log.system_key.clone(), "", log.system_key.clone()),
    };
    let mut details = row.details.clone();
    if let (Some(name), Some(actor)) = (&username, details.get_mut("actor")) {
        if actor.get("username").is_none_or(Value::is_null) {
            actor["username"] = json!(name);
        }
    }
    if let Some(event) = log.rerouted_from {
        details["rerouted_from_election_event"] = json!(event);
    }
    let timestamp = u64::try_from(row.occurred_at.timestamp())
        .context("An audit log entry happened before 1970")?;
    let message = Message::audit_message(
        EventIdString(log.statement_event.clone()),
        AuditLogEntry {
            kind: row.statement_kind,
            event_type: row.event_type.clone(),
            log_type: row.log_type.clone(),
            description: row.description.clone(),
            details_json: details.to_string(),
            entry_id: row.id.to_string(),
        },
        timestamp,
        &signing_data,
        row.user_id.clone(),
        username.clone(),
        row.election_id.map(|id| id.to_string()),
        row.area_id.map(|id| id.to_string()),
    )?;
    let entry = LogEntry {
        delivery_id: delivery_id(row.id),
        message: ElectoralLogMessage::try_from(&message)?,
    };
    Ok(SignedEntry {
        row,
        username,
        message,
        entry,
    })
}

/// The JSON line monitoring collects for a posted entry: who, what and when,
/// with the names of the changed fields but never their values.
pub fn monitoring_line(board: &str, record_id: Option<i64>, stored: &ElectoralLogMessage) -> Value {
    let message = Message::record_deserialize(&stored.message).ok();
    let head = message.as_ref().map(|message| &message.statement.head);
    let (mut object, mut action, mut changed, mut ip_address) =
        (Value::Null, Value::Null, Value::Null, Value::Null);
    match message.as_ref().map(|message| &message.statement.body) {
        Some(StatementBody::Audit(entry)) => {
            let details: Value = serde_json::from_str(&entry.details_json).unwrap_or(Value::Null);
            match entry.kind {
                AuditStatementKind::AdminChange => {
                    object = details["object"].clone();
                    action = details["action"].clone();
                    changed = details["changes"]
                        .as_array()
                        .map(|changes| {
                            changes
                                .iter()
                                .map(|change| change["path"].clone())
                                .collect()
                        })
                        .unwrap_or(Value::Null);
                }
                AuditStatementKind::KeycloakAdminEvent => {
                    object = json!({
                        "type": details["resource_type"],
                        "id": details["resource_path"],
                    });
                    action = details["operation"].clone();
                    ip_address = details["actor"]["ip_address"].clone();
                }
                _ => {}
            }
        }
        Some(StatementBody::KeycloakUserEvent(text, event)) => {
            action = json!(event.0);
            if let Ok(details) = serde_json::from_str::<Value>(&text.0) {
                ip_address = details["ip_address"].clone();
            }
        }
        _ => {}
    }
    json!({
        "target": MONITORING_TARGET,
        "log": board,
        "entry_id": record_id,
        "kind": stored.statement_kind,
        "event_type": head.map(|head| head.event_type.to_string()),
        "log_type": head.map(|head| head.log_type.to_string()),
        "time": DateTime::<Utc>::from_timestamp(stored.statement_timestamp, 0)
            .map(|time| time.to_rfc3339_opts(SecondsFormat::Secs, true)),
        "user_id": stored.user_id,
        "username": stored.username,
        "object": object,
        "action": action,
        "changed_fields": changed,
        "ip_address": ip_address,
        "entry_sha256": hex::encode(Sha256::digest(&stored.message)),
    })
}

/// Writes a monitoring line to the process's output, one JSON object per line.
pub fn emit_monitoring_line(line: &Value) {
    let mut output = std::io::stdout().lock();
    if let Err(error) = writeln!(output, "{line}") {
        warn!("Error writing an audit log monitoring line: {error}");
    }
}

/// What a pass over one log did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassOutcome {
    /// Entries posted.
    Posted(usize),
    /// Another worker holds the log.
    Busy,
    /// The first entry failed before and waits to be tried again.
    Backoff,
}

fn error_text(error: &anyhow::Error) -> String {
    format!("{error:#}").chars().take(2000).collect()
}

async fn fail(
    hasura_transaction: &Transaction<'_>,
    target: AuditLogTarget,
    id: Uuid,
    error: &anyhow::Error,
) -> Result<()> {
    let attempts = mark_audit_log_outbox_failed(hasura_transaction, id, &error_text(error)).await?;
    if attempts >= ALERT_ATTEMPTS {
        error!(?target, %id, attempts,
            "An audit log entry keeps failing; its log is held up: {error:#}");
    } else {
        warn!(?target, %id, attempts, "Error posting an audit log entry: {error:#}");
    }
    Ok(())
}

/// Posts a log's unposted entries, in order, and marks them posted. The first
/// entry that doesn't read or sign is marked failed and ends the pass, after
/// the entries before it are posted. When the log refuses the entries,
/// nothing is marked posted and the first is marked failed.
#[instrument(skip(client, usernames, now), err)]
pub async fn post_target<U: Usernames>(
    client: &mut Client,
    target: AuditLogTarget,
    usernames: &U,
    now: DateTime<Utc>,
) -> Result<PassOutcome> {
    let log = prepare_target_log(client, target, usernames).await?;
    let transaction = client
        .transaction()
        .await
        .context("Error starting to post an audit log")?;
    if !try_lock_audit_log_worker(&transaction, target).await? {
        return Ok(PassOutcome::Busy);
    }
    let rows = fetch_unposted_audit_log_outbox(&transaction, target, OUTBOX_BATCH).await?;
    if let Some((_, Ok(first))) = rows.first() {
        let waits = first
            .last_attempt_at
            .is_some_and(|last| first.attempts > 0 && now < retry_after(first.attempts, last));
        if waits {
            return Ok(PassOutcome::Backoff);
        }
    }
    let mut signed = Vec::with_capacity(rows.len());
    let mut failed = None;
    for (id, row) in rows {
        let entry = match row {
            Ok(row) => {
                let username = match (&row.username, &row.user_id) {
                    (Some(name), _) => Some(name.clone()),
                    (None, Some(user_id)) => usernames.username(target.tenant_id, user_id).await,
                    (None, None) => None,
                };
                sign_row(row, username, &log)
            }
            Err(error) => Err(error),
        };
        match entry {
            Ok(entry) => signed.push(entry),
            Err(error) => {
                failed = Some((id, error));
                break;
            }
        }
    }
    let entries: Vec<LogEntry> = signed.iter().map(|entry| entry.entry.clone()).collect();
    let stored = match get_board_client().await?.append(&log.board, &entries).await {
        Ok(stored) => stored,
        Err(error) => {
            if let Some(first) = signed.first() {
                fail(&transaction, target, first.row.id, &error).await?;
                transaction.commit().await?;
            }
            return Err(error.context(format!("Error posting to log {}", log.board)));
        }
    };
    let ids: Vec<Uuid> = signed.iter().map(|entry| entry.row.id).collect();
    mark_audit_log_outbox_posted(&transaction, &ids).await?;
    if let Some((id, error)) = &failed {
        fail(&transaction, target, *id, error).await?;
    }
    transaction
        .commit()
        .await
        .context("Error marking audit log entries posted")?;
    let record_ids: HashMap<&str, i64> = stored
        .iter()
        .map(|record| (record.delivery_id.as_str(), record.id))
        .collect();
    for entry in &signed {
        emit_monitoring_line(&monitoring_line(
            &log.board,
            record_ids.get(entry.entry.delivery_id.as_str()).copied(),
            &entry.entry.message,
        ));
    }
    match failed {
        Some((id, error)) => Err(error.context(format!("Audit log entry {id} can't be posted"))),
        None => Ok(PassOutcome::Posted(signed.len())),
    }
}

/// One pass over every log with unposted entries. Each log has its own
/// transactions: one that fails doesn't hold up the others. Ends with the
/// outbox's waiting time as a monitoring line when entries still wait.
#[instrument(skip_all, err)]
pub async fn post_audit_log_outboxes<U: Usernames>(
    client: &mut Client,
    usernames: &U,
) -> Result<Vec<(AuditLogTarget, Result<PassOutcome>)>> {
    let targets = {
        let transaction = client
            .transaction()
            .await
            .context("Error starting to list the audit log outboxes")?;
        let targets = list_audit_log_outbox_targets(&transaction).await?;
        transaction.commit().await?;
        targets
    };
    let mut outcomes = Vec::with_capacity(targets.len());
    for target in targets {
        let outcome = post_target(client, target, usernames, Utc::now()).await;
        match &outcome {
            Ok(PassOutcome::Posted(posted)) => info!(?target, posted, "Audit log posted"),
            Ok(_) => {}
            Err(error) => warn!(?target, "Error posting the audit log: {error:#}"),
        }
        outcomes.push((target, outcome));
    }
    let transaction = client.transaction().await?;
    let (waiting, oldest_seconds) = audit_log_outbox_backlog(&transaction).await?;
    transaction.commit().await?;
    if waiting > 0 {
        emit_monitoring_line(&json!({
            "target": MONITORING_OUTBOX_TARGET,
            "waiting": waiting,
            "oldest_seconds": oldest_seconds,
        }));
    }
    Ok(outcomes)
}

/// The writer starting or stopping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogWriterEvent {
    Started,
    Stopped,
}

/// The entry of a writer event for a tenant's log: the release that runs and
/// the last entry of the tenant's log the outbox posted.
pub fn log_writer_entry(
    event: LogWriterEvent,
    tenant_id: Uuid,
    release: &str,
    last_entry_id: Option<Uuid>,
) -> NewAuditLogEntry {
    let (statement_kind, verb) = match event {
        LogWriterEvent::Started => (AuditStatementKind::LogWriterStarted, "started"),
        LogWriterEvent::Stopped => (AuditStatementKind::LogWriterStopped, "stopped"),
    };
    NewAuditLogEntry {
        target: AuditLogTarget {
            tenant_id,
            election_event_id: None,
        },
        statement_kind,
        event_type: StatementEventType::SYSTEM,
        log_type: StatementLogType::INFO,
        user_id: None,
        username: None,
        election_id: None,
        area_id: None,
        description: format!("The log writer {verb} (release {release})"),
        details: json!({"release": release, "last_entry_id": last_entry_id}),
    }
}

/// Queues a writer event in every tenant's log. Returns the tenants it was
/// queued for.
#[instrument(skip(client), err)]
pub async fn record_log_writer(client: &mut Client, event: LogWriterEvent) -> Result<usize> {
    let release = std::env::var(ENV_VAR_APP_VERSION).unwrap_or_else(|_| "-".to_string());
    let transaction = client
        .transaction()
        .await
        .context("Error starting to record a log writer event")?;
    let tenants = transaction
        .query(
            "SELECT t.id,
                    (SELECT o.id FROM sequent_backend.audit_log_outbox o
                     WHERE o.tenant_id = t.id AND o.election_event_id IS NULL
                         AND o.posted_at IS NOT NULL
                     ORDER BY o.seq DESC LIMIT 1)
             FROM sequent_backend.tenant t ORDER BY t.id",
            &[],
        )
        .await
        .context("Error listing the tenants of a log writer event")?;
    for row in &tenants {
        let entry = log_writer_entry(event, row.try_get(0)?, &release, row.try_get(1)?);
        insert_audit_log_outbox(&transaction, &entry).await?;
    }
    transaction
        .commit()
        .await
        .context("Error recording a log writer event")?;
    Ok(tenants.len())
}
