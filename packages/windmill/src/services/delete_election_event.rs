// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::electoral_log_board::get_election_board;
use super::jwks::remove_realm_jwks;
use crate::postgres::election::get_elections;
use crate::services::electoral_log_board::get_event_board;
use crate::services::electoral_log_board::get_immudb_client;
use anyhow::{anyhow, ensure, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use futures::future::try_join_all;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::s3;
use tracing::info;
use tracing::{event, instrument, Level};

#[instrument(err)]
pub async fn delete_keycloak_realm(realm: &str) -> Result<()> {
    let client = KeycloakAdminClient::new().await?;
    remove_realm_jwks(&realm).await?;

    let realm_exists = client
        .client
        .realm_get(&realm)
        .await
        .map_err(|err| anyhow!("Keycloak error: {err:?}"));

    info!("realm_exists? {:?}", realm_exists.is_ok());

    if realm_exists.is_ok() {
        client
            .client
            .realm_delete(&realm)
            .await
            .map_err(|err| anyhow!("Keycloak error: {err:?}"))?;
    }
    Ok(())
}

/// Retire protocol boards while preserving their signed records.
pub async fn archive_protocol_boards(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    event_id: &str,
) -> Result<()> {
    let boards = crate::postgres::protocol_board::get_event_protocol_boards(
        transaction,
        tenant_id,
        event_id,
    )
    .await?;
    if boards.is_empty() {
        return Ok(());
    }
    // Refuse before touching the independent board store when application
    // metadata would prevent the surrounding deletion from committing.
    let cleanup_allowed: bool = transaction
        .query_one(
            "SELECT NOT EXISTS (
                SELECT 1 FROM pg_constraint
                WHERE conrelid = 'sequent_backend.protocol_board'::regclass
                  AND contype = 'f'
                  AND confrelid IN (
                      'sequent_backend.election_event'::regclass,
                      'sequent_backend.keys_ceremony'::regclass,
                      'sequent_backend.tally_session'::regclass,
                      'sequent_backend.protocol_board'::regclass
                  )
                  AND confdeltype <> 'c'
            )",
            &[],
        )
        .await?
        .get(0);
    ensure!(
        cleanup_allowed,
        "protocol board metadata currently restricts election event deletion"
    );
    let url = std::env::var("WBRAID_BOARD_DATABASE_URL")
        .context("WBRAID_BOARD_DATABASE_URL must be set")?;
    let db = wbraid_b4::db::BoardDb::open_url(&url).await?;
    let pool = db
        .postgres_pool()
        .context("native board archive requires PostgreSQL")?;
    for row in boards {
        sqlx::query("UPDATE boards SET status='archived' WHERE name=$1")
            .bind(&row.name)
            .execute(pool)
            .await?;
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
