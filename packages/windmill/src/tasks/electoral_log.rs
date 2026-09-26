// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::celery_app::get_celery_connection;
use crate::services::celery_app::Queue;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::database::PgConfig;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_queue::drain_electoral_log_queue;
use crate::services::protocol_manager::get_board_client;
use crate::services::users::get_user_area_id;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, ensure, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use electoral_log::client::board_client::{retry_electoral_log_transaction, ElectoralLogMessage};
use immudb_rs::TxMode;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{instrument, warn};

use lapin::message::BasicGetMessage;

/// Classifies the type of an incoming log event.
///
/// Serializes as a plain string for wire compatibility:
/// - `Internal`            → `"internal"`
/// - `KeycloakEvent(s)`   → `s` (the raw Keycloak event type, e.g. `"LOGIN"`)
#[derive(Clone, Debug, PartialEq)]
pub enum LogMessageType {
    Internal,
    KeycloakEvent(String),
}

impl Serialize for LogMessageType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            LogMessageType::Internal => serializer.serialize_str("internal"),
            LogMessageType::KeycloakEvent(event_type) => serializer.serialize_str(event_type),
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEventInput {
    pub election_event_id: String,
    pub message_type: LogMessageType,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub tenant_id: String,
    pub body: LogEventBody,
}

/// Enqueue the electoral log event.
/// This envelope is drained from the durable electoral_log_event_queue.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn enqueue_electoral_log_event(input: LogEventInput) -> Result<()> {
    // The dispatcher owns delivery and acknowledgement; this task is not consumed directly.
    Ok(())
}

/// Legacy wire signature retained for batches already in RabbitMQ. Worker queue
/// selection redirects this queue to the durable dispatcher; it must not be
/// consumed with Celery's acknowledge-on-final-error behavior.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(bind = true, max_retries = 0)]
pub async fn process_electoral_log_events_batch(
    task: &Self,
    events: Vec<LogEventInput>,
) -> Result<()> {
    let deliveries = identify_events(&task.request.correlation_id, true, events)?;
    persist_electoral_log_deliveries(deliveries).await?;
    Ok(())
}

#[derive(Debug)]
pub struct IdentifiedLogEvent {
    pub delivery_id: String,
    pub payload_hash: String,
    pub input: LogEventInput,
}

fn identify_events(
    id: &str,
    legacy_batch: bool,
    events: Vec<LogEventInput>,
) -> anyhow::Result<Vec<IdentifiedLogEvent>> {
    ensure!(
        !id.is_empty(),
        "electoral log delivery has no stable correlation ID"
    );
    events
        .into_iter()
        .enumerate()
        .map(|(index, input)| {
            // Namespace original events separately from old batch task IDs, and
            // distinguish each event within a legacy batch without using delivery tags.
            let identity = if legacy_batch {
                format!("batch:{id}:{index}")
            } else {
                format!("event:{id}")
            };
            Ok(IdentifiedLogEvent {
                delivery_id: hex::encode(Sha256::digest(identity.as_bytes())),
                payload_hash: hex::encode(Sha256::digest(serde_json::to_vec(&input)?)),
                input,
            })
        })
        .collect()
}

pub fn decode_electoral_log_delivery(
    delivery: &BasicGetMessage,
    legacy_batch: bool,
) -> anyhow::Result<Vec<IdentifiedLogEvent>> {
    let id = delivery
        .properties
        .correlation_id()
        .as_ref()
        .ok_or_else(|| anyhow!("electoral log delivery has no stable correlation ID"))?;
    let value: serde_json::Value =
        serde_json::from_slice(&delivery.data).context("Error parsing Celery message as JSON")?;
    let payload = value
        .as_array()
        .and_then(|array| array.get(1))
        .ok_or_else(|| anyhow!("Invalid Celery message: expected arguments array"))?;
    let events = if legacy_batch {
        serde_json::from_value(
            payload
                .get("events")
                .cloned()
                .ok_or_else(|| anyhow!("Missing events in legacy electoral log batch"))?,
        )?
    } else {
        vec![serde_json::from_value(
            payload
                .get("input")
                .cloned()
                .ok_or_else(|| anyhow!("Missing input in electoral log event"))?,
        )?]
    };
    identify_events(id.as_str(), legacy_batch, events)
}

