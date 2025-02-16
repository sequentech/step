// SPDX-FileCopyrightText: 2025 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_board_client;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use celery::prelude::Task;
use deadpool_postgres::Client as DbClient;
use electoral_log::client::board_client::ElectoralLogMessage;
use immudb_rs::TxMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;

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
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn enqueue_electoral_log_event(input: LogEventInput) -> Result<()> {
    // By virtue of being a Celery task, the event is enqueued.
    Ok(())
}

/// Process a batch of electoral log events using a single Hasura transaction
/// for per‑event work and then grouping messages by election event (board) so
/// that each board's messages can be inserted in one immudb transaction.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_electoral_log_events_batch(events: Vec<LogEventInput>) -> Result<()> {
    // Group immudb messages by board name.
    let mut messages_by_board: HashMap<String, Vec<ElectoralLogMessage>> = HashMap::new();

    // Begin a Hasura transaction to process election events.
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting DB pool for batch processing")?;
    let hasura_tx = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting Hasura transaction")?;

    // Process each incoming event.
    for input in events.iter() {
        // 1) Query the election event from Hasura.
        let election_event =
            get_election_event_by_id(&hasura_tx, &input.tenant_id, &input.election_event_id)
                .await
                .with_context(|| "Error getting election event")?;

        // 2) Determine the board (database) name.
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "Error getting election event board")?;

        let user_id = input
            .user_id
            .clone()
            .unwrap_or_else(|| "unknown_user".into());
        let username = input.username.clone();
        let tenant_id = input.tenant_id.clone();

        // 3) Create an ElectoralLog instance for the admin user.
        let electoral_log = ElectoralLog::for_admin_user(&board_name, &tenant_id, &user_id)
            .await
            .with_context(|| "Error initializing electoral log")?;

        // 4) Build the keycloak event message.
        let keycloak_msg = electoral_log
            .build_keycloak_event_message(
                input.election_event_id.clone(),
                input.message_type.clone(),
                input.body.clone(),
                Some(user_id.clone()),
                username.clone(),
            )
            .with_context(|| "Error building keycloak event message")?;

        messages_by_board
            .entry(board_name.clone())
            .or_insert_with(Vec::new)
            .push(keycloak_msg);

        // 5) If this is a communications event, build the send-template message.
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
                )
                .with_context(|| "Error building send template message")?;
            messages_by_board
                .entry(board_name.clone())
                .or_insert_with(Vec::new)
                .push(send_template_msg);
        }
    }

    // For each board group, open an immudb session and insert all messages in a
    // single transaction.
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
