// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::error::{Error, Result};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportVotersDelegationInput {
    pub tenant_id: String,
    pub document_id: String,
    pub election_event_id: String,
    pub sha256: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportVotersDelegationOutput {
    pub task_execution: TasksExecution,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn import_voters_delegation_task(
    body: ImportVotersDelegationInput,
    task_execution: TasksExecution,
) -> Result<()> {
    Ok(())
}
