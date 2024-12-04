// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::{
    services::import::import_election_event::{self as import_election_event_service},
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tracing::{event, info, instrument, Level};

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportElectionEventBody {
    pub tenant_id: String,
    pub document_id: String,
    pub password: Option<String>,
    pub check_only: Option<bool>,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_election_event(
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let task_execution_clone = task_execution.clone();

    let result = provide_hasura_transaction(|hasura_transaction| {
        let object = object.clone();
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let task_execution = task_execution_clone.clone();

        Box::pin(async move {
            match import_election_event_service::process_document(
                hasura_transaction,
                object,
                election_event_id,
                tenant_id,
            )
            .await
            {
                Ok(_) => Ok(()),
                Err(err) => {
                    update_fail(&task_execution, &format!("{:?}", err)).await?;
                    Err(anyhow!("Error process election event document: {:?}", err))
                }
            }
        })
    })
    .await;
    match result {
        Ok(_) => {
            update_complete(&task_execution)
                .await
                .context("Failed to update task execution status to COMPLETED")?;
        }
        Err(error) => {
            update_fail(&task_execution, &format!("{}", error)).await?;
        }
    }

    Ok(())
}
