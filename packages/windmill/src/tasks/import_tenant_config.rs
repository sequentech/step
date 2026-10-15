// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit_log::{AdminAction, AuditActor};
use crate::services::import::import_tenant_config::import_tenant_config_zip;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::task_audit;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::{
    services::import::import_election_event::{self as import_election_event_service},
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    audit_actor: Option<AuditActor>,
) -> Result<()> {
    let task_execution_clone = task_execution.clone();

    let object = object.clone();
    let tenant_id = tenant_id.clone();
    let task_execution = task_execution_clone.clone();

    let mut extra = task_audit::source_document(&document_id, sha256.as_deref());
    extra["included"] = json!({
        "tenant": object.include_tenant.unwrap_or(false),
        "keycloak": object.include_keycloak.unwrap_or(false),
        "roles": object.include_roles.unwrap_or(false),
    });

    match import_tenant_config_zip(object, &tenant_id, &document_id, sha256).await {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, &err.to_string()).await?;
            return Err(anyhow!("Error process tenant configuration documents: {:?}", err).into());
        }
    };

    let recorded =
        match task_audit::tenant_config(audit_actor, &tenant_id, AdminAction::Import, extra) {
            Ok(entry) => task_audit::record(entry).await,
            Err(err) => Err(err),
        };
    if let Err(err) = recorded {
        let err_str = format!(
            "Tenant configuration imported, but the audit entry could not be queued: {err:#}"
        );
        update_fail(&task_execution, &err_str).await?;
        return Err(anyhow!(err_str).into());
    }

    update_complete(&task_execution, Some(document_id.to_string()))
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}
