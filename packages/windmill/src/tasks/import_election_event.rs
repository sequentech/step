// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::election_event_id_exists;
use crate::postgres::maintenance::vacuum_analyze_direct;
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log::ElectoralLogAdminContext;
use crate::services::protocol_manager::get_event_databases;
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
    #[serde(default)]
    pub may_write_secret_attributes: bool,
    #[serde(default)]
    pub secret_write_initiator: Option<ElectoralLogAdminContext>,
}

/// After a failed import, drop the electoral-log database it created for the new
/// election event, which the rolled-back Hasura transaction no longer has. The
/// database is kept when an election event with that ID exists in any tenant, or
/// when it is registered to another tenant. Failures are logged.
async fn drop_unimported_event_database(tenant_id: &str, election_event_id: &str) {
    let dropped: anyhow::Result<bool> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.build_transaction().read_only(true).start().await?;
        let exists = election_event_id_exists(&transaction, election_event_id).await?;
        transaction.commit().await?;
        if exists {
            return Ok(false);
        }
        let databases = get_event_databases().await?;
        let Some(entry) = databases.event(election_event_id).await? else {
            return Ok(false);
        };
        if uuid::Uuid::parse_str(&entry.tenant_id)? != uuid::Uuid::parse_str(tenant_id)? {
            return Ok(false);
        }
        databases.drop_event(tenant_id, election_event_id).await?;
        Ok(true)
    }
    .await;
    match dropped {
        Ok(true) => info!("Dropped the electoral-log database of the unimported event {election_event_id}"),
        Ok(false) => {}
        Err(error) => tracing::error!(
            "Could not drop the electoral-log database of the unimported event {election_event_id}: {error:?}"
        ),
    }
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
            let err_str = format!("Error processing election event document: {error:#}");
            drop_unimported_event_database(&tenant_id, &election_event_id).await;
            let _ = update_fail(&task_execution, &err_str).await;
            Err(err_str.into())
        }
    }
}
