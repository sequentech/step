// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::export::export_verifiable_bulletin_board::{
    self, export_verifiable_bulletin_board_db_file,
};
use crate::services::protocol_manager::get_b3_pgsql_client;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error, Result as TaskResult};
use anyhow::{Context, Result};
use base64;
use celery::error::TaskError;
use chrono::{DateTime, Utc};
use csv::ReaderBuilder;
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::TryStreamExt;
use rusqlite::{params, Connection};
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tokio::task;
use tracing::{event, instrument, Level};

pub async fn process_export_verifiable_bulletin_board(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    document_id: String,
    tally_session_id: String,
    election_event_id: String,
) -> Result<()> {
    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Failed to get Tally Session: {err}"))?;

    let keys_ceremony = get_keys_ceremony_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session.keys_ceremony_id,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Failed to get Key Ceremony: {err}"))?;

    let (bulletin_board, election_id) = get_keys_ceremony_board(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Failed to get Key Ceremony Board: {err}"))?;

    export_verifiable_bulletin_board_db_file(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        election_id,
        Some(document_id.clone()),
        &bulletin_board,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Error exporting verifiable bulletin board: {err}"))?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_verifiable_bulletin_board_task(
    tenant_id: String,
    document_id: String,
    task_execution: TasksExecution,
    tally_session_id: String,
    election_event_id: String,
) -> TaskResult<()> {
    let result = provide_hasura_transaction(|hasura_transaction| {
        let document_copy = document_id.clone();
        Box::pin(async move {
            process_export_verifiable_bulletin_board(
                hasura_transaction,
                tenant_id,
                document_copy.clone(),
                tally_session_id,
                election_event_id,
            )
            .await
        })
    })
    .await;

    match result {
        Ok(_) => {
            let _res = update_complete(&task_execution, Some(document_id.clone())).await;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Error exporting verifiable bulletin board: {err:?}");
            let _res = update_fail(&task_execution, &err.to_string()).await;
            Err(err_str.into())
        }
    }
}
