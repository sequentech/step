// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::get_elections_ids;
use crate::postgres::election_event::delete_election_event as delete_election_event_postgres;
use crate::postgres::election_event_trash::lock_claimed_election_event;
use crate::services::database::get_hasura_pool;
use crate::services::delete_election_event::delete_election_event_b3;
use crate::services::election_event_trash::{
    move_election_event_to_trash, trash_retention_days, TrashOutcome, TRASH_RETENTION_EXECUTOR,
};
use crate::services::tasks_execution::{post, update_complete, update_fail};
use crate::types::tasks::ETasksExecution;
use crate::{
    services::{
        delete_election_event::{
            delete_election_event_immudb, delete_election_event_related_documents, delete_event_b3,
            delete_keycloak_realm,
        },
        providers::transactions_provider::provide_hasura_transaction,
    },
    types::error::Result,
};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use futures::try_join;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{info, instrument};

#[instrument(err)]
async fn delete_election_event_related_data(
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    election_ids: &Vec<String>,
) -> AnyhowResult<()> {
    let immudb_future = delete_election_event_immudb(tenant_id, election_event_id);
    let b3_future = delete_election_event_b3(tenant_id, election_event_id, election_ids);
    let documents_future = delete_election_event_related_documents(tenant_id, election_event_id);
    let keycloak_future = delete_keycloak_realm(realm);
    try_join!(immudb_future, b3_future, documents_future, keycloak_future)?;

    Ok(())
}

/// Permanently deletes the election event in `hasura_transaction`, with its
/// boards and its other non-transactional data.
#[instrument(skip(hasura_transaction), err)]
async fn delete_election_event_in(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
) -> AnyhowResult<()> {
    delete_event_b3(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|err| anyhow!("Error deleting election event from hasura db: {err}"))?;

    let election_ids = get_elections_ids(hasura_transaction, tenant_id, election_event_id).await?;

    delete_election_event_postgres(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|err| anyhow!("Error deleting election event from postgres db: {err}"))?; // FIX APPLIED

    delete_election_event_related_data(tenant_id, election_event_id, realm, &election_ids)
        .await
        .map_err(|e| anyhow!("Error deleting related non-transactional data: {e}"))?;

    Ok(())
}

#[instrument(err)]
async fn move_to_trash(tenant_id: String, election_event_id: String) -> AnyhowResult<()> {
    // Read first: a misconfigured retention leaves the event where it is.
    let retention_days = trash_retention_days()?;
    provide_hasura_transaction(|hasura_transaction| {
        Box::pin(async move {
            match move_election_event_to_trash(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                retention_days,
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
    .await
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
) -> Result<()> {
    let res = move_to_trash(tenant_id, election_event_id).await;

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

/// Permanently deletes an election event claimed after its Trash retention.
/// The claim was committed before, so the event can't be restored meanwhile;
/// a failure rolls the database back and the claim is retried later.
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
    let res = async {
        delete_election_event_in(&hasura_transaction, tenant_id, election_event_id, &realm).await?;
        hasura_transaction
            .commit()
            .await
            .context("Error committing the purge")
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
                &format!("Error deleting election event from Trash, it will be retried: {err}"),
            )
            .await?;
            Err(err)
        }
    }
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
