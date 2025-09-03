// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::export::export_tally_results::export_tally_results_to_xlsx;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::tasks_execution::*;
use crate::types::error::Result;
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_tally_results_to_xlsx_task(
    tenant_id: String,
    election_event_id: String,
    results_sqlite_document_id: String,
    results_event_id: String,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let result = provide_hasura_transaction(|hasura_transaction| {
        let document_copy = document_id.clone();
        Box::pin(async move {
            export_tally_results_to_xlsx(
                hasura_transaction,
                tenant_id,
                election_event_id,
                results_sqlite_document_id,
                results_event_id,
                document_copy.clone(),
            )
            .await
        })
    })
    .await;

    match result {
        Ok(_) => {
            let _res = update_complete(&task_execution, Some(document_id.clone())).await;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Error importing applications: {err:?}");
            let _res = update_fail(&task_execution, &err.to_string()).await;
            Err(err_str.into())
        }
    }
}
