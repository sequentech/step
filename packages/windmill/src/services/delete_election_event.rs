// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::electoral_log_board::get_election_board;
use super::jwks::remove_realm_jwks;
use crate::postgres::election::get_elections;
use crate::services::electoral_log_board::get_board_client;
use crate::services::electoral_log_board::get_event_board;
use anyhow::{anyhow, Context, Result};
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
    let url = std::env::var("WBRAID_BOARD_DATABASE_URL")
        .context("WBRAID_BOARD_DATABASE_URL must be set")?;
    let mut options: sqlx::postgres::PgConnectOptions = url.parse()?;
    if let Ok(password) = std::env::var("WBRAID_B4_PG_PASSWORD") {
        options = options.password(&password);
    }
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    let db = wbraid_b4::db::BoardDb::from_postgres_pool(pool).await?;
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
pub async fn archive_election_event_audit(
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &[String],
) -> Result<()> {
    let mut client = get_board_client().await?;
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let name = get_event_board(tenant_id, election_event_id, &slug);
    client.archive_board(&name).await?;
    for election_id in election_ids {
        client
            .archive_board(&get_election_board(tenant_id, election_id, &slug))
            .await?;
    }
    use electoral_log::client::database_audit::{database_audit_board, DatabaseAuditTable};
    for table in [
        DatabaseAuditTable::PgauditHasura,
        DatabaseAuditTable::PgauditKeycloak,
    ] {
        client
            .archive_board(&database_audit_board(tenant_id, election_event_id, table)?)
            .await?;
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
