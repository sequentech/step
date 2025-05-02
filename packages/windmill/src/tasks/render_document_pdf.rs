// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000)]
pub async fn render_document_pdf(
    tenant_id: String,
    document_id: String,
    election_event_id: Option<String>,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
) -> Result<()> {
    Ok(())
}
