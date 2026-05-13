// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Coordinated deletion of an election event and dependent rows.

use super::jwks::remove_realm_jwks;
use super::protocol_manager::{get_b3_pgsql_client, get_election_board};
use crate::postgres::election::get_elections;
use crate::services::protocol_manager::get_event_board;
use crate::services::protocol_manager::get_immudb_client;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use futures::future::try_join_all;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::s3;
use tracing::info;
use tracing::{event, instrument, Level};

#[instrument(err)]
/// Delete a Keycloak realm if it exists.
///
/// # Errors
///
/// Returns an error if Keycloak cannot be reached or realm deletion fails.
pub async fn delete_keycloak_realm(realm: &str) -> Result<()> {
    let client = KeycloakAdminClient::new().await?;
    remove_realm_jwks(realm).await?;

    let realm_exists = client
        .client
        .realm_get(realm)
        .await
        .map_err(|err| anyhow!("Keycloak error: {err:?}"));

    info!("realm_exists? {:?}", realm_exists.is_ok());

    if realm_exists.is_ok() {
        client
            .client
            .realm_delete(realm)
            .await
            .map_err(|err| anyhow!("Keycloak error: {err:?}"))?;
    }
    Ok(())
}

#[instrument(err)]
/// Delete B3 boards for an election event and its elections.
///
/// # Errors
///
/// Returns an error if the B3 client cannot be created or boards cannot be deleted.
pub async fn delete_event_b3(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let mut board_client = get_b3_pgsql_client().await?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let event_board_name = get_event_board(tenant_id, election_event_id, &slug);

    let elections = get_elections(hasura_transaction, tenant_id, election_event_id).await?;
    board_client.delete_board(event_board_name.as_str()).await?;

    for election in elections {
        let election_board_name = get_election_board(tenant_id, &election.id, &slug);
        board_client
            .delete_board(election_board_name.as_str())
            .await?;
    }

    Ok(())
}

#[instrument(err)]
/// Delete B3 boards for an election event and the provided election ids, if present.
///
/// # Errors
///
/// Returns an error if environment configuration is missing or B3 operations fail.
pub async fn delete_election_event_b3(
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &Vec<String>,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let event_board_name = get_event_board(tenant_id, election_event_id, &slug);
    let mut board_client = get_b3_pgsql_client().await?;
    let event_board_row: Option<b3::client::pgsql::B3IndexRow> =
        board_client.get_board(event_board_name.as_str()).await?;

    if event_board_row.is_some() {
        board_client.delete_board(event_board_name.as_str()).await?;
    }

    for election_id in election_ids {
        let election_board_name = get_election_board(tenant_id, election_id, &slug);
        let election_board_row: Option<b3::client::pgsql::B3IndexRow> =
            board_client.get_board(election_board_name.as_str()).await?;

        if election_board_row.is_some() {
            board_client
                .delete_board(election_board_name.as_str())
                .await?;
        }
    }
    Ok(())
}

#[instrument(err)]
/// Delete the immudb database associated with an election event, if it exists.
///
/// # Errors
///
/// Returns an error if immudb cannot be reached or database deletion fails.
pub async fn delete_election_event_immudb(tenant_id: &str, election_event_id: &str) -> Result<()> {
    let mut client = get_immudb_client().await?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);

    event!(Level::INFO, "database name = {board_name}");

    let has_database = client
        .has_database(&board_name)
        .await
        .map_err(|err| anyhow!("error reading immudb database: {err:?}"))?;

    if has_database {
        client
            .delete_database(&board_name)
            .await
            .map_err(|err| anyhow!("error delete immudb database: {err:?}"))?;
    }
    Ok(())
}

#[instrument(err)]
/// Delete S3 documents associated with an election event (private and public prefixes).
///
/// # Errors
///
/// Returns an error if bucket resolution fails or S3 deletion fails.
pub async fn delete_election_event_related_documents(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let documents_prefix = format!("tenant-{tenant_id}/event-{election_event_id}/");
    let bucket = s3::get_private_bucket()?;
    s3::delete_files_from_s3(bucket, documents_prefix.clone(), false)
        .await
        .map_err(|err| anyhow!("Error delete private files from s3: {err:?}"))?;

    // Also delete the public files related to the election event, such as the election event config
    let public_bucket = s3::get_public_bucket()?;
    s3::delete_files_from_s3(public_bucket, documents_prefix, false)
        .await
        .map_err(|err| anyhow!("Error delete public files from s3: {err:?}"))?;
    Ok(())
}
