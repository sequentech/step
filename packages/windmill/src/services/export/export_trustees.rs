// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::export_election_event::generate_encrypted_zip;
use crate::postgres::trustee::get_all_trustees;
use crate::services::documents::upload_and_return_document;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::services::vault::{self, get_vault, VaultManagerType};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::TasksExecution;
use std::env;
use std::fs::File;
use std::io::{Seek, Write};
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};
use zip::write::FileOptions;

#[instrument(err, skip(transaction))]
pub async fn read_trustees_config_base(
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
    let vault_type = get_vault()?.vault_type();

    let secret_prefix: String = match vault_type {
        VaultManagerType::HashiCorpVault => "".to_string(),
        VaultManagerType::AwsSecretManager => "secrets/".to_string(),
    };

    for trustee in trustees {
        let trustee_name = trustee
            .name
            .clone()
            .ok_or(anyhow!("Missing trustee name"))?;
        let trustee_key = format!("{}{}_config", secret_prefix, trustee_name);
        let secret = vault::read_secret(transaction, tenant_id, None, &trustee_key)
            .await?
            .ok_or(anyhow!(
                "Missing vault secret for '{}'  and key '{}'",
                trustee_name,
                trustee_key
            ))?;
        info!("length of secret for {}: '{}'", trustee_name, secret.len());

        let data_bytes = secret.into_bytes();

        let toml_filename = format!("{}/{}.toml", trustee_name, trustee_name);
        zip_writer.start_file(&toml_filename, options)?;
        zip_writer.write_all(&data_bytes)?;
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
    let document = upload_and_return_document(
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

#[instrument(err, skip(transaction))]
pub async fn read_trustees_config(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    document_id: &str,
    encryption_password: &str,
    task_execution: &TasksExecution,
) -> Result<()> {
    let res = read_trustees_config_base(
        transaction,
        tenant_id,
        document_id,
        encryption_password,
        task_execution,
    )
    .await;

    match res {
        Ok(_) => {
            update_complete(&task_execution, Some(document_id.to_string()))
                .await
                .context("Failed to update task execution status to COMPLETED")?;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Failed reading trustees config: {err:?}");
            update_fail(&task_execution, &err_str).await.context(
                "Failed to update task reading trustees config execution status to FAILED",
            )?;
            Err(err)
        }
    }
}
