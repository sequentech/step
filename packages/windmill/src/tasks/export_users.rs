// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::election::{get_election_event_elections, ElectionHead};
use crate::services::export_users::{export_users_file, ExportBody};
use crate::services::s3;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::util;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use tempfile::{NamedTempFile, TempPath};
use tracing::{debug, info, instrument};

pub const USER_FIELDS: [&str; 8] = [
    "id",
    "email",
    "first_name",
    "last_name",
    "username",
    "enabled",
    "email_verified",
    "area-id",
];

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_users(body: ExportBody, document_id: String) -> Result<()> {
    let mut hasura_db_client = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring Hasura DB pool")?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting Hasura transaction")?;

    // Export the users to a temporary file
    let temp_path = export_users_file(&hasura_transaction, body.clone()).await?;
    let size = temp_path.metadata()?.len();

    // Upload to S3
    let (tenant_id, election_event_id) = match &body {
        ExportBody::Users {
            tenant_id,
            election_event_id,
            ..
        } => (
            tenant_id.to_string(),
            election_event_id.clone().unwrap_or_default(),
        ),
        ExportBody::TenantUsers { tenant_id } => (tenant_id.to_string(), "".to_string()),
    };

    let key = s3::get_document_key(
        &tenant_id,
        Some(&election_event_id),
        &document_id,
        "users-export",
    );
    let media_type = "text/csv".to_string();

    s3::upload_file_to_s3(
        key,
        false,
        s3::get_private_bucket()?,
        media_type.clone(),
        temp_path.to_string_lossy().to_string(),
        None,
    )
    .await
    .with_context(|| "Error uploading file to S3")?;

    temp_path
        .close()
        .with_context(|| "Error closing temporary file path")?;

    let auth_headers = keycloak::get_client_credentials()
        .await
        .with_context(|| "Error acquiring Keycloak client credentials")?;

    let _document = &hasura::document::insert_document(
        auth_headers,
        tenant_id.clone(),
        Some(election_event_id),
        "users-export".to_string(),
        media_type,
        size as i64,
        false,
        Some(document_id),
    )
    .await?
    .data
    .ok_or(anyhow!("Missing data in document insertion response"))?
    .insert_sequent_backend_document
    .ok_or(anyhow!("Missing document in insertion response"))?
    .returning[0];

    Ok(())
}
