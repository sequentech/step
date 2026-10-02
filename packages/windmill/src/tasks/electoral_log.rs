// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::celery_app::Queue;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::database::PgConfig;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_board_client;
use crate::services::users::get_user_area_id;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use electoral_log::client::board_client::ElectoralLogMessage;
use immudb_rs::TxMode;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{event, instrument};

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

/// Process a batch of electoral log events.
/// Uses a single Hasura transaction to fetch event details and group messages by board,
/// then for each board group, opens an immudb session/transaction to insert all messages.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_electoral_log_events_batch(events: Vec<LogEventInput>) -> Result<()> {
    let mut messages_by_board: HashMap<String, Vec<ElectoralLogMessage>> = HashMap::new();

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

    for input in events.iter() {
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
                    messages_by_board
                        .entry(board_name.clone())
                        .or_insert_with(Vec::new)
                        .push(send_template_msg);
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

        messages_by_board
            .entry(board_name.clone())
            .or_insert_with(Vec::new)
            .push(event_message);
    }

    hasura_tx
        .commit()
        .await
        .with_context(|| "Error committing Hasura transaction")?;

    for (board, messages) in messages_by_board.into_iter() {
        let mut board_client = get_board_client().await?;
        board_client.open_session(&board).await?;
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
        board_client.close_session().await?;
    }

    Ok(())
}

/// Promote raw events into a processing task in the same PostgreSQL transaction.
/// The raw-event queue is deliberately excluded from normal Celery consumers.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 30, max_retries = 0, expires = 1)]
pub async fn electoral_log_batch_dispatcher() -> Result<()> {
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let batch_size = PgConfig::from_env()?.default_sql_batch_size;
    dispatch_electoral_log_batches(get_keycloak_pool().await.as_ref(), &slug, batch_size).await
}

