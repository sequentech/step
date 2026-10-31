// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::count_tenant_election_events;
use crate::postgres::tenant::delete_tenant as delete_tenant_postgres;
use crate::services::audit_log::AuditActor;
use crate::services::delete_tenant::{delete_tenant_related_data, stage_tenant_deletion};
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use crate::{
    services::providers::transactions_provider::provide_hasura_transaction, types::error::Result,
};
use anyhow::{anyhow, Result as AnyhowResult};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

/// Commit tenant deletion, with its entry in the log of the tenant that
/// queued the task, before removing its external realm and objects.
#[instrument(err, skip(task_execution))]
async fn delete_tenant(
    tenant_id: String,
    realm: String,
    task_execution: &TasksExecution,
    audit_actor: Option<AuditActor>,
) -> AnyhowResult<()> {
    let tenant_id_cloned = tenant_id.clone();
    let log_tenant_id = task_execution.tenant_id.clone();
    let task_id = task_execution.id.clone();

    provide_hasura_transaction(|hasura_transaction| {
        Box::pin(async move {
            let existing_events =
                count_tenant_election_events(&hasura_transaction, &tenant_id_cloned).await?;
            if existing_events > 0 {
                return Err(anyhow!(
                    "cannot delete tenant {tenant_id_cloned}: {existing_events} election event(s) still exist — delete them first"
                ));
            }

            stage_tenant_deletion(
                hasura_transaction,
                &tenant_id_cloned,
                &log_tenant_id,
                audit_actor,
                Some(&task_id),
            )
            .await?;

            delete_tenant_postgres(&hasura_transaction, &tenant_id_cloned)
                .await
                .map_err(|err| anyhow!("Error deleting tenant from postgres db: {err}"))?;

            Ok(())
        })
    })
    .await?;
    kick_audit_log_outbox();

    // External cleanup starts only after commit: rollback must leave a live
    // tenant's realm and objects intact. Missing database rows permit retries.
    delete_tenant_related_data(&tenant_id, &realm)
        .await
        .map_err(|e| anyhow!("Error deleting related non-transactional data: {e}"))
}

/// Record completion only after both database and external cleanup succeed.
/// The admin is optional: a task queued before it was passed still runs.
#[instrument(err, skip(task_execution))]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn delete_tenant_t(
    tenant_id: String,
    realm: String,
    task_execution: TasksExecution,
    audit_actor: Option<AuditActor>,
) -> Result<()> {
    let res = delete_tenant(tenant_id, realm, &task_execution, audit_actor).await;

    let _ = match res {
        Ok(_) => {
            update_complete(&task_execution, None).await?;
        }
        Err(err) => {
            let error = format!("Error deleting tenant: {err}");
            update_fail(&task_execution, &error).await?;
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_task_queued_without_its_admin_still_reads() {
        let params: delete_tenant_tParams = serde_json::from_value(json!({
            "tenant_id": "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61",
            "realm": "tenant-4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61",
            "task_execution": {
                "id": "task", "tenant_id": "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43",
                "election_event_id": null, "name": "Delete", "task_type": "DELETE_TENANT",
                "execution_status": "IN_PROGRESS", "created_at": "2026-10-13T00:00:00Z",
                "start_at": null, "end_at": null, "annotations": null, "labels": null,
                "logs": null, "executed_by_user": "Ada",
            },
        }))
        .unwrap();
        assert_eq!(params.audit_actor, None);
    }
}
