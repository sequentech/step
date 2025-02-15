// SPDX-FileCopyrightText: 2025 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::types::error::{Error, Result};
use anyhow::Context;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use tracing::instrument;

const EVENT_TYPE_COMMUNICATIONS: &str = "communications";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEventInput {
    election_event_id: String,
    message_type: String,
    user_id: Option<String>,
    username: Option<String>,
    tenant_id: String,
    body: String,
}

// Inserts a new event in the IMMUDB electoral log
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn insert_electoral_log_event(input: LogEventInput) -> Result<()> {
    // 1) Fetch database client
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting DB pool")?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting transaction")?;

    // 2) Query the election event
    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &input.tenant_id,
        &input.election_event_id,
    )
    .await
    .with_context(|| "Error getting election event")?;

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "error getting election event board")?;

    // 3) Create or reuse the ElectoralLog
    let user_id = input
        .user_id
        .clone()
        .unwrap_or_else(|| "unknown_user".into());
    let username = input.username.clone();
    let tenant_id = input.tenant_id.clone();

    let electoral_log = ElectoralLog::for_admin_user(&board_name, &tenant_id, &user_id)
        .await
        .with_context(|| "error getting electoral log")?;

    // 4) Insert the main event into Immudb
    electoral_log
        .post_keycloak_event(
            input.election_event_id.clone(),
            input.message_type.clone(),
            input.body.clone(),
            Some(user_id.clone()),
            username.clone(),
        )
        .await
        .with_context(|| "error posting keycloak event")?;

    // 5) If communications event, do additional call
    let event_type_communications = "communications";
    if input.body.contains(event_type_communications) {
        let body = input
            .body
            .replace(event_type_communications, "")
            .trim()
            .to_string();

        electoral_log
            .post_send_template(
                Some(body),
                input.election_event_id.clone(),
                Some(user_id.clone()),
                username.clone(),
                None,
            )
            .await
            .with_context(|| "error posting communications")?;
    }

    Ok(())
}
