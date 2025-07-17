// SPDX-FileCopyrightText: 2025 Eduardo Robles <edu@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::celery_app::get_celery_connection;
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
use tracing::{event, info, instrument};

use lapin::{
    options::{BasicAckOptions, BasicGetOptions, QueueDeclareOptions},
    types::FieldTable,
};

const EVENT_TYPE_COMMUNICATIONS: &str = "communications";
pub const INTERNAL_MESSAGE_TYPE: &str = "internal";

/// Represents an incoming log event.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEventInput {
    pub election_event_id: String,
    pub message_type: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub tenant_id: String,
    pub body: String,
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

        let user_id = input
            .user_id
            .clone()
            .unwrap_or_else(|| "unknown_user".into());
        let username = input.username.clone();
        let tenant_id = input.tenant_id.clone();
        let realm = get_event_realm(&input.tenant_id, &input.election_event_id);

        let user_area_id = get_user_area_id(&keycloak_transaction, &realm, &user_id)
            .await
            .with_context(|| "Error getting user area id")?;

        let electoral_log = ElectoralLog::for_admin_user(
            &hasura_tx,
            &board_name,
            &tenant_id,
            &election_event.id,
            &user_id,
            username.clone(),
            None,
            user_area_id.clone(),
        )
        .await
        .with_context(|| "Error initializing electoral log")?;

        let event_message = match input.message_type.as_str() {
            INTERNAL_MESSAGE_TYPE => {
                let message: ElectoralLogMessage = deserialize_str(&input.body)
                    .with_context(|| "Error parsing input.body into a ElectoralLogMessage")?;
                message
            }
            _ => {
                if input.body.contains(EVENT_TYPE_COMMUNICATIONS) {
                    let template_body = input
                        .body
                        .replace(EVENT_TYPE_COMMUNICATIONS, "")
                        .trim()
                        .to_string();
                    let send_template_msg = electoral_log
                        .build_send_template_message(
                            Some(template_body),
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
                        input.message_type.clone(),
                        input.body.clone(),
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

/// Dispatcher: repeatedly reads batches of messages from the electoral_log_batch_queue and dispatches them
/// to the processing task. Each batch is processed sequentially so that only a single batch is held in memory.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 30, max_retries = 0, expires = 1)]
pub async fn electoral_log_batch_dispatcher() -> Result<()> {
    info!("starting electoral_log_batch_dispatcher");

    // Reuse the global AMQP connection.
    let connection_arc = get_celery_connection().await?;
    let channel = connection_arc
        .create_channel()
        .await
        .with_context(|| "Error creating RabbitMQ channel")?;

    let queue_name = Queue::ElectoralLogEvent.as_ref();
    let _queue = channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .with_context(|| "Error declaring electoral_log_batch_queue")?;

    // Get the batch size from PgConfig.
    let batch_size: usize = PgConfig::from_env()?.default_sql_batch_size.try_into()?;

    loop {
        info!("starting a new batch for queue {queue_name}, max batch_size={batch_size}");
        let mut batch_deliveries = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            if let Some(delivery) = channel
                .basic_get(queue_name, BasicGetOptions { no_ack: false })
                .await?
            {
                info!("adding delivery element to batch_deliveries");
                batch_deliveries.push(delivery);
            } else {
                info!("not adding to batch_deliveries, break");
                break;
            }
        }

        if batch_deliveries.is_empty() {
            info!("no more elements to process in queue");
            break;
        }
        info!(
            "deserializing {len} elements for this batch",
            len = batch_deliveries.len()
        );

        // Deserialize messages sequentially.
        let mut events = Vec::with_capacity(batch_deliveries.len());
        for delivery in &batch_deliveries {
            // Parse the raw message into a JSON value.
            let v: serde_json::Value = serde_json::from_slice(&delivery.data)
                .with_context(|| "Error parsing Celery message as JSON")?;
            // Expect the message to be an array.
            if let serde_json::Value::Array(arr) = v {
                if arr.len() < 2 {
                    return Err(
                        "Invalid message format: expected array with at least 2 elements".into(),
                    );
                }
                let payload = &arr[1];
                let input_value = payload
                    .get("input")
                    .ok_or_else(|| anyhow!("Missing 'input' field in message payload"))?;
                let event: LogEventInput = serde_json::from_value(input_value.clone())
                    .with_context(|| "Error deserializing LogEventInput from input field")?;
                events.push(event);
            } else {
                return Err("Invalid message format: expected JSON array".into());
            }
        }

        // Dispatch the processing task via the Celery app.
        let celery_app = crate::services::celery_app::get_celery_app().await;
        let celery_task = process_electoral_log_events_batch::new(events);
        info!("sending processing task for current batch");
        celery_app
            .send_task(celery_task)
            .await
            .with_context(|| "Error sending process_electoral_log_events_batch task")?;

        // Acknowledge all messages in the current batch.
        for delivery in batch_deliveries {
            channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .await
                .with_context(|| "Error acknowledging message")?;
        }
    }
    info!("finishing electoral_log_batch_dispatcher");
    Ok(())
}