async fn dispatch_electoral_log_batches(
    pool: &deadpool_postgres::Pool,
    slug: &str,
    batch_size: i32,
) -> Result<()> {
    let source = pgmq_broker::queue_name(&Queue::ElectoralLogEvent.queue_name(slug));
    let target = Queue::ElectoralLogBatch.queue_name(slug);
    if batch_size <= 0 {
        return Err(anyhow!("default_sql_batch_size must be positive").into());
    }
    loop {
        let mut client = pool
            .get()
            .await
            .context("Error obtaining PGMQ batch connection")?;
        let tx = client.transaction().await?;
        tx.batch_execute("SET LOCAL statement_timeout = '10s'")
            .await?;
        // pgmq.read retains row locks until commit, including while building the batch.
        let rows = tx
            .query(
                "SELECT msg_id, message FROM pgmq.read($1, 60, $2)",
                &[&source, &batch_size],
            )
            .await?;
        if rows.is_empty() {
            break;
        }
        let mut events = Vec::with_capacity(rows.len());
        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.get("msg_id");
            let decoded = (|| -> anyhow::Result<LogEventInput> {
                let message = pgmq_broker::decode(
                    row.get::<_, Option<serde_json::Value>>("message")
                        .unwrap_or(serde_json::Value::Null),
                )?;
                if message.headers.task != "enqueue_electoral_log_event" {
                    return Err(anyhow!("unexpected raw-event task"));
                }
                let body: serde_json::Value = serde_json::from_slice(&message.raw_body)?;
                let input = body
                    .get(1)
                    .and_then(|kwargs| kwargs.get("input"))
                    .ok_or_else(|| anyhow!("missing event input"))?;
                Ok(serde_json::from_value(input.clone())?)
            })();
            match decoded {
                Ok(input) => {
                    events.push(input);
                    ids.push(id);
                }
                Err(_) => {
                    tracing::error!(
                        message_id = id,
                        "Archiving malformed electoral-log queue message"
                    );
                    tx.query_one("SELECT pgmq.archive($1, $2::bigint)", &[&source, &id])
                        .await?;
                }
            }
        }
        if !events.is_empty() {
            let message = celery::protocol::Message::try_from(
                process_electoral_log_events_batch::new(events),
            )
            .context("Error encoding electoral-log batch")?;
            pgmq_broker::send(&*tx, &target, &message)
                .await
                .context("Error enqueueing electoral-log batch")?;
            tx.query("SELECT pgmq.delete($1, $2::bigint[])", &[&source, &ids])
                .await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

#[cfg(test)]
mod pgmq_tests {
    use super::*;
    use celery::broker::{Broker, BrokerBuilder};
    use pgmq_broker::PgmqBrokerBuilder;
    use std::sync::Arc;

    #[tokio::test]
    #[ignore = "requires PGMQ_TEST_DATABASE_URL pointing to a disposable PGMQ database"]
    async fn batch_handoff_is_atomic_and_quarantines_invalid_events() {
        let config = deadpool_postgres::Config {
            url: Some(std::env::var("PGMQ_TEST_DATABASE_URL").expect("PGMQ_TEST_DATABASE_URL")),
            ..Default::default()
        };
        let pool = Arc::new(
            config
                .create_pool(
                    Some(deadpool_postgres::Runtime::Tokio1),
                    tokio_postgres::NoTls,
                )
                .unwrap(),
        );
        let slug = format!("batch_{}", uuid::Uuid::new_v4());
        let source = Queue::ElectoralLogEvent.queue_name(&slug);
        let target = Queue::ElectoralLogBatch.queue_name(&slug);
        let broker = Box::new(PgmqBrokerBuilder::from_pool(pool.clone()))
            .declare_queue(&source)
            .declare_queue(&target)
            .build(5)
            .await
            .unwrap();
        for id in ["event-a", "event-b"] {
            let input = LogEventInput {
                election_event_id: id.into(),
                message_type: LogMessageType::KeycloakEvent("LOGIN_ERROR".into()),
                user_id: None,
                username: None,
                tenant_id: "test-tenant".into(),
                body: LogEventBody::Plain("test".into()),
            };
            broker
                .send(
                    &celery::protocol::Message::try_from(enqueue_electoral_log_event::new(input))
                        .unwrap(),
                    &source,
                )
                .await
                .unwrap();
        }
        let client = pool.get().await.unwrap();
        let source = pgmq_broker::queue_name(&source);
        let target = pgmq_broker::queue_name(&target);
        client
            .query_one(
                "SELECT pgmq.send($1, $2)",
                &[&source, &serde_json::json!({"invalid": true})],
            )
            .await
            .unwrap();
        client
            .query_one("SELECT pgmq.drop_queue($1)", &[&target])
            .await
            .unwrap();
        assert!(dispatch_electoral_log_batches(&pool, &slug, 10)
            .await
            .is_err());
        let count = client
            .query_one("SELECT queue_length FROM pgmq.metrics($1)", &[&source])
            .await
            .unwrap()
            .get::<_, i64>(0);
        assert_eq!(count, 3, "failed promotion must retain every source event");
        client
            .query_one("SELECT pgmq.create($1)", &[&target])
            .await
            .unwrap();
        dispatch_electoral_log_batches(&pool, &slug, 10)
            .await
            .unwrap();
        assert_eq!(
            client
                .query_one("SELECT queue_length FROM pgmq.metrics($1)", &[&source])
                .await
                .unwrap()
                .get::<_, i64>(0),
            0
        );
        let payload: serde_json::Value = client
            .query_one(&format!("SELECT message FROM pgmq.q_{target}"), &[])
            .await
            .unwrap()
            .get(0);
        let message = pgmq_broker::decode(payload).unwrap();
        assert_eq!(message.headers.task, "process_electoral_log_events_batch");
        let body: serde_json::Value = serde_json::from_slice(&message.raw_body).unwrap();
        assert_eq!(body[1]["events"].as_array().unwrap().len(), 2);
        assert_eq!(
            client
                .query_one(&format!("SELECT count(*) FROM pgmq.a_{source}"), &[])
                .await
                .unwrap()
                .get::<_, i64>(0),
            1
        );
        client
            .query_one("SELECT pgmq.drop_queue($1)", &[&source])
            .await
            .unwrap();
        client
            .query_one("SELECT pgmq.drop_queue($1)", &[&target])
            .await
            .unwrap();
    }
}
