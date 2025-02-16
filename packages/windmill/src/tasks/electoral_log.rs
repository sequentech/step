// SPDX-FileCopyrightText: 2025 Eduardo Robles <edu@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log::ElectoralLog;
use crate::services::election_event_board::get_election_event_board;
use crate::types::error::{Error, Result};
use rayon::prelude::*;
use electoral_log::client::board_client::ElectoralLogMessage;
use anyhow::{Context, anyhow};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use immudb_rs::TxMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;
use crate::services::protocol_manager::get_board_client;
use crate::services::database::PgConfig;

// Use the typed queue name from celery_app.
use crate::services::celery_app::Queue;

// Use the helper to reuse the AMQP connection.
use crate::services::celery_app::get_celery_connection;

use lapin::{
    Connection,
    options::{BasicGetOptions, BasicAckOptions},
    types::FieldTable,
};

const EVENT_TYPE_COMMUNICATIONS: &str = "communications";

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

    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await
        .with_context(|| "Error getting DB pool for batch processing")?;
    let hasura_tx = hasura_db_client.transaction().await
        .with_context(|| "Error starting Hasura transaction")?;

    for input in events.iter() {
        let election_event = get_election_event_by_id(
            &hasura_tx,
            &input.tenant_id,
            &input.election_event_id,
        ).await.with_context(|| "Error getting election event")?;
        
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "Error getting election event board")?;
        
        let user_id = input.user_id.clone().unwrap_or_else(|| "unknown_user".into());
        let username = input.username.clone();
        let tenant_id = input.tenant_id.clone();
        
        let electoral_log = ElectoralLog::for_admin_user(&board_name, &tenant_id, &user_id)
            .await.with_context(|| "Error initializing electoral log")?;
        
        let keycloak_msg = electoral_log.build_keycloak_event_message(
            input.election_event_id.clone(),
            input.message_type.clone(),
            input.body.clone(),
            Some(user_id.clone()),
            username.clone(),
        ).with_context(|| "Error building keycloak event message")?;
        
        messages_by_board
            .entry(board_name.clone())
            .or_insert_with(Vec::new)
            .push(keycloak_msg);
        
        if input.body.contains(EVENT_TYPE_COMMUNICATIONS) {
            let template_body = input.body.replace(EVENT_TYPE_COMMUNICATIONS, "").trim().to_string();
            let send_template_msg = electoral_log.build_send_template_message(
                Some(template_body),
                input.election_event_id.clone(),
                Some(user_id.clone()),
                username.clone(),
                None,
            ).with_context(|| "Error building send template message")?;
            messages_by_board
                .entry(board_name.clone())
                .or_insert_with(Vec::new)
                .push(send_template_msg);
        }
    }
    
    hasura_tx.commit().await.with_context(|| "Error committing Hasura transaction")?;
    
    for (board, messages) in messages_by_board.into_iter() {
        let mut board_client = get_board_client().await?;
        board_client.open_session(&board).await?;
        let immudb_tx = board_client.new_tx(TxMode::ReadWrite).await?;
        board_client.insert_electoral_log_messages_batch(&immudb_tx, &messages).await
            .with_context(|| format!("Error inserting batch electoral log messages for board {}", board))?;
        board_client.commit(&immudb_tx).await
            .with_context(|| format!("Error committing immudb transaction for board {}", board))?;
        board_client.close_session().await?;
    }
    
    Ok(())
}

/// Dispatcher: reads all pending messages from the electoral_log_batch_queue and dispatches them
/// to the processing task. This function reuses the global AMQP connection from the Celery app.
/// Note: This task does not have a schedule in its attribute; scheduling is defined in beat.rs.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(crate::types::error::Error)]
#[celery::task(max_retries = 0)]
pub async fn electoral_log_batch_dispatcher() -> Result<()> {
    // Reuse the global AMQP connection.
    let connection_arc = get_celery_connection().await?;
    // connection_arc is an Arc<Connection>; get a channel from its inner value.
    let channel = connection_arc.create_channel().await
        .with_context(|| "Error creating RabbitMQ channel")?;
    
    let queue_name = crate::services::celery_app::Queue::ElectoralLogBatch.as_ref();
    let _queue = channel.queue_declare(
        queue_name,
        QueueDeclareOptions { durable: true, ..Default::default() },
        FieldTable::default(),
    ).await.with_context(|| "Error declaring electoral_log_batch_queue")?;
    
    // Use PgConfig for the batch size.
    let batch_size: usize = PgConfig::from_env()?.default_sql_batch_size.into();
    
    let mut deliveries = Vec::new();
    loop {
        if let Some(delivery) = channel.basic_get(queue_name, BasicGetOptions { no_ack: false }).await? {
            deliveries.push(delivery);
        } else {
            break;
        }
    }
    
    if deliveries.is_empty() {
        return Ok(());
    }
    
    // Deserialize messages concurrently using Rayon.
    use rayon::prelude::*;
    let events: Vec<LogEventInput> = deliveries.par_iter()
        .map(|delivery| {
            serde_json::from_slice::<LogEventInput>(&delivery.data)
                .map_err(|e| anyhow!("Error deserializing LogEventInput: {:?}", e))
        })
        .collect::<Result<Vec<_>>>()?;
    
    let delivery_tags: Vec<u64> = deliveries.into_iter().map(|d| d.delivery_tag).collect();
    
    // Dispatch the processing task via the Celery app.
    let celery_app = crate::services::celery_app::get_celery_app().await;
    let celery_task = process_electoral_log_events_batch::new(events);
    let _celery_task_result = celery_app.send_task(celery_task).await
        .with_context(|| "Error sending process_electoral_log_events_batch task")?;
    
    for tag in delivery_tags {
        channel.basic_ack(tag, BasicAckOptions::default()).await
            .with_context(|| "Error acknowledging message")?;
    }
    
    Ok(())
}