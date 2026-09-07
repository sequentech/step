// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::count_tenant_election_events;
use crate::postgres::tenant::delete_tenant as delete_tenant_postgres;
use crate::services::delete_tenant::delete_tenant_related_data;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::{
    services::providers::transactions_provider::provide_hasura_transaction, types::error::Result,
};
use anyhow::{anyhow, Result as AnyhowResult};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

#[instrument(err)]
async fn delete_tenant(tenant_id: String, realm: String) -> AnyhowResult<()> {
    let tenant_id_cloned = tenant_id.clone();
    let realm_cloned = realm.clone();

    provide_hasura_transaction(|hasura_transaction| {
        Box::pin(async move {
            let existing_events =
                count_tenant_election_events(&hasura_transaction, &tenant_id_cloned).await?;
            if existing_events > 0 {
                return Err(anyhow!(
                    "cannot delete tenant {tenant_id_cloned}: {existing_events} election event(s) still exist — delete them first"
                ));
            }

            delete_tenant_postgres(&hasura_transaction, &tenant_id_cloned)
                .await
                .map_err(|err| anyhow!("Error deleting tenant from postgres db: {err}"))?;

            delete_tenant_related_data(&tenant_id_cloned, &realm_cloned)
                .await
                .map_err(|e| anyhow!("Error deleting related non-transactional data: {e}"))?;

            Ok(())
        })
    })
    .await
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn delete_tenant_t(
    tenant_id: String,
    realm: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let res = delete_tenant(tenant_id, realm).await;

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
