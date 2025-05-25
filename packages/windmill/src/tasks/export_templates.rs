// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::error::Error;
use crate::{
    services::{
        export::export_template::process_export,
        tasks_execution::{update_complete, update_fail},
    },
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_templates(
    tenant_id: String,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let res = process_export(&tenant_id, &document_id).await;
    if let Err(err) = res {
        let err_str = format!("Error process export templates: {}", err);
        update_fail(&task_execution, &err_str)
            .await
            .context("Failed to update task export templates to FAILED")?;
        return Err(Error::from(err));
    }
    update_complete(&task_execution, None)
        .await
        .context("Failed to update task execution status to COMPLETED")?;
    Ok(())
}
