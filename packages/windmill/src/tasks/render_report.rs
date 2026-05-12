// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Renders report HTML/PDF.
use crate::postgres::render_report::render_report_task;
use crate::services::database::get_hasura_pool;
use crate::services::tasks_semaphore::acquire_semaphore;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::instrument;

/// Format type for the rendered report.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum FormatType {
    /// Text or HTML.
    TEXT,
    /// PDF.
    PDF,
}

/// Body sent to the render task.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RenderTemplateBody {
    /// Mustache-style template.
    pub template: String,
    /// Document name.
    pub name: String,
    /// Variables merged into the template context.
    pub variables: Map<String, Value>,
    /// Format type for the rendered report.
    pub format: FormatType,
}

mod render_report_celery {
    #![allow(missing_docs)]
    #![allow(clippy::missing_docs_in_private_items)]

    use super::{
        acquire_semaphore, get_hasura_pool, instrument, render_report_task, DbClient,
        RenderTemplateBody, Result, TaskError,
    };

    /// Celery task: renders `input` inside a blocking Hasura transaction using the shared Postgres render helper.
    ///
    /// # Errors
    ///
    /// Fails on semaphore acquisition, DB pool/transaction errors, render failures, commit errors, or join panics.
    #[instrument(err)]
    #[wrap_map_err::wrap_map_err(TaskError)]
    #[celery::task(time_limit = 60000)]
    pub async fn render_report(
        input: RenderTemplateBody,
        tenant_id: String,
        election_event_id: String,
    ) -> Result<()> {
        let _permit = acquire_semaphore().await?;
        // Spawn the task using an async block
        let handle = tokio::task::spawn_blocking({
            move || {
                tokio::runtime::Handle::current().block_on(async move {
                    let mut db_client: DbClient = get_hasura_pool()
                        .await
                        .get()
                        .await
                        .map_err(|err| format!("Error getting DB pool: {err:?}"))?;

                    let hasura_transaction = match db_client.transaction().await {
                        Ok(transaction) => transaction,
                        Err(err) => {
                            return Err(format!("Error starting Hasura transaction: {err}"));
                        }
                    };

                    render_report_task(&hasura_transaction, input, tenant_id, election_event_id)
                        .await
                        .map_err(|err| format!("{err}"))?;

                    match hasura_transaction.commit().await {
                        Ok(()) => (),
                        Err(err) => {
                            return Err(format!("Commit failed: {err}"));
                        }
                    }
                    Ok(())
                })
            }
        });

        // Await the result and handle JoinError explicitly
        match handle.await {
            Ok(inner_result) => Ok(inner_result.map_err(|err| format!("Task failed: {err:?}"))?),
            Err(join_error) => Err(format!("Join error. Task panicked: {join_error:?}")),
        }?;

        Ok(())
    }
}

pub use render_report_celery::render_report;
