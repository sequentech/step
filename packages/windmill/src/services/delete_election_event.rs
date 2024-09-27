// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{database::get_hasura_pool, jwks::remove_realm_jwks};
use crate::postgres::election_event::delete_election_event;
use crate::services::protocol_manager::get_event_board;
use crate::services::protocol_manager::get_immudb_client;
use crate::services::s3;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak::KeycloakAdminClient;
use tracing::{event, instrument, Level};

#[instrument(err)]
pub async fn delete_keycloak_realm(realm: &str) -> Result<()> {
    let client = KeycloakAdminClient::new().await?;
    remove_realm_jwks(&realm).await?;
    let _ = client
        .client
        .realm_delete(&realm)
        .await
        .map_err(|err| anyhow!("Keycloak error: {err:?}"));
    Ok(())
}

#[instrument(err)]
pub async fn delete_election_event_db(tenant_id: &str, election_event_id: &str) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    delete_election_event(&hasura_transaction, &tenant_id, &election_event_id).await?;

    hasura_transaction
        .commit()
        .await
        .map_err(|err| anyhow!("error comitting transaction: {err:?}"))?;
    Ok(())
}

#[instrument(err)]
pub async fn delete_election_event_immudb(tenant_id: &str, election_event_id: &str) -> Result<()> {
    let mut client = get_immudb_client().await?;
    let board_name = get_event_board(tenant_id, election_event_id);

    event!(Level::INFO, "database name = {board_name}");
    client
        .delete_database(&board_name)
        .await
        .map_err(|err| anyhow!("error delete immudb database: {err:?}"))?;
    Ok(())
}

#[instrument(err)]
pub async fn delete_election_event_related_documents(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let documents_prefix = format!("tenant-{}/event-{}/", tenant_id, election_event_id);
    let bucket = s3::get_private_bucket()?;
    s3::delete_files_from_s3(bucket, documents_prefix, false)
        .await
        .map_err(|err| anyhow!("Error delete private files from s3: {err:?}"))?;
    Ok(())
}
