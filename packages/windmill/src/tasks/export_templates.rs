// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit_log::{AdminAction, AdminChange, AuditActor};
use crate::services::task_audit;
use crate::types::error::Error;
use crate::{
    services::{
        export::export_template::process_export,
        tasks_execution::{update_complete, update_fail},
    },
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use serde_json::json;
use tracing::instrument;

/// The tenant log's entry of an export of the tenant's templates.
fn exported_entry(
    actor: Option<AuditActor>,
    tenant_id: &str,
    document_id: &str,
) -> anyhow::Result<AdminChange> {
    Ok(task_audit::bulk(
        actor,
        task_audit::target(tenant_id, None)?,
        task_audit::TEMPLATE,
        AdminAction::Export,
        "Exported the templates",
        json!({"document_id": document_id}),
    ))
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_templates(
    tenant_id: String,
    document_id: String,
    task_execution: TasksExecution,
    audit_actor: Option<AuditActor>,
) -> Result<()> {
    let res = process_export(&tenant_id, &document_id).await;
    if let Err(err) = res {
        let err_str = format!("Error process export templates: {}", err);
        update_fail(&task_execution, &err_str)
            .await
            .context("Failed to update task export templates to FAILED")?;
        return Err(Error::from(err));
    }
    let recorded = match exported_entry(audit_actor, &tenant_id, &document_id) {
        Ok(entry) => task_audit::record(entry).await,
        Err(err) => Err(err),
    };
    if let Err(err) = recorded {
        let err_str =
            format!("Templates exported, but the audit entry could not be queued: {err:#}");
        update_fail(&task_execution, &err_str)
            .await
            .context("Failed to update task export templates to FAILED")?;
        return Err(Error::String(err_str));
    }
    update_complete(&task_execution, None)
        .await
        .context("Failed to update task execution status to COMPLETED")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";

    #[test]
    fn an_export_is_one_entry_naming_its_document() {
        let entry = exported_entry(None, TENANT_ID, "document-1").unwrap();

        assert_eq!(entry.target.election_event_id, None);
        assert_eq!(entry.object.kind, "template");
        assert_eq!(entry.action, AdminAction::Export);
        assert_eq!(entry.describe(), "Exported the templates");
        assert_eq!(entry.extra, json!({"document_id": "document-1"}));
        assert!(exported_entry(None, "tenant", "document-1").is_err());
    }
}
