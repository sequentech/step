// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::services::celery_app::get_celery_app;
use crate::services::celery_app::get_celery_connection;
use crate::services::celery_app::Queue;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::database::PgConfig;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_queue::{
    drain_log_event_queue, AmqpLogEventQueue, LogEventPreparation, LogEventStore,
};
use crate::services::protocol_manager::get_board_client;
use crate::services::users::get_user_area_id;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::client::board_client::ElectoralLogMessage;
use immudb_rs::TxMode;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

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
/// This task is routed to the durable electoral_log_batch_queue.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn enqueue_electoral_log_event(input: LogEventInput) -> Result<()> {
    // By calling this task, the event is enqueued into the electoral_log_batch_queue.
    Ok(())
}

/// Moves a batch queued by an earlier version back to the electoral log event
/// queue, where the dispatcher acknowledges each event once it is stored.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_electoral_log_events_batch(events: Vec<LogEventInput>) -> Result<()> {
    let celery_app = get_celery_app().await;
    for input in events {
        celery_app
            .send_task(enqueue_electoral_log_event::new(input))
            .await
            .with_context(|| "Error requeuing electoral log event")?;
    }
    Ok(())
}

/// Builds the entries of one event. Errors from the databases or the vault are
/// returned as errors so the event is retried; an event that can never be
/// written is rejected.
async fn prepare_log_event(
    hasura_tx: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    input: &LogEventInput,
) -> anyhow::Result<LogEventPreparation<ElectoralLogMessage>> {
    if let Err(err) =
        parse_uuid_v4(&input.tenant_id).and_then(|_| parse_uuid_v4(&input.election_event_id))
    {
        return Ok(LogEventPreparation::Rejected(err));
    }
    let Some(election_event) =
        get_election_event_by_id_if_exist(hasura_tx, &input.tenant_id, &input.election_event_id)
            .await
            .with_context(|| "Error getting election event")?
    else {
        return Ok(LogEventPreparation::Rejected(anyhow!(
            "Election event {} not found",
            input.election_event_id
        )));
    };
    let Some(board) = get_election_event_board(election_event.bulletin_board_reference.clone())
    else {
        return Ok(LogEventPreparation::Rejected(anyhow!(
            "Election event {} has no board",
            input.election_event_id
        )));
    };

    let messages = match &input.message_type {
        LogMessageType::Internal => deserialize_str(&input.body.as_raw())
            .map(|message| vec![message])
            .with_context(|| "Error parsing input.body into a ElectoralLogMessage"),
        LogMessageType::KeycloakEvent(event_type) => {
            let user_id = input
                .user_id
                .clone()
                .unwrap_or_else(|| "unknown_user".into());
            let realm = get_event_realm(&input.tenant_id, &input.election_event_id);
            let user_area_id = get_user_area_id(keycloak_transaction, &realm, &user_id)
                .await
                .with_context(|| "Error getting user area id")?;
            let electoral_log = ElectoralLog::new(
                hasura_tx,
                &input.tenant_id,
                Some(&election_event.id),
                &board,
            )
            .await
            .with_context(|| "Error initializing electoral log")?;
            build_keycloak_event_messages(&electoral_log, input, event_type, user_id, user_area_id)
        }
    };

    Ok(match messages {
        Ok(messages) => LogEventPreparation::Ready { board, messages },
        Err(err) => LogEventPreparation::Rejected(err),
    })
}

fn build_keycloak_event_messages(
    electoral_log: &ElectoralLog,
    input: &LogEventInput,
    event_type: &str,
    user_id: String,
    user_area_id: Option<String>,
) -> anyhow::Result<Vec<ElectoralLogMessage>> {
    let mut messages = Vec::new();
    if let LogEventBody::Communications(ref template_body) = input.body {
        messages.push(
            electoral_log
                .build_send_template_message(
                    Some(template_body.clone()),
                    input.election_event_id.clone(),
                    Some(user_id.clone()),
                    input.username.clone(),
                    None,
                    user_area_id.clone(),
                )
                .with_context(|| "Error building send template message")?,
        );
    }
    messages.push(
        electoral_log
            .build_keycloak_event_message(
                input.election_event_id.clone(),
                event_type.to_string(),
                input.body.as_raw(),
                Some(user_id),
                input.username.clone(),
                user_area_id,
            )
            .with_context(|| "Error building keycloak event message")?,
    );
    Ok(messages)
}

/// Writes electoral log events to the board of their election event.
struct ElectoralLogBoards;

#[async_trait]
impl LogEventStore for ElectoralLogBoards {
    type Message = ElectoralLogMessage;

    /// Uses a single Hasura transaction to fetch the event details of the
    /// whole batch.
    async fn prepare(
        &mut self,
        events: &[LogEventInput],
    ) -> anyhow::Result<Vec<LogEventPreparation<ElectoralLogMessage>>> {
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

        let mut preparations = Vec::with_capacity(events.len());
        for input in events {
            preparations.push(prepare_log_event(&hasura_tx, &keycloak_transaction, input).await?);
        }

        hasura_tx
            .commit()
            .await
            .with_context(|| "Error committing Hasura transaction")?;
        Ok(preparations)
    }

    async fn persist(
        &mut self,
        board: &str,
        messages: Vec<ElectoralLogMessage>,
    ) -> anyhow::Result<()> {
        let mut board_client = get_board_client().await?;
        board_client.open_session(board).await?;
        let immudb_tx = board_client.new_tx(TxMode::ReadWrite).await?;
        board_client
            .insert_electoral_log_messages_batch(&immudb_tx, &messages)
            .await
            .with_context(|| {
                format!(
                    "Error inserting batch electoral log messages for board {}",
                    board
                )
            })?;
        board_client
            .commit(&immudb_tx)
            .await
            .with_context(|| format!("Error committing immudb transaction for board {}", board))?;
        // The entries are committed: a failed close must not put them back in the queue.
        if let Err(err) = board_client.close_session().await {
            warn!(%board, error = ?err, "Error closing immudb session");
        }
        Ok(())
    }
}

/// Dispatcher: reads batches of events from the electoral_log_event_queue,
/// writes them to their boards and acknowledges each event only once its
/// entries are committed. Events that can never be written are moved to the
/// electoral_log_dead_letter_queue.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
// No time_limit: a batch stays in the queue until it is committed, so a
// deadline shorter than a large batch would retry it forever.
#[celery::task(max_retries = 0, expires = 1)]
pub async fn electoral_log_batch_dispatcher() -> Result<()> {
    info!("starting electoral_log_batch_dispatcher");

    // Reuse the global AMQP connection.
    let connection_arc = get_celery_connection().await?;
    let channel = connection_arc
        .create_channel()
        .await
        .with_context(|| "Error creating RabbitMQ channel")?;

    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let mut queue = AmqpLogEventQueue::new(
        channel,
        Queue::ElectoralLogEvent.queue_name(&slug),
        Queue::ElectoralLogDeadLetter.queue_name(&slug),
    )
    .await?;

    // Get the batch size from PgConfig.
    let batch_size: usize = PgConfig::from_env()?.default_sql_batch_size.try_into()?;

    drain_log_event_queue(&mut queue, &mut ElectoralLogBoards, batch_size).await?;
    info!("finishing electoral_log_batch_dispatcher");
    Ok(())
}
