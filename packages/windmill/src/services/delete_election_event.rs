// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::jwks::remove_realm_jwks;
use super::protocol_manager::{get_b3_pgsql_client, get_election_board};
use crate::postgres::election::get_elections;
use crate::services::protocol_manager::get_event_board;
use crate::services::protocol_manager::get_immudb_client;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use futures::future::try_join_all;
use keycloak::KeycloakError;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::s3;
use tracing::info;
use tracing::{event, instrument, Level};

#[instrument(err)]
pub async fn delete_keycloak_realm(realm: &str) -> Result<()> {
    let client = KeycloakAdminClient::new().await?;
    remove_realm_jwks(&realm).await?;

    // DELETE is idempotent only when the realm is confirmed absent. A failed
    // existence lookup must never turn authorization or transport errors into success.
    realm_deletion_result(client.client.realm_delete(realm).await)
}

fn realm_deletion_result(result: std::result::Result<(), KeycloakError>) -> Result<()> {
    match result {
        Ok(()) | Err(KeycloakError::HttpFailure { status: 404, .. }) => Ok(()),
        Err(error) => Err(anyhow::Error::new(error)).context("Failed to delete Keycloak realm"),
    }
}

#[instrument(err)]
pub async fn delete_event_b3(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let mut board_client = get_b3_pgsql_client().await?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);

    let elections = get_elections(&hasura_transaction, tenant_id, election_event_id).await?;
    board_client.delete_board(board_name.as_str()).await?;

    for election in elections {
        let board_name = get_election_board(tenant_id, &election.id, &slug);
        board_client.delete_board(board_name.as_str()).await?;
    }

    Ok(())
}

#[instrument(err)]
pub async fn delete_election_event_b3(
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &Vec<String>,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);
    let mut board_client = get_b3_pgsql_client().await?;
    let existing: Option<b3::client::pgsql::B3IndexRow> =
        board_client.get_board(board_name.as_str()).await?;

    if existing.is_some() {
        board_client.delete_board(board_name.as_str()).await?;
    }

    for election_id in election_ids {
        let board_name = get_election_board(tenant_id, &election_id, &slug);
        let existing: Option<b3::client::pgsql::B3IndexRow> =
            board_client.get_board(board_name.as_str()).await?;

        if existing.is_some() {
            board_client.delete_board(board_name.as_str()).await?;
        }
    }
    Ok(())
}

#[instrument(err)]
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
pub async fn delete_election_event_related_documents(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let documents_prefix = format!("tenant-{}/event-{}/", tenant_id, election_event_id);
    let bucket = s3::get_private_bucket()?;
    s3::delete_files_from_s3(bucket, documents_prefix.clone(), s3::S3Endpoint::Server)
        .await
        .map_err(|err| anyhow!("Error delete private files from s3: {err:?}"))?;

    // Also delete the public files related to the election event, such as the election event config
    let public_bucket = s3::get_public_bucket()?;
    s3::delete_files_from_s3(
        public_bucket.clone(),
        documents_prefix,
        s3::S3Endpoint::Server,
    )
    .await
    .map_err(|err| anyhow!("Error delete public files from s3: {err:?}"))?;

    let results_index_key = format!("results-index/{election_event_id}.json");
    s3::delete_files_from_s3(public_bucket, results_index_key, s3::S3Endpoint::Server)
        .await
        .map_err(|err| anyhow!("Error delete public results index from s3: {err:?}"))?;
    Ok(())
}

#[cfg(test)]
mod realm_deletion_tests {
    use super::*;

    fn http_failure(status: u16) -> KeycloakError {
        KeycloakError::HttpFailure {
            status,
            body: None,
            text: String::new(),
        }
    }

    #[test]
    fn deleted_and_already_absent_realms_complete_cleanup() {
        assert!(realm_deletion_result(Ok(())).is_ok());
        assert!(realm_deletion_result(Err(http_failure(404))).is_ok());
    }

    #[test]
    fn other_http_failures_preserve_the_error_for_the_cleanup_task() {
        for status in [400, 401, 403, 409, 429, 500, 503] {
            let error = realm_deletion_result(Err(http_failure(status))).unwrap_err();
            assert!(matches!(
                error.downcast_ref::<KeycloakError>(),
                Some(KeycloakError::HttpFailure { status: actual, .. }) if *actual == status
            ));
        }
    }

    #[test]
    fn request_failures_do_not_mean_the_realm_is_absent() {
        let request_error = reqwest::Client::new().get("http://[").build().unwrap_err();
        let error =
            realm_deletion_result(Err(KeycloakError::ReqwestFailure(request_error))).unwrap_err();
        assert!(matches!(
            error.downcast_ref::<KeycloakError>(),
            Some(KeycloakError::ReqwestFailure(_))
        ));
    }
}
