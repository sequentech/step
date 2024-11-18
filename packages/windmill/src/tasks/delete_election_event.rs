// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::delete_election_event;
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
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::{info, instrument};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn delete_election_event_t(
    tenant_id: String,
    election_event_id: String,
    realm: String,
) -> Result<()> {
    provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        Box::pin(async move {
            delete_event_b3(hasura_transaction, &tenant_id, &election_event_id)
                .await
                .map_err(|err| anyhow!("Error deleting election event from hasura db: {err}"))?;

            delete_election_event(&hasura_transaction, &tenant_id, &election_event_id).await
        })
    })
    .await?;

    let immudb_result = delete_election_event_immudb(&tenant_id, &election_event_id)
        .await
        .map_err(|err| anyhow!("Error deleting election event immudb database: {err}"));
    info!("immudb result: {:?}", immudb_result);
    let documents_result = delete_election_event_related_documents(&tenant_id, &election_event_id)
        .await
        .map_err(|err| anyhow!("Error deleting election event related documents: {err}"));
    info!("documents result: {:?}", documents_result);
    delete_keycloak_realm(&realm)
        .await
        .map_err(|err| anyhow!("Error deleting election event keycloak realm: {err}"))?;
    Ok(())
}
