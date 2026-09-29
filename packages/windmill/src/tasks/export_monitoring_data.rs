// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Exports what a monitoring dashboard showed, as CSV or SQL, and hands the
//! file over as a document.
//!
//! Like the other exports it is acknowledged only once it ran, so a worker
//! lost midway has it delivered again. A delivery takes the task's lock and
//! reloads the task first: one that already completed, or whose document is
//! already written, is not exported twice, and the status only ever moves
//! from IN_PROGRESS. The export runs within a budget shorter than the
//! task's time limit, so running out of time is recorded as a failure
//! rather than leaving the task in progress.

use crate::postgres::document::get_document;
use crate::postgres::monitoring_config::EventRef;
use crate::postgres::tasks_execution::{
    get_task_by_id_with_transaction, lock_export_task_with_transaction,
};
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::monitoring::export::{
    build_file, collect_export, file_name, MonitoringExportError, MonitoringExportRequest,
};
use crate::services::monitoring::snapshot::request_election_set;
use crate::services::tasks_execution::{
    is_matching_completed_export_task, update_export_complete, update_export_fail,
};
use crate::services::tasks_semaphore::acquire_semaphore;
use crate::types::error::Result;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::temp_path::write_into_named_temp_file;
use std::future::Future;
use std::time::Duration;
use tokio_postgres::IsolationLevel;
use tracing::{error, instrument, warn};
use uuid::Uuid;

/// The task's time limit, in seconds.
const TIME_LIMIT: u32 = 600;

/// How long the export itself may take: less than the time limit, so a
/// failure can still be recorded before the worker stops the task.
const BUDGET: Duration = Duration::from_secs(570);

async fn export_monitoring_data_impl(
    request: &MonitoringExportRequest,
) -> std::result::Result<(), MonitoringExportError> {
    let _permit = acquire_semaphore()
        .await
        .map_err(|error| anyhow!("{error:?}"))?;
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting DB pool")?;
    let collected = {
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::RepeatableRead)
            .read_only(true)
            .start()
            .await
            .context("Error starting the read transaction")?;
        collect_export(&transaction, request).await
    };
    let data = match collected {
        Err(MonitoringExportError::ScopePending { widget_id }) => {
            // Ask the next pass to count the viewer's elections, as a
            // render would.
            if let Err(error) = record_set(&mut client, request).await {
                warn!("Could not record the set of elections to count: {error:?}");
            }
            return Err(MonitoringExportError::ScopePending { widget_id });
        }
        other => other?,
    };
    let bytes = build_file(&data, request.format)?;
    let name = file_name(&data, request.format);
    let (_temp_path, path, size) = write_into_named_temp_file(
        &bytes,
        "monitoring-export-",
        &format!(".{}", request.format.extension()),
    )
    .context("Error writing the export file")?;
    let transaction = client
        .transaction()
        .await
        .context("Error starting transaction")?;
    upload_and_return_document(
        &transaction,
        &path,
        size,
        request.format.media_type(),
        &request.tenant_id,
        Some(request.election_event_id.clone()),
        &name,
        Some(request.document_id.clone()),
        false,
    )
    .await
    .context("Error uploading the export file")?;
    transaction
        .commit()
        .await
        .context("Failed to commit the document")?;
    Ok(())
}

async fn record_set(
    client: &mut deadpool_postgres::Client,
    request: &MonitoringExportRequest,
) -> anyhow::Result<()> {
    let event = EventRef {
        tenant_id: Uuid::parse_str(&request.tenant_id)?,
        election_event_id: Uuid::parse_str(&request.election_event_id)?,
    };
    let ids = request
        .election_ids
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<std::result::Result<Vec<Uuid>, _>>()?;
    let transaction = client.transaction().await?;
    request_election_set(&transaction, event, &ids).await?;
    transaction.commit().await?;
    Ok(())
}

