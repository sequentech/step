// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres;
use crate::services::documents;
use crate::services::import::import_tenant::upsert_tenant;
use crate::tasks::import_tenant_config::ImportOptions;
use crate::types::documents::EDocuments;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use deadpool_postgres::Transaction;
use std::fs::File;
use std::io;
use std::io::{Cursor, Read, Seek};
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use zip::read::ZipArchive;

pub async fn import_keycloak_config_file(
    hasura_transaction: &Transaction<'_>,
    object: ImportOptions,
    tenant_id: &str,
) -> Result<()> {
    Ok(())
}
pub async fn import_roles_config_file(
    hasura_transaction: &Transaction<'_>,
    object: ImportOptions,
    tenant_id: &str,
) -> Result<()> {
    Ok(())
}

pub async fn import_tenant_config_zip(
    hasura_transaction: &Transaction<'_>,
    object: ImportOptions,
    tenant_id: &str,
    document_id: &str,
) -> Result<()> {
    // Import Document
    let document =
        postgres::document::get_document(hasura_transaction, &tenant_id, None, &document_id)
            .await?
            .ok_or(anyhow!(
                "Error trying to get document id {}: not found",
                &document_id
            ))?;

    let temp_zip_file = documents::get_document_as_temp_file(&tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))
        .unwrap();

    // Iterate over zip files
    let zip_entries = get_zip_entries(temp_zip_file)
        .await
        .context("Failed to get zip entries")?;

    // Zip file processing
    for (file_name, mut file_contents) in zip_entries {
        info!("Importing file: {:?}", file_name);

        let mut cursor = Cursor::new(&mut file_contents[..]);

        if file_name.contains(&format!("{}", EDocuments::TENANT_CONFIG.to_file_name())) {
            let mut temp_file =
                NamedTempFile::new().context("Failed to create tenant temporary file")?;

            io::copy(&mut cursor, &mut temp_file)
                .context("Failed to copy contents of tenant to temporary file")?;
            temp_file.as_file_mut().rewind()?;

            upsert_tenant(hasura_transaction, &tenant_id.clone(), temp_file)
                .await
                .with_context(|| "Failed to upsert tenant")?;
        }
    }

    Ok(())
}

#[instrument(err, skip(temp_file_path))]
pub async fn get_zip_entries(temp_file_path: NamedTempFile) -> Result<Vec<(String, Vec<u8>)>> {
    let zip_file = File::open(&temp_file_path).map_err(|e| anyhow!("File open error: {}", e))?;
    let mut zip = ZipArchive::new(zip_file).map_err(|e| anyhow!("Zip archive error: {}", e))?;
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..zip.len() {
        let mut file = zip
            .by_index(i)
            .map_err(|e| anyhow!("Zip entry error: {}", e))?;
        let file_name = file.name().to_string();
        let mut file_contents = Vec::new();
        file.read_to_end(&mut file_contents)
            .map_err(|e| anyhow!("File read error: {}", e))?;
        entries.push((file_name, file_contents));
    }

    Ok(entries)
}
