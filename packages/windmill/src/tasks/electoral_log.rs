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
use crate::services::electoral_log_board::{
    ensure_tenant_log, get_board_client, get_protocol_manager_secret_path,
};
use crate::services::electoral_log_dead_letter::{
    dead_letter_events, dead_letter_message, DeadLetterStage,
};
use crate::services::monitoring::login_counter::{
    count_login_attempts_apart, login_attempt, LoginAttempt,
};
use crate::services::tenant_log::{
    names_no_election_event, tenant_activity_message, TenantActivity,
};
use crate::services::users::get_user_area_id;
use crate::services::vault;
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
use electoral_log::{ElectoralLogMessage, LogEntry};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{event, info, instrument};
use uuid::Uuid;
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
/// appended, one transaction per board. Infrastructure failures fail the batch so that it
/// is retried; delivery IDs make the retry idempotent. The last retry dead-letters the
/// whole batch instead of dropping it.
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
        Err(error) if retries_exhausted(task.request().retries, task.max_retries()) => {
            tracing::error!(
                "Dead-lettering {} electoral-log events after the last retry: {error:#}",
                events.len()
            );
            let reason = format!("retries exhausted: {error:#}");
            let mut set_aside = Vec::with_capacity(events.len());
            for event in &events {
                set_aside.push((event, reason.as_str()));
            }
            dead_letter_events(&set_aside, DeadLetterStage::Batch).await?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

/// Whether a failed run was the last one Celery would make.
fn retries_exhausted(retries: u32, max_retries: Option<u32>) -> bool {
    max_retries.is_some_and(|max| retries >= max)
}

/// Build the records of a batch, dead-letter the events that cannot be stored, and
/// append the rest board by board.
async fn store_batch(events: &[LogEventInput]) -> anyhow::Result<()> {
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

    crate::services::privacy_signin::allocate_registered_voters(&hasura_tx, &keycloak_tx, events)
        .await?;
    crate::services::privacy_signin::retain_queued_signin_network(&hasura_tx, events).await?;
    let built = build_batch(
        &mut DbLookups {
            hasura: &hasura_tx,
            keycloak: &keycloak_tx,
        },
        events,
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
    }

    // Append every board before failing, so one failing board does not hold back the
    // others; a retry appends the failed boards again, idempotently.
    let client = get_board_client().await?;
    let mut failures = Vec::new();
    for (board, messages) in built.by_board {
        match client.append(&board, &messages).await {
            Ok(stored) => emit_monitoring_lines(&board, &messages, &stored),
            Err(error) => {
                tracing::error!("Error appending electoral-log batch for board {board}: {error:?}");
                failures.push(format!("{board}: {error:#}"));
            }
        }
    }
    anyhow::ensure!(
        failures.is_empty(),
        "Error appending electoral-log batches: {}",
        failures.join("; ")
    );
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
}

/// The records of a batch, grouped by board, the events that cannot be stored,
/// and the sign-in attempts of the events stored, each under the SHA-256 of its
/// delivery ID (the monitoring counters' receipts take a 64-digit hex ID).
#[derive(Default)]
struct BuiltBatch {
    by_board: HashMap<String, Vec<LogEntry>>,
    dead_letters: Vec<(LogEventInput, String)>,
    login_attempts: Vec<(String, LoginAttempt)>,
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
}

struct DbLookups<'a, 'b> {
    hasura: &'a Transaction<'b>,
    keycloak: &'a Transaction<'b>,
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
        let Some(contents) = vault::read_secret(
            self.hasura,
            tenant_id,
            Some(election_event_id),
            &get_protocol_manager_secret_path(board),
        )
        .await
        .context("Error reading the protocol-manager key")?
        else {
            return Ok(None);
        };
        let protocol_manager =
            crate::services::electoral_log_board::parse_signing_identity(&contents)
                .context("Error decoding the protocol-manager key")?;
        Ok(Some(ElectoralLog::for_system_with_signing_key(
            board,
            &protocol_manager,
        )))
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
}

/// Lookups already made for this batch, by (tenant, election event), and the
/// tenants' logs by tenant.
#[derive(Default)]
struct BatchCache {
    boards: HashMap<(String, String), Option<String>>,
    signers: HashMap<(String, String), Option<ElectoralLog>>,
    tenant_logs: HashMap<String, Option<(String, ElectoralLog)>>,
}

async fn build_batch<L: BatchLookups + Send>(
    lookups: &mut L,
    events: &[LogEventInput],
) -> anyhow::Result<BuiltBatch> {
    let mut cache = BatchCache::default();
    let mut built = BuiltBatch::default();
    for original in events {
        let input = crate::services::privacy_signin::sanitized_event(original);
        match build_event(lookups, &mut cache, &input).await {
            Ok(event) => {
                for (board, entry) in event.entries {
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
        .map_err(Permanent)?;
    // The tenant realm's events and the requests made outside an election
    // event name none.
    if names_no_election_event(&input.election_event_id) {
        return build_tenant_event(lookups, cache, input, delivery_id, None).await;
    }
    Uuid::parse_str(&input.election_event_id)
        .context("Invalid election event ID")
        .map_err(Permanent)?;

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
        LogMessageType::Internal => deserialize_str::<ElectoralLogMessage>(&input.body.as_raw())
            .context("Error parsing the body as an ElectoralLogMessage")
            .map_err(Permanent)?,
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
            if !cache.signers.contains_key(&key) {
                let signer = lookups
                    .signer(&input.tenant_id, &input.election_event_id, &board)
                    .await
                    .map_err(Transient)?;
                cache.signers.insert(key.clone(), signer);
            }
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

            if let LogEventBody::Communications(ref template_body) = input.body {
                let send_template_msg = signer
                    .build_send_template_message(
                        Some(template_body.clone()),
                        input.election_event_id.clone(),
                        Some(user_id.clone()),
                        input.username.clone(),
                        None,
                        user_area_id.clone(),
                    )
                    .context("Error building send template message")
                    .map_err(Permanent)?;
                entries.push((
                    board.clone(),
                    LogEntry {
                        delivery_id: format!("{delivery_id}:communication"),
                        message: send_template_msg,
                    },
                ));
            }

            signer
                .build_keycloak_event_message(
                    input.election_event_id.clone(),
                    event_type.clone(),
                    input.body.as_raw(),
                    Some(user_id),
                    input.username.clone(),
                    user_area_id,
                )
                .context("Error building keycloak event message")
                .map_err(Permanent)?
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
        .map_err(Permanent)?;
        ElectoralLogMessage::try_from(&message).map_err(Permanent)?
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
            .map_err(Permanent)?;
        tenant_activity_message(
            signer,
            &input.tenant_id,
            &activity,
            occurred_at,
            present(&input.user_id),
            present(&input.username),
        )
        .context("Error building the tenant activity message")
        .map_err(Permanent)?
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
    use EventFailure::Permanent;

    let message = if event_type == KEYCLOAK_ADMIN_EVENT && unplaced.is_none() {
        let author = input
            .admin_event
            .as_ref()
            .map(|event| admin_event_author(event, service_client))
            .unwrap_or(AdminEventAuthor::Operator);
        admin_event_message(signer, &input.tenant_id, input, delivery_id, author)
            .map_err(Permanent)?
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
        .map_err(Permanent)?;
        ElectoralLogMessage::try_from(&message).map_err(Permanent)?
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
    use EventFailure::{Permanent, Transient};

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
    if !cache.signers.contains_key(&key) {
        let signer = lookups
            .signer(&input.tenant_id, &input.election_event_id, &board)
            .await
            .map_err(Transient)?;
        cache.signers.insert(key.clone(), signer);
    }
    let Some(signer) = cache.signers.get(&key).and_then(Option::as_ref) else {
        let reason = format!(
            "Election event {} has no protocol-manager key",
            input.election_event_id
        );
        return build_tenant_event(lookups, cache, input, delivery_id, Some(reason)).await;
    };
    let message = admin_event_message(signer, &input.election_event_id, input, delivery_id, author)
        .map_err(Permanent)?;
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
#[celery::task(time_limit = 30, max_retries = 0, expires = 1)]
pub async fn electoral_log_batch_dispatcher() -> Result<()> {
    let limits = BatchLimits::from_env()?;
    dispatch_electoral_log_batches(
        get_queue_pool().await.as_ref(),
        &DispatchQueues::default(),
        limits,
    )
    .await
}

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
    use voting_crypto::signing::SigningKey;
    use voting_crypto::RecordDeserialize;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const EVENT: &str = "fdd21db2-dd68-4974-90eb-7f2750b2b5df";
    const DELETED_EVENT: &str = "6f0c5c2e-3f43-4a5e-9d55-0d6a3e1f7b21";
    const KEYLESS_EVENT: &str = "2b8e4a1d-7c6f-4e93-b0a2-5d9c8f1e3a47";
    const UNKNOWN_TENANT: &str = "5a3c1f0e-9b7d-4c2a-8e6f-1d0b9a8c7e65";

    struct FakeLookups {
        signing_key: SigningKey,
        unreachable_event: Option<&'static str>,
        unreachable_tenant: Option<&'static str>,
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
                board_calls: 0,
                signer_calls: 0,
                tenant_log_calls: 0,
            }
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
            let signer = ElectoralLog::for_system_with_signing_key(&board, &self.signing_key);
            Ok((tenant_id != UNKNOWN_TENANT).then_some((board, signer)))
        }

        fn service_client(&self) -> String {
            "step".to_string()
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
        }
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
