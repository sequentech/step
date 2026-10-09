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
use sequent_core::util::temp_path::generate_temp_file;
use std::io::{Seek, Write};
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};
use zip::write::FileOptions;

const EXPORT_TRUSTEES_FILENAME: &str = "export-trustees.zip";
const EXPORT_TRUSTEES_TEMP_PREFIX: &str = "export-trustees-";

#[instrument(err, skip(transaction))]
pub async fn read_trustees_config_base(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    document_id: &str,
    encryption_password: &str,
    task_execution: &TasksExecution,
) -> Result<()> {
    let trustees = get_all_trustees(transaction, tenant_id).await?;
    let vault_type = get_vault()?.vault_type();

    let secret_prefix: String = match vault_type {
        VaultManagerType::HashiCorpVault => "".to_string(),
        VaultManagerType::AwsSecretManager => "secrets/".to_string(),
        VaultManagerType::EnvVarMasterSecret => "".to_string(),
    };

    let mut trustee_configs = Vec::new();
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

        trustee_configs.push((trustee_name, secret));
    }

    let encrypted_zip = write_encrypted_trustees_zip(&trustee_configs, encryption_password).await?;

    let zip_size = std::fs::metadata(encrypted_zip.path())?.len();

    // Upload the ZIP file (encrypted or original) to Hasura
    let document = upload_and_return_document(
        &transaction,
        encrypted_zip
            .path()
            .to_str()
            .ok_or(anyhow!("Empty encrypted zip path"))?,
        zip_size,
        "application/zip",
        &tenant_id.to_string(),
        None,
        EXPORT_TRUSTEES_FILENAME,
        Some(document_id.to_string()),
        false,
    )
    .await?;

    Ok(())
}

async fn write_encrypted_trustees_zip(
    trustee_configs: &[(String, String)],
    encryption_password: &str,
) -> Result<NamedTempFile> {
    let zip_file = generate_temp_file(EXPORT_TRUSTEES_TEMP_PREFIX, ".zip")?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::DEFLATE);

    for (trustee_name, config) in trustee_configs {
        let toml_filename = format!("{}/{}.toml", trustee_name, trustee_name);
        zip_writer.start_file(&toml_filename, options)?;
        zip_writer.write_all(config.as_bytes())?;
    }
    // Finalize the ZIP file
    let zip_file = zip_writer.finish()?;

    let encrypted_zip = generate_temp_file(EXPORT_TRUSTEES_TEMP_PREFIX, ".ezip")?;

    generate_encrypted_zip(
        zip_file.path().to_string_lossy().to_string(),
        encrypted_zip.path().to_string_lossy().to_string(),
        encryption_password.to_string(),
    )
    .await?;

    Ok(encrypted_zip)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::consolidation::aes_256_cbc_encrypt::decrypt_file_aes_256_cbc;
    use std::io::Read;
    use std::path::Path;

    fn read_encrypted_trustees_zip(path: &Path, password: &str) -> Result<Vec<(String, String)>> {
        let decrypted = NamedTempFile::new()?;
        decrypt_file_aes_256_cbc(
            &path.to_string_lossy(),
            &decrypted.path().to_string_lossy(),
            password,
        )?;
        let mut archive = zip::ZipArchive::new(decrypted.reopen()?)?;
        let mut entries = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            entries.push((entry.name().to_string(), content));
        }
        Ok(entries)
    }

    fn trustee_config(name: &str, config: &str) -> Vec<(String, String)> {
        vec![(name.to_string(), config.to_string())]
    }

    #[tokio::test]
    async fn each_trustee_export_keeps_its_own_archive() -> Result<()> {
        let first =
            write_encrypted_trustees_zip(&trustee_config("trustee1", "first"), "first-password")
                .await?;
        let second =
            write_encrypted_trustees_zip(&trustee_config("trustee2", "second"), "second-password")
                .await?;

        assert_eq!(
            read_encrypted_trustees_zip(first.path(), "first-password")?,
            vec![("trustee1/trustee1.toml".to_string(), "first".to_string())]
        );
        assert_eq!(
            read_encrypted_trustees_zip(second.path(), "second-password")?,
            vec![("trustee2/trustee2.toml".to_string(), "second".to_string())]
        );
        Ok(())
    }

    #[tokio::test]
    async fn trustee_export_archive_is_removed_when_dropped() -> Result<()> {
        let encrypted =
            write_encrypted_trustees_zip(&trustee_config("trustee1", "config"), "password").await?;
        let path = encrypted.path().to_path_buf();
        assert!(path.exists());

        drop(encrypted);

        assert!(!path.exists());
        Ok(())
    }
}
