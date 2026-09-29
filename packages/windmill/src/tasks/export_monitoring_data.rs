// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Exports what a monitoring dashboard showed, as CSV or SQL, and hands the
//! file over as a document.

use crate::postgres::monitoring_config::EventRef;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::monitoring::export::{
    build_file, collect_export, file_name, MonitoringExportError, MonitoringExportRequest,
};
use crate::services::monitoring::snapshot::request_election_set;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::services::tasks_semaphore::acquire_semaphore;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::temp_path::write_into_named_temp_file;
use tokio_postgres::IsolationLevel;
use tracing::{instrument, warn};
use uuid::Uuid;

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

#[instrument(err, skip(task_execution))]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, time_limit = 600)]
pub async fn export_monitoring_data(
    request: MonitoringExportRequest,
    task_execution: TasksExecution,
) -> Result<()> {
    match export_monitoring_data_impl(&request).await {
        Ok(()) => {
            update_complete(&task_execution, Some(request.document_id.clone()))
                .await
                .context("Failed to update task execution status to COMPLETED")?;
            Ok(())
        }
        Err(error) => {
            if let Err(update_error) = update_fail(&task_execution, &error.to_string()).await {
                tracing::error!(
                    "Failed to update task execution status to FAILED: {:?}",
                    update_error
                );
            }
            Err(anyhow!(error).into())
        }
    }
}
