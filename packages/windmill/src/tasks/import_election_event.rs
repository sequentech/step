// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Imports a packaged election event into an existing tenant.
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

/// Payload for restoring an election event from an uploaded export archive.
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportElectionEventBody {
    /// Tenant receiving the import.
    pub tenant_id: String,
    /// Source document ID containing the ZIP.
    pub document_id: String,
    /// Optional password when the archive is encrypted.
    pub password: Option<String>,
    /// When true, validate only without mutating Hasura rows.
    pub check_only: Option<bool>,
    /// Expected SHA-256 of the document for integrity verification.
    pub sha256: Option<String>,
}

mod import_election_event_task {
    #![allow(missing_docs)]
    #![allow(clippy::missing_docs_in_private_items)]

    use super::*;
    /// Celery task: import election event data from a document.
    #[instrument(err)]
    #[wrap_map_err::wrap_map_err(TaskError)]
    #[celery::task]
    pub async fn import_election_event(
        object: super::ImportElectionEventBody,
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
            Ok(()) => {
                // Execute database maintenance
                info!("Performing mainteinance after election event import.");
                vacuum_analyze_direct().await?;
                let _ = update_complete(&task_execution, Some(object.document_id.clone())).await;
                Ok(())
            }
            Err(error) => {
                let err_str = format!("Error process election event document: {error}");
                let _ = update_fail(&task_execution, &err_str).await;
                Err(err_str.into())
            }
        }
    }
}

pub use import_election_event_task::import_election_event;
