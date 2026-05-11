// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Imports tenant configuration ZIP bundle.
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

/// Options for which tenant configuration slices are applied from the ZIP.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportOptions {
    /// Restore tenant data.
    pub include_tenant: Option<bool>,
    /// Restore Keycloak realm data.
    pub include_keycloak: Option<bool>,
    /// Restore roles and permissions data.
    pub include_roles: Option<bool>,
}

mod import_tenant_config_task {
    #![allow(missing_docs)]
    #![allow(clippy::missing_docs_in_private_items)]

    use super::*;

    /// Celery task: unpack a tenant configuration archive into Hasura and related services.
    #[instrument(err)]
    #[wrap_map_err::wrap_map_err(TaskError)]
    #[celery::task]
    pub async fn import_tenant_config(
        object: super::ImportOptions,
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
            Ok(()) => (),
            Err(err) => {
                update_fail(&task_execution, &err.to_string()).await?;
                return Err(
                    anyhow!("Error process tenant configuration documents: {err:?}").into(),
                );
            }
        };

        update_complete(&task_execution, Some(document_id.to_string()))
            .await
            .context("Failed to update task execution status to COMPLETED")?;

        Ok(())
    }
}

pub use import_tenant_config_task::import_tenant_config;
