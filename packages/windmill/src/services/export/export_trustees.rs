// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::export_election_event::generate_encrypted_zip;
use crate::postgres::election::get_elections;
use crate::postgres::trustee::get_all_trustees;
use crate::services::documents::upload_and_return_document_postgres;
use crate::services::protocol_manager::{
    get_election_board, get_event_board, get_protocol_manager_secret_path,
};
use crate::services::vault;
use crate::services::{
    ceremonies::keys_ceremony::get_keys_ceremony_board, protocol_manager::get_b3_pgsql_client,
    temp_path::generate_temp_file,
};
use crate::{postgres::keys_ceremony::get_keys_ceremonies, util::aws::get_max_upload_size};
use anyhow::{anyhow, Context, Result};
use b3::client::pgsql::B3MessageRow;
use base64::engine::general_purpose;
use base64::Engine;
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::future::try_join_all;
use regex::Regex;
use sequent_core::types::hasura::core::TasksExecution;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};
use zip::write::FileOptions;

#[instrument(err, skip(transaction))]
pub async fn read_trustees_config(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    document_id: &str,
    encryption_password: &str,
    task_execution: &TasksExecution,
) -> Result<()> {
    // Temporary file path for the ZIP archive
    let zip_filename = "export-trustees.zip".to_string();
    let zip_path = env::temp_dir().join(&zip_filename);

    // Create a new ZIP file
    let zip_file = File::create(&zip_path)?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::DEFLATE);

    let trustees = get_all_trustees(transaction, tenant_id).await?;

    for trustee in trustees {
        let trustee_name = trustee.name.clone().unwrap_or_default();
        let secret = vault::read_secret(format!("{}_config", trustee_name))
            .await?
            .unwrap_or_default();

        let data_bytes = secret.into_bytes();

        // Create and write the data into a temporary file
        let mut tmp_file = NamedTempFile::new()?;
        tmp_file.write_all(&data_bytes)?;

        let toml_filename = format!("{}/{}.toml", trustee_name, trustee_name);
        zip_writer.start_file(&toml_filename, options)?;
        std::io::copy(&mut tmp_file, &mut zip_writer)?;
    }
    // Finalize the ZIP file
    zip_writer.finish()?;

    // Encrypt ZIP file if required
    let encrypted_zip_path = zip_path.with_extension("ezip");

    generate_encrypted_zip(
        zip_path.to_string_lossy().to_string(),
        encrypted_zip_path.to_string_lossy().to_string(),
        encryption_password.to_string(),
    )
    .await?;

    let zip_size = std::fs::metadata(&encrypted_zip_path)?.len();

    // Upload the ZIP file (encrypted or original) to Hasura
    let document = upload_and_return_document_postgres(
        &transaction,
        encrypted_zip_path.to_str().unwrap(),
        zip_size,
        "application/zip",
        &tenant_id.to_string(),
        None,
        &zip_filename,
        Some(document_id.to_string()),
        false,
    )
    .await?;

    // Clean up the ZIP files (optional)
    std::fs::remove_file(&zip_path)?;

    Ok(())
}
