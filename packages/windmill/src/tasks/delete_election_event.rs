// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    services::delete_election_event::{
        delete_election_event_db, delete_election_event_immudb, delete_keycloak_realm,
        delete_s3_related_artifacts,
    },
    types::error::Result,
};
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
    delete_election_event_immudb(&tenant_id, &election_event_id).await?;
    delete_election_event_db(&tenant_id, &election_event_id).await?;
    delete_s3_related_artifacts(&tenant_id, &election_event_id).await?;
    delete_keycloak_realm(&realm).await?;
    Ok(())
}
