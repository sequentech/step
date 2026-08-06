// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{error, instrument};

use crate::services::ballot_styles::ballot_style;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error, Result};

#[instrument(skip(task_execution), err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn update_election_event_ballot_styles(
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    match ballot_style::update_election_event_ballot_styles(
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await
    {
        Ok(()) => {
            // The Publish screen polls this record, so a dropped status
            // update leaves the task pending indefinitely. The generation
            // itself succeeded and is still reported as such; surface the
            // bookkeeping failure so it is alertable rather than invisible.
            if let Err(status_error) = update_complete(&task_execution, None).await {
                error!(
                    task_id = %task_execution.id,
                    "Ballot styles were generated but the task execution could not be marked complete: {status_error:?}"
                );
            }
            Ok(())
        }
        Err(error) => {
            if let Err(status_error) = update_fail(&task_execution, &error.to_string()).await {
                error!(
                    task_id = %task_execution.id,
                    "Ballot style generation failed and the task execution could not be marked failed: {status_error:?}"
                );
            }
            Err(Error::Anyhow(error))
        }
    }
}
