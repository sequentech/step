// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::electoral_log_audit::{log_to_tally, record_audit, run_electoral_log_audit};
use crate::services::tasks_execution::update_fail;
use crate::types::error::Result;
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

/// Audit an election event's electoral log and record the result on `task_execution`.
///
/// Started when a tally completes (with `tally_session_id`, so the result is also
/// reported in the tally logs) or on demand.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn audit_electoral_log(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: Option<String>,
    task_execution: TasksExecution,
) -> Result<()> {
    let result = run_electoral_log_audit(&tenant_id, &election_event_id).await;
    let summary = match record_audit(&task_execution, &result).await {
        Ok(summary) => summary,
        Err(error) => {
            let message = format!("Could not record the electoral-log audit: {error:#}");
            if let Err(update_error) = update_fail(&task_execution, &message).await {
                tracing::error!("Failed to mark the audit task as failed: {update_error:?}");
            }
            message
        }
    };
    if let Some(tally_session_id) = &tally_session_id {
        log_to_tally(&tenant_id, &election_event_id, tally_session_id, &summary).await;
    }
    result?;
    Ok(())
}