/// Runs `export` for at most `budget`.
pub async fn within_budget<F>(
    budget: Duration,
    export: F,
) -> std::result::Result<(), MonitoringExportError>
where
    F: Future<Output = std::result::Result<(), MonitoringExportError>>,
{
    match tokio::time::timeout(budget, export).await {
        Ok(outcome) => outcome,
        Err(_) => Err(MonitoringExportError::TimedOut {
            seconds: budget.as_secs(),
        }),
    }
}

/// Records how the export ended. A task that cannot be marked complete is
/// marked failed rather than left in progress.
async fn record_outcome(
    task: &TasksExecution,
    request: &MonitoringExportRequest,
    outcome: std::result::Result<(), MonitoringExportError>,
) -> Result<()> {
    match outcome {
        Ok(()) => {
            if let Err(complete_error) =
                update_export_complete(task, request.document_id.clone()).await
            {
                error!("Failed to mark the monitoring export complete: {complete_error:?}");
                let message =
                    format!("INTERNAL: the export could not be recorded: {complete_error}");
                if let Err(fail_error) = update_export_fail(task, &message).await {
                    error!("Failed to mark the monitoring export failed: {fail_error:?}");
                }
                return Err(complete_error
                    .context("Failed to update task execution status to COMPLETED")
                    .into());
            }
            Ok(())
        }
        Err(error) => {
            if let Err(update_error) = update_export_fail(task, &error.to_string()).await {
                error!("Failed to update task execution status to FAILED: {update_error:?}");
            }
            Err(anyhow!(error).into())
        }
    }
}

#[instrument(err, skip(task_execution))]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, acks_late = true, time_limit = 600)]
pub async fn export_monitoring_data(
    request: MonitoringExportRequest,
    task_execution: TasksExecution,
) -> Result<()> {
    let mut lock_client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting DB pool")?;
    // Held until the export is recorded: a second delivery waits, then sees
    // the outcome.
    let lock = lock_client
        .transaction()
        .await
        .context("Error starting the lock transaction")?;
    lock_export_task_with_transaction(&lock, &task_execution.id)
        .await
        .context("Failed to lock the monitoring export task")?;
    let task = get_task_by_id_with_transaction(&lock, &request.tenant_id, &task_execution.id)
        .await
        .context("Failed to load the monitoring export task")?;
    if is_matching_completed_export_task(
        &task,
        &request.tenant_id,
        &request.election_event_id,
        &ETasksExecution::EXPORT_MONITORING_DATA,
        &request.document_id,
    ) {
        return Ok(());
    }
    let written = get_document(
        &lock,
        &request.tenant_id,
        Some(request.election_event_id.clone()),
        &request.document_id,
    )
    .await
    .context("Failed to look for the export's document")?
    .is_some();
    let outcome = if written {
        // An earlier delivery wrote it, and stopped before recording it.
        Ok(())
    } else {
        within_budget(BUDGET, export_monitoring_data_impl(&request)).await
    };
    let recorded = record_outcome(&task, &request, outcome).await;
    lock.commit()
        .await
        .context("Failed to release the monitoring export task lock")?;
    recorded
}

#[cfg(test)]
mod tests {
    use super::*;
    use celery::task::Task;

    #[test]
    fn monitoring_exports_ack_only_after_execution_and_stop_before_the_time_limit() {
        assert_eq!(export_monitoring_data::DEFAULTS.acks_late, Some(true));
        assert_eq!(
            export_monitoring_data::DEFAULTS.time_limit,
            Some(TIME_LIMIT)
        );
        assert!(BUDGET.as_secs() + 10 <= u64::from(TIME_LIMIT));
    }

    #[tokio::test]
    async fn an_export_out_of_time_fails_as_timed_out() {
        let slow = async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok(())
        };
        let outcome = within_budget(Duration::from_millis(10), slow).await;
        assert!(
            matches!(outcome, Err(MonitoringExportError::TimedOut { .. })),
            "{outcome:?}"
        );
        assert!(within_budget(Duration::from_secs(5), async { Ok(()) })
            .await
            .is_ok());
    }
}
