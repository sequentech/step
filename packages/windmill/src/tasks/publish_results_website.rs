// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tally_results_publication::mark_publication_failed;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::results_publication::publish_results_website_artifacts;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::Result;
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn publish_results_website_task(
    tenant_id: String,
    election_event_id: String,
    publication_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let result = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let publication_id = publication_id.clone();
        Box::pin(async move {
            publish_results_website_artifacts(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                &publication_id,
            )
            .await
        })
    })
    .await;

    match result {
        Ok(_) => {
            let _res = update_complete(&task_execution, Some(publication_id.clone())).await;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Error publishing results website: {err:?}");
            let _res = update_fail(&task_execution, &err_str).await;
            let _publication_res = provide_hasura_transaction(|hasura_transaction| {
                let tenant_id = tenant_id.clone();
                let election_event_id = election_event_id.clone();
                let publication_id = publication_id.clone();
                let err_str = err_str.clone();
                Box::pin(async move {
                    mark_publication_failed(
                        hasura_transaction,
                        &tenant_id,
                        &election_event_id,
                        &publication_id,
                        &err_str,
                    )
                    .await
                })
            })
            .await;
            Err(err_str.into())
        }
    }
}
