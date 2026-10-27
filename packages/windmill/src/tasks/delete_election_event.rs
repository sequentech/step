// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::delete_election_event as delete_election_event_postgres;
use crate::services::ballot_box_seal::refuse_sealed_event_delete;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::{
    services::{
        delete_election_event::{
            archive_protocol_boards, delete_election_event_electoral_log,
            delete_election_event_related_documents, delete_keycloak_realm,
            find_election_event_deletion_refusal,
        },
        providers::transactions_provider::provide_hasura_transaction,
    },
    types::error::Result,
};
use anyhow::{anyhow, Result as AnyhowResult};
use celery::error::TaskError;
use futures::try_join;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

#[instrument(err)]
async fn delete_election_event_related_data(
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
) -> AnyhowResult<()> {
    let documents_future = delete_election_event_related_documents(tenant_id, election_event_id);
    let keycloak_future = delete_keycloak_realm(realm);
    try_join!(documents_future, keycloak_future)?;

    Ok(())
}

#[instrument(err)]
async fn delete_election_event(
    tenant_id: String,
    election_event_id: String,
    realm: String,
    permission_labels: Vec<String>,
    task_id: String,
) -> AnyhowResult<()> {
    let tenant_id_cloned = tenant_id.clone();
    let election_event_id_cloned = election_event_id.clone();
    provide_hasura_transaction(|hasura_transaction| {
        Box::pin(async move {
            let expected_realm=sequent_core::services::keycloak::get_event_realm(&tenant_id_cloned,&election_event_id_cloned);
            anyhow::ensure!(realm==expected_realm,"Deletion task names a different realm");
            hasura_transaction.query_one("SELECT id FROM sequent_backend.election_event WHERE tenant_id=$1::text::uuid AND id=$2::text::uuid FOR UPDATE", &[&tenant_id_cloned,&election_event_id_cloned]).await?;
            let permission_labels=crate::services::board_operations::authorize_deletion(hasura_transaction,
                &tenant_id_cloned,&election_event_id_cloned,&task_id).await?;
            if let Some(refusal) = find_election_event_deletion_refusal(
                hasura_transaction,
                &tenant_id_cloned,
                &election_event_id_cloned,
                &permission_labels,
            )
            .await?
            {
                if refusal != crate::services::delete_election_event::ElectionEventDeletionRefusal::LockedDown {
                    return Err(refusal.into());
                }
            }

            delete_election_event_in(
                hasura_transaction,
                &tenant_id_cloned,
                &election_event_id_cloned,
                &realm,
            )
            .await
        })
    })
    .await?;

    // Dropping a database can't be rolled back, so it runs once the event's deletion
    // is committed. Dropping is idempotent: a failed drop can be run again with
    // `electoral-log-admin drop-event-database`.
    delete_election_event_electoral_log(&tenant_id, &election_event_id)
        .await
        .map_err(|err| anyhow!("Error dropping the election event's electoral-log database: {err}"))
}

/// Deletes the election event in `hasura_transaction`, with its boards and
/// its other non-transactional data. An event with ballot box seals is
/// refused first, before anything is deleted (VOTE-FREEZE).
#[instrument(skip(hasura_transaction), err)]
pub async fn delete_election_event_in(
    hasura_transaction: &deadpool_postgres::Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
) -> AnyhowResult<()> {
    // Hold the parent row so a concurrent ceremony cannot add metadata between
    // archival and the explicit child cleanup (foreign keys take KEY SHARE).
    hasura_transaction.query_one("SELECT id FROM sequent_backend.election_event WHERE tenant_id=$1::text::uuid AND id=$2::text::uuid FOR UPDATE", &[&tenant_id,&election_event_id]).await?;
    refuse_sealed_event_delete(hasura_transaction, tenant_id, election_event_id).await?;

    archive_protocol_boards(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|err| anyhow!("Error deleting election event from hasura db: {err}"))?;

    delete_election_event_postgres(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|err| anyhow!("Error deleting election event from postgres db: {err}"))?; // FIX APPLIED

    delete_election_event_related_data(tenant_id, election_event_id, realm)
        .await
        .map_err(|e| anyhow!("Error deleting related non-transactional data: {e}"))?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn delete_election_event_t(
    tenant_id: String,
    election_event_id: String,
    realm: String,
    task_execution: TasksExecution,
    permission_labels: Vec<String>,
) -> Result<()> {
    let res = delete_election_event(
        tenant_id,
        election_event_id,
        realm,
        permission_labels,
        task_execution.id.clone(),
    )
    .await;

    let _ = match res {
        Ok(_) => {
            update_complete(&task_execution, None).await?;
        }
        Err(err) => {
            let error = format!("Error deleting election event: {err}");
            update_fail(&task_execution, &error).await?;
        }
    };
    Ok(())
}
