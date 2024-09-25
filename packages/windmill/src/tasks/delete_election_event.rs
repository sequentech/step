// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    services::delete_election_event::{
        delete_election_event_db, delete_election_event_immudb,
        delete_election_event_related_documents, delete_keycloak_realm,
    },
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn delete_election_event_t(
    tenant_id: String,
    election_event_id: String,
    realm: String,
) -> Result<()> {
    delete_election_event_db(&tenant_id, &election_event_id)
        .await
        .map_err(|err| anyhow!("Error delete election event from hasura db: {err}"))?;

    delete_election_event_immudb(&tenant_id, &election_event_id)
        .await
        .map_err(|err| anyhow!("Error delete election event immudb database: {err}"))?;
    delete_election_event_related_documents(&tenant_id, &election_event_id)
        .await
        .map_err(|err| anyhow!("Error delete election event related documents: {err}"))?;
    delete_keycloak_realm(&realm)
        .await
        .map_err(|err| anyhow!("Error delete election event keycloak realm: {err}"))?;
    Ok(())
}
