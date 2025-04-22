// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::{export::export_application::process_export, tasks_execution::update_fail},
    types::error::{Error, Result},
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{event, info, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_application(
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    // Process the export
    match process_export(&tenant_id, &election_event_id, election_id, &document_id).await {
        Ok(_) => (),
        Err(err) => {
            let err_str = format!("Error sending export_application task: {err:?}");
            update_fail(&task_execution, &err_str).await;
            return Err(Error::String(err_str));
        }
    }

    Ok(())
}
