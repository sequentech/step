// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::results_event::get_results_event_by_id;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::services::database::get_hasura_pool;
use crate::services::tasks_execution::update_fail;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::serde::json::Json;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::info;
use tracing::instrument;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum EGenerateTemplate {
    BallotImages {
        election_event_id: String,
        election_id: String,
        tally_session_id: String,
    },
    VoteReceipts {
        election_event_id: String,
        election_id: String,
        tally_session_id: String,
    },
}

async fn generate_ballot_images(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    tally_session_id: &str,
) -> AnyhowResult<()> {
    let tally_session = get_tally_session_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;

    if !tally_session.is_execution_completed
        || tally_session.execution_status != Some(TallyExecutionStatus::SUCCESS.to_string())
    {
        return Err(anyhow!("Tally session is not completed"));
    }

    let last_tally_session_execution = get_last_tally_session_execution(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?
    .ok_or(anyhow!("No tally session execution found"))?;

    let results_event_id = last_tally_session_execution
        .results_event_id
        .ok_or(anyhow!("No results event found"))?;

    let results_event = get_results_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &results_event_id,
    )
    .await?;

    let original_tgz_document_id = results_event
        .documents
        .map(|document| document.tar_gz_original)
        .flatten()
        .ok_or(anyhow!("No tar gz document found"))?;

    Ok(())
}

async fn generate_vote_receipts(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    tally_session_id: &str,
) -> AnyhowResult<()> {
    Ok(())
}

#[instrument(err)]
async fn generate_template_block(
    tenant_id: String,
    document_id: String,
    input: EGenerateTemplate,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
) -> AnyhowResult<()> {
    let mut db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, "Failed to get Hasura DB pool").await;
            }
            return Err(anyhow!("Error getting Hasura DB pool: {}", err));
        }
    };

    let hasura_transaction = match db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, "Failed to get Hasura DB pool").await;
            };
            return Err(anyhow!("Error starting Hasura transaction: {err}"));
        }
    };

    match input {
        EGenerateTemplate::BallotImages {
            election_event_id,
            election_id,
            tally_session_id,
        } => {
            generate_ballot_images(
                &hasura_transaction,
                &tenant_id,
                &election_event_id,
                &election_id,
                &tally_session_id,
            )
            .await?;
        }
        EGenerateTemplate::VoteReceipts {
            election_event_id,
            election_id,
            tally_session_id,
        } => {
            generate_vote_receipts(
                &hasura_transaction,
                &tenant_id,
                &election_event_id,
                &election_id,
                &tally_session_id,
            )
            .await?;
        }
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn generate_template(
    tenant_id: String,
    document_id: String,
    input: EGenerateTemplate,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                generate_template_block(
                    tenant_id,
                    document_id,
                    input,
                    task_execution,
                    executer_username,
                )
                .await
                .map_err(|err| anyhow!("generate_report error: {:?}", err))
            })
        }
    });

    // Await the result and handle JoinError explicitly
    match handle.await {
        Ok(inner_result) => inner_result.map_err(|err| Error::from(err.context("Task failed"))),
        Err(join_error) => Err(Error::from(anyhow!("Task panicked: {}", join_error))),
    }?;

    Ok(())
}
