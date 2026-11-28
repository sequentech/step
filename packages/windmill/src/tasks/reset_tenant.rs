// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Returns a tenant to what it was before its first configuration package.
//! See `services::tenant_reset`.

use crate::services::audit_log::AuditActor;
use crate::services::tasks_execution::update_fail;
use crate::services::tenant_reset::{run_reset, ResetOptions};
use crate::types::error::Result;
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{error, instrument};

/// `keep_user` is the Keycloak id of the admin doing the reset, who stays in
/// the tenant realm as they are.
#[instrument(err, skip(task_execution))]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn reset_tenant_t(
    tenant_id: String,
    keep_user: String,
    options: ResetOptions,
    task_execution: TasksExecution,
    audit_actor: Option<AuditActor>,
) -> Result<()> {
    match run_reset(
        &task_execution,
        &tenant_id,
        audit_actor,
        &keep_user,
        &options,
    )
    .await
    {
        // The reset completed its task with its last change.
        Ok(_) => {}
        Err(err) => {
            error!("Reset of tenant {tenant_id} failed: {err:?}");
            update_fail(
                &task_execution,
                &format!("Error resetting the tenant: {err}"),
            )
            .await?;
        }
    }
    Ok(())
}
