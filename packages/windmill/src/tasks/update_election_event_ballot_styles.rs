// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

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
            update_complete(&task_execution, None).await.ok();
            Ok(())
        }
        Err(error) => {
            update_fail(&task_execution, &error.to_string()).await.ok();
            Err(Error::Anyhow(error))
        }
    }
}
