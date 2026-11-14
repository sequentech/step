// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::delete_election_event as delete_election_event_postgres;
use crate::postgres::election_event_trash::lock_claimed_election_event;
use crate::services::audit_log::AuditActor;
use crate::services::ballot_box_seal::refuse_sealed_event_delete;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_trash::{
    move_election_event_to_trash, trash_retention_days, TrashOutcome, TRASH_RETENTION_EXECUTOR,
};
use crate::services::tasks_execution::{post, update_complete, update_fail};
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use crate::types::tasks::ETasksExecution;
use crate::{
    services::{
        delete_election_event::{
            archive_protocol_boards, delete_election_event_related_documents,
            delete_keycloak_realm, retire_election_event_electoral_log,
            stage_election_event_deletion,
        },
        providers::transactions_provider::provide_hasura_transaction,
    },
    types::error::Result,
};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use futures::try_join;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{info, instrument};

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

/// Permanently deletes an election event claimed after its Trash retention,
/// with the deletion users made before Trash existed. The claim was committed
/// before, so the event can't be restored meanwhile; a failure in the
/// transaction rolls it back and the claim is retried later.
#[instrument(err)]
async fn purge_trashed_election_event(
    tenant_id: &str,
    election_event_id: &str,
) -> AnyhowResult<()> {
    let mut hasura_db_client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting hasura client")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .context("Error starting the purge transaction")?;
    if !lock_claimed_election_event(&hasura_transaction, tenant_id, election_event_id).await? {
        info!(
            tenant_id,
            election_event_id, "Election event already purged"
        );
        return Ok(());
    }

    let task_execution = post(
        tenant_id,
        None,
        ETasksExecution::DELETE_ELECTION_EVENT,
        TRASH_RETENTION_EXECUTOR,
    )
    .await?;
    let realm = get_event_realm(tenant_id, election_event_id);
    // Names the retired log database, in the tenant log's entry and on the server.
    let retired_at = electoral_log::timestamp();
    let res = async {
        stage_election_event_deletion(
            &hasura_transaction,
            tenant_id,
            election_event_id,
            None,
            Some(&task_execution.id),
            retired_at,
        )
        .await?;
        delete_election_event_in(&hasura_transaction, tenant_id, election_event_id, &realm).await?;
        hasura_transaction
            .commit()
            .await
            .context("Error committing the purge")?;
        kick_audit_log_outbox();
        // As for deletions before Trash, retiring the log database can't be
        // rolled back, so it runs once the deletion is committed.
        retire_election_event_electoral_log(tenant_id, election_event_id, retired_at)
            .await
            .map_err(|err| {
                anyhow!("Error retiring the election event's electoral-log database: {err}")
            })
    }
    .await;

    match res {
        Ok(()) => {
            info!(
                tenant_id,
                election_event_id, "Purged election event from Trash"
            );
            update_complete(&task_execution, None).await
        }
        Err(err) => {
            update_fail(
                &task_execution,
                &format!("Error deleting election event from Trash: {err}"),
            )
            .await?;
            Err(err)
        }
    }
}

#[instrument(err)]
async fn move_to_trash(
    tenant_id: String,
    election_event_id: String,
    permission_labels: Vec<String>,
    deleted_by: Option<AuditActor>,
    task_id: String,
) -> AnyhowResult<()> {
    // Read first: a misconfigured retention leaves the event where it is.
    let retention_days = trash_retention_days()?;
    provide_hasura_transaction(|hasura_transaction| {
        Box::pin(async move {
            match move_election_event_to_trash(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                retention_days,
                &permission_labels,
                deleted_by.as_ref(),
                Some(&task_id),
            )
            .await?
            {
                TrashOutcome::Trashed {
                    trashed_at,
                    purge_after,
                } => info!(
                    tenant_id,
                    election_event_id,
                    %trashed_at,
                    %purge_after,
                    "Moved election event to Trash"
                ),
                TrashOutcome::AlreadyInTrash => {
                    info!(
                        tenant_id,
                        election_event_id, "Election event already in Trash"
                    )
                }
            }
            Ok(())
        })
    })
    .await?;
    kick_audit_log_outbox();
    Ok(())
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

/// Moves the election event to Trash. It is deleted permanently once its
/// Trash retention is over, unless it is restored first. `realm` is no longer
/// used; it stays so that messages keep the arguments older workers expect.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn delete_election_event_t(
    tenant_id: String,
    election_event_id: String,
    realm: String,
    task_execution: TasksExecution,
    permission_labels: Vec<String>,
    deleted_by: Option<AuditActor>,
) -> Result<()> {
    let res = move_to_trash(
        tenant_id,
        election_event_id,
        permission_labels,
        deleted_by,
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

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn purge_trashed_election_event_t(
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    purge_trashed_election_event(&tenant_id, &election_event_id).await?;
    Ok(())
}
