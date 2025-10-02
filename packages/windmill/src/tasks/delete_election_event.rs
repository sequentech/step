// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::get_elections_ids;
use crate::postgres::election_event::delete_election_event as delete_election_event_postgres;
use crate::services::delete_election_event::delete_election_event_b3;
use crate::services::tasks_execution::{update_complete, update_fail};
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
    election_ids: &Vec<String>,
) -> AnyhowResult<()> {
    let immudb_future = delete_election_event_immudb(tenant_id, election_event_id);
    let b3_future = delete_election_event_b3(tenant_id, election_event_id, election_ids);
    let documents_future = delete_election_event_related_documents(tenant_id, election_event_id);
    let keycloak_future = delete_keycloak_realm(realm);
    try_join!(immudb_future, b3_future, documents_future, keycloak_future)?;

    Ok(())
}

#[instrument(err)]
async fn delete_election_event(
    tenant_id: String,
    election_event_id: String,
    realm: String,
) -> AnyhowResult<()> {
    let tenant_id_cloned = tenant_id.clone();
    let election_event_id_cloned = election_event_id.clone();
    let realm_cloned = realm.clone();

    provide_hasura_transaction(|hasura_transaction| {
        Box::pin(async move {
            delete_event_b3(
                hasura_transaction,
                &tenant_id_cloned,
                &election_event_id_cloned,
            )
            .await
            .map_err(|err| anyhow!("Error deleting election event from hasura db: {err}"))?;

            let election_ids = get_elections_ids(
                &hasura_transaction,
                &tenant_id_cloned,
                &election_event_id_cloned,
            )
            .await?;

            delete_election_event_postgres(
                &hasura_transaction,
                &tenant_id_cloned,
                &election_event_id_cloned,
            )
            .await
            .map_err(|err| anyhow!("Error deleting election event from postgres db: {err}"))?; // FIX APPLIED

            delete_election_event_related_data(
                &tenant_id_cloned,
                &election_event_id_cloned,
                &realm_cloned,
                &election_ids,
            )
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
pub async fn delete_election_event_t(
    tenant_id: String,
    election_event_id: String,
    realm: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let res = delete_election_event(tenant_id, election_event_id, realm).await;

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
