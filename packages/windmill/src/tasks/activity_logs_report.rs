// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::reports::electoral_log::{generate_report, ReportFormat},
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{event, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn generate_activity_logs_report(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    format: ReportFormat,
    task_execution: TasksExecution,
) -> Result<()> {
    let _data = generate_report(
        &tenant_id,
        &election_event_id,
        &document_id,
        format,
        task_execution.clone(),
    )
    .await?;

    Ok(())
}
