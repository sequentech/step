// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::maintenance::vacuum_analyze_direct;
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
    pub sha256: Option<String>,
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
    let result = provide_hasura_transaction(|hasura_transaction| {
        let object = object.clone();
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();

        Box::pin(async move {
            import_election_event_service::process_document(
                hasura_transaction,
                object,
                election_event_id,
                tenant_id,
            )
            .await
        })
    })
    .await;

    match &result {
        Ok(_) => {
            // Execute database maintenance
            info!("Performing mainteinance after election event import.");
            vacuum_analyze_direct().await?;
            let _ = update_complete(&task_execution, Some(object.document_id.clone())).await;
            Ok(())
        }
        Err(error) => {
            let err_str = format!(
                "Error process election event document: {}",
                error.to_string()
            );
            let _ = update_fail(&task_execution, &err_str).await;
            Err(err_str.into())
        }
    }
}
