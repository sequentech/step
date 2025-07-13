// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::delete_election_event as delete_election_event_postgres;
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

async fn delete_election_event_related_data(
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
) -> Result<()> {
    let immudb_future = delete_election_event_immudb(tenant_id, election_event_id);

    let documents_future = delete_election_event_related_documents(tenant_id, election_event_id);

    let keycloak_future = delete_keycloak_realm(realm);

    let (_immudb_result, _documents_result, _keycloak_result) =
        try_join!(immudb_future, documents_future, keycloak_future)?;

    Ok(())
}

async fn delete_election_event(
    tenant_id: String,
    election_event_id: String,
    realm: String,
) -> AnyhowResult<()> {
    let results = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        Box::pin(async move {
            delete_event_b3(hasura_transaction, &tenant_id, &election_event_id)
                .await
                .map_err(|err| anyhow!("Error deleting election event from hasura db: {err}"))?;

            delete_election_event_postgres(&hasura_transaction, &tenant_id, &election_event_id)
                .await
        })
    })
    .await;

    match &results {
        Ok(_) => {
            match delete_election_event_related_data(&tenant_id, &election_event_id, &realm).await {
                Ok(_) => (),
                Err(e) => return Err(anyhow!("Error deleting related data: {e}")),
            }
        }
        Err(err) => {
            let error = format!("Error deleting election event: {err}");
            return Err(anyhow!(error));
        }
    }
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