async fn persist_electoral_log_deliveries(events: Vec<IdentifiedLogEvent>) -> anyhow::Result<()> {
    let mut messages_by_board: HashMap<
        String,
        Vec<(IdentifiedLogEvent, Vec<ElectoralLogMessage>)>,
    > = HashMap::new();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting DB pool for batch processing")?;
    let hasura_tx = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting Hasura transaction")?;

    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting keycloak DB pool for batch processing")?;
    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .with_context(|| "Error starting keycloak transaction")?;

    for delivery in events {
        let input = &delivery.input;
        let mut messages = Vec::new();
        let election_event =
            get_election_event_by_id(&hasura_tx, &input.tenant_id, &input.election_event_id)
                .await
                .with_context(|| "Error getting election event")?;

        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "Error getting election event board")?;

        let event_message = match &input.message_type {
            LogMessageType::Internal => {
                let message: ElectoralLogMessage = deserialize_str(&input.body.as_raw())
                    .with_context(|| "Error parsing input.body into a ElectoralLogMessage")?;
                message
            }
            LogMessageType::KeycloakEvent(event_type) => {
                let user_id = input
                    .user_id
                    .clone()
                    .unwrap_or_else(|| "unknown_user".into());
                let username = input.username.clone();
                let realm = get_event_realm(&input.tenant_id, &input.election_event_id);
                let user_area_id = get_user_area_id(&keycloak_transaction, &realm, &user_id)
                    .await
                    .with_context(|| "Error getting user area id")?;
                let electoral_log = ElectoralLog::new(
                    &hasura_tx,
                    &input.tenant_id,
                    Some(&election_event.id),
                    &board_name,
                )
                .await
                .with_context(|| "Error initializing electoral log")?;

                if let LogEventBody::Communications(ref template_body) = input.body {
                    let send_template_msg = electoral_log
                        .build_send_template_message(
                            Some(template_body.clone()),
                            input.election_event_id.clone(),
                            Some(user_id.clone()),
                            username.clone(),
                            None,
                            user_area_id.clone(),
                        )
                        .with_context(|| "Error building send template message")?;
                    messages.push(send_template_msg);
                }

                electoral_log
                    .build_keycloak_event_message(
                        input.election_event_id.clone(),
                        event_type.clone(),
                        input.body.as_raw(),
                        Some(user_id.clone()),
                        username.clone(),
                        user_area_id,
                    )
                    .with_context(|| "Error building keycloak event message")?
            }
        };

        messages.push(event_message);
        messages_by_board
            .entry(board_name)
            .or_default()
            .push((delivery, messages));
    }

    hasura_tx
        .commit()
        .await
        .with_context(|| "Error committing Hasura transaction")?;

    for (board, messages) in messages_by_board.into_iter() {
        retry_electoral_log_transaction(|| async {
            let mut board_client = get_board_client().await?;
            board_client.open_session(&board).await?;
            let result = async {
                board_client
                    .ensure_electoral_log_delivery_receipts()
                    .await?;
                let immudb_tx = board_client.new_tx(TxMode::ReadWrite).await?;
                for (delivery, rows) in &messages {
                    board_client
                        .insert_electoral_log_delivery(
                            &immudb_tx,
                            &delivery.delivery_id,
                            &delivery.payload_hash,
                            rows,
                        )
                        .await
                        .with_context(|| {
                            format!("Error persisting electoral log delivery for board {board}")
                        })?;
                }
                board_client.commit(&immudb_tx).await.with_context(|| {
                    format!("Error committing immudb transaction for board {}", board)
                })?;
                Ok(())
            }
            .await;
            // Cleanup must neither hide a rejected transaction nor turn a
            // confirmed commit into a replay or prevent later boards' delivery.
            if let Err(error) = board_client.close_session().await {
                warn!(%board, ?error, "Error closing electoral log batch session");
            }
            result
        })
        .await?;
    }

    Ok(())
}

/// Retain the original durable messages until every board has confirmed its
/// transaction. Receipts make replays after a partial/ambiguous commit harmless.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
// Persistence inherits the former batch processor's lack of a task deadline:
// a fixed 30s cutoff could requeue a large valid batch before its first commit.
#[celery::task(max_retries = 0, expires = 1)]
pub async fn electoral_log_batch_dispatcher() -> Result<()> {
    let connection = get_celery_connection().await?;
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let batch_size: usize = PgConfig::from_env()?.default_sql_batch_size.try_into()?;
    // Drain old batches as well as newly published single events. No publish/ACK
    // handoff is involved, and cancellation drops the owned channel (requeueing).
    for (queue, legacy_batch) in [
        (Queue::ElectoralLogBatch, true),
        (Queue::ElectoralLogEvent, false),
    ] {
        let channel = connection.create_channel().await?;
        drain_electoral_log_queue(
            channel,
            &queue.queue_name(&slug),
            batch_size,
            |deliveries| async move {
                let mut events = Vec::new();
                for delivery in deliveries {
                    events.extend(decode_electoral_log_delivery(&delivery, legacy_batch)?);
                }
                persist_electoral_log_deliveries(events).await
            },
        )
        .await?;
    }
    Ok(())
}
