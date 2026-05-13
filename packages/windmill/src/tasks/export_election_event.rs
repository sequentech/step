// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Exports election event configuration and related entities.

use crate::services::database::get_hasura_pool;
use crate::services::export::export_election_event::process_export_zip;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error, Result};
use anyhow::Context;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

/// Slice toggles for which election artifacts are included in the ZIP export.
#[allow(clippy::struct_excessive_bools)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportOptions {
    /// Optional password when `is_encrypted` is true.
    pub password: Option<String>,
    /// When true, emit an encrypted archive instead of a plain ZIP.
    pub is_encrypted: bool,
    /// Include voter data in the export.
    pub include_voters: bool,
    /// Include activity logs in the export.
    pub activity_logs: bool,
    /// Include bulletin board data and proofs.
    pub bulletin_board: bool,
    /// Include ballot publication data.
    pub publications: bool,
    /// Include S3 file listings and small assets.
    pub s3_files: bool,
    /// Include scheduled events.
    pub scheduled_events: bool,
    /// Include generated reports and attachments.
    pub reports: bool,
    /// Include voter enrollment applications.
    pub applications: bool,
    /// Include encrypted tally data and results.
    pub tally: bool,
    /// Attach CA / TLS certificate material referenced by the event.
    pub include_certificates: bool,
}

mod export_election_event_task {
    #![allow(missing_docs)]
    #![allow(clippy::missing_docs_in_private_items)]

    use super::{
        event, get_hasura_pool, instrument, process_export_zip, update_complete, update_fail,
        Context, DbClient, Error, ExportOptions, Level, Result, TaskError, TasksExecution,
    };

    /// Celery task: export an election event as a ZIP.
    #[instrument(err)]
    #[wrap_map_err::wrap_map_err(TaskError)]
    #[celery::task(max_retries = 0)]
    pub async fn export_election_event(
        tenant_id: String,
        election_event_id: String,
        export_config: ExportOptions,
        document_id: String,
        task_execution: TasksExecution,
    ) -> Result<()> {
        let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
            Ok(client) => client,
            Err(err) => {
                let err_str = format!("Failed to get Hasura DB pool: {err:?}");
                if let Err(update_err) = update_fail(&task_execution, &err_str).await {
                    event!(
                        Level::ERROR,
                        "Failed to update task execution status to FAILED: {:?}",
                        update_err
                    );
                }
                return Err(Error::String(err_str));
            }
        };

        let hasura_transaction = match hasura_db_client.transaction().await {
            Ok(transaction) => transaction,
            Err(err) => {
                let err_str = format!("Failed to start Hasura transaction: {err:?}");
                if let Err(update_err) = update_fail(&task_execution, &err_str).await {
                    event!(
                        Level::ERROR,
                        "Failed to update task execution status to FAILED: {:?}",
                        update_err
                    );
                }
                return Err(Error::String(err_str));
            }
        };

        // Process the export
        match Box::pin(process_export_zip(
            &tenant_id,
            &election_event_id,
            &document_id,
            export_config,
        ))
        .await
        {
            Ok(()) => (),
            Err(err) => {
                let err_str = format!("Failed to export election event data: {err:?}");
                if let Err(update_err) = update_fail(&task_execution, &err_str).await {
                    event!(
                        Level::ERROR,
                        "Failed to update task execution status to FAILED: {:?}",
                        update_err
                    );
                }
                return Err(Error::String(err_str));
            }
        }

        match hasura_transaction.commit().await {
            Ok(()) => (),
            Err(err) => {
                let err_str = format!("Commit failed: {err:?}");
                if let Err(update_err) = update_fail(&task_execution, &err_str).await {
                    event!(
                        Level::ERROR,
                        "Failed to update task execution status to FAILED: {:?}",
                        update_err
                    );
                }
                return Err(Error::String(err_str));
            }
        }

        update_complete(&task_execution, Some(document_id.clone()))
            .await
            .context("Failed to update task execution status to COMPLETED")?;

        Ok(())
    }
}

pub use export_election_event_task::export_election_event;
