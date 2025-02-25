// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::import::import_tenant_config::import_tenant_config_zip;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportOptions {
    pub include_tenant: Option<bool>,
    pub include_keycloak: Option<bool>,
    pub include_roles: Option<bool>,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_tenant_config(
    object: ImportOptions,
    tenant_id: String,
    document_id: String,
    sha256: Option<String>,
    task_execution: TasksExecution,
) -> Result<()> {
    let task_execution_clone = task_execution.clone();

    let object = object.clone();
    let tenant_id = tenant_id.clone();
    let task_execution = task_execution_clone.clone();

    match import_tenant_config_zip(object, &tenant_id, &document_id, sha256).await {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, &err.to_string()).await?;
            return Err(anyhow!("Error process tenant configuration documents: {:?}", err).into());
        }
    };

    update_complete(&task_execution, Some(document_id.to_string()))
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}
