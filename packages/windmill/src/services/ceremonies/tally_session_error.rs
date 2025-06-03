use crate::postgres::tally_session_execution::get_tally_session_executions;
use crate::postgres::tally_session_execution::insert_tally_session_execution;
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::types::ceremonies::Log;
use tracing::{event, info, instrument, Level};

use super::tally_ceremony::get_tally_ceremony_status;

#[instrument(err)]
pub async fn handle_tally_session_error(
    error: &str,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    let executions = get_tally_session_executions(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;

    let Some(last_execution) = executions.first() else {
        tracing::error!("No successful executions, skipping");
        return Ok(());
    };
    let mut status = get_tally_ceremony_status(last_execution.status.clone())?;

    let mut new_logs = status.logs.clone();
    let now = ISO8601::now();
    let new_log = Log {
        created_date: ISO8601::to_string(&now),
        log_text: error.to_string(),
    };
    let mut last_log = new_logs.pop().unwrap_or(new_log.clone());
    if last_log.log_text == error.to_string() {
        last_log.created_date = ISO8601::to_string(&now);
        new_logs.push(last_log);
    } else {
        new_logs.push(last_log);
        new_logs.push(new_log);
    }
    status.logs = new_logs;

    insert_tally_session_execution(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        last_execution.current_message_id,
        tally_session_id,
        Some(status),
        last_execution.results_event_id.clone(),
        last_execution.session_ids.clone(),
        None,
    )
    .await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}
