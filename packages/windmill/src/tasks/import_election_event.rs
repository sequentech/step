// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::maintenance::vacuum_analyze_direct;
use crate::services::delete_election_event::drop_uncommitted_event_database;
use crate::services::electoral_log::ElectoralLogAdminContext;
use crate::services::import::import_users::ImportUsersPrivileges;
use crate::services::import::rejection::problems_of;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::tasks_execution::{update_complete, update_fail, update_fail_with_problems};
use crate::{
    services::import::import_election_event::{self as import_election_event_service},
    types::error::Result,
};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

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
    /// The administrator who started the import, set by the server; the
    /// signing configuration's log entries name them.
    #[serde(default)]
    pub importer: Option<ElectoralLogAdminContext>,
    /// Whether the importer holds `messaging-config-write`, set by the
    /// server: a bundle's messaging configuration is imported only then.
    #[serde(default)]
    pub messaging_config_permission: crate::services::messaging::config::MessagingConfigPermission,
    /// What the importing user may grant through the voters file, derived by
    /// the server from that user's permissions.
    #[serde(default)]
    pub privileges: ImportUsersPrivileges,
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
    // What the import left out, with its reason, for the task's log.
    let notes = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let result = provide_hasura_transaction(|hasura_transaction| {
        let object = object.clone();
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let notes = notes.clone();

        Box::pin(async move {
            let imported = import_election_event_service::process_document(
                hasura_transaction,
                object,
                election_event_id,
                tenant_id,
            )
            .await?;
            *notes.lock().unwrap_or_else(|e| e.into_inner()) = imported;
            Ok(())
        })
    })
    .await;

    match &result {
        Ok(_) => {
            // Execute database maintenance
            info!("Performing mainteinance after election event import.");
            vacuum_analyze_direct().await?;
            let mut task_execution = task_execution.clone();
            for note in notes.lock().unwrap_or_else(|e| e.into_inner()).iter() {
                task_execution.logs = serde_json::to_value(
                    crate::services::serialize_tasks_logs::append_general_log(
                        &task_execution.logs,
                        note,
                    ),
                )
                .ok();
            }
            let _ = update_complete(&task_execution, Some(object.document_id.clone())).await;
            Ok(())
        }
        Err(error) => {
            let err_str = format!("Error processing election event document: {error:#}");
            drop_uncommitted_event_database(&tenant_id, &election_event_id).await;
            let _ = match problems_of(error) {
                Some(problems) => {
                    update_fail_with_problems(&task_execution, &err_str, &problems).await
                }
                None => update_fail(&task_execution, &err_str).await,
            };
            Err(err_str.into())
        }
    }
}
