// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres;
use crate::services::database::get_hasura_pool;
use crate::services::documents;
use crate::services::import::import_tenant::upsert_tenant;
use crate::tasks::import_tenant_config::ImportOptions;
use crate::types::documents::EDocuments;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use deadpool_postgres::{Client as DbClient, Transaction};
use keycloak::types::GroupRepresentation;
use keycloak::types::RealmRepresentation;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{Cursor, Read, Seek};
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use zip::read::ZipArchive;

pub async fn read_keycloak_config_file(
    hasura_transaction: &Transaction<'_>,
    object: ImportOptions,
    tenant_id: &str,
) -> Result<()> {
    Ok(())
}
pub async fn read_roles_config_file(tenant_id: &str, temp_file: NamedTempFile) -> Result<()> {
    let mut reader = csv::Reader::from_path(temp_file.path())
        .map_err(|e| anyhow!("Error reading roles and permissions config file: {e}"))?;

    let headers = reader
        .headers()
        .map(|headers| headers.clone())
        .map_err(|err| anyhow!("Error reading CSV headers: {err:?}"))?;

    let mut realm_groups = HashMap::new();
    for result in reader.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        let role: String = record
            .get(0)
            .ok_or_else(|| anyhow!("Role not found"))?
            .to_string();
        let permissions_str: String = record
            .get(1)
            .ok_or_else(|| anyhow!("Permissions not found"))?
            .to_string();
        let permissions: Vec<String> = permissions_str.split("|").map(|s| s.to_string()).collect();

        // TODO: Process and import roles & permissions configurations
        let group = GroupRepresentation {
            name: Some(role.clone().to_string()),
            realm_roles: Some(permissions),
            ..Default::default()
        };
        realm_groups.insert(role, group);
    }
    println!("**** {:?}", realm_groups);

    Ok(())
}

pub async fn import_tenant_config_zip(
    import_options: ImportOptions,
    tenant_id: &str,
    document_id: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    // Import Document
    let document =
        postgres::document::get_document(&hasura_transaction, &tenant_id, None, &document_id)
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

    // Fetching realm only if needed
    let mut realm;
    if (import_options.include_keycloak == Some(true) || import_options.include_roles == Some(true))
    {
        let realm_name = get_tenant_realm(&tenant_id);
        let keycloak_client = KeycloakAdminClient::new().await?;
        let other_client = KeycloakAdminClient::pub_new().await?;
        realm = keycloak_client
            .get_realm(&other_client, &realm_name)
            .await
            .with_context(|| "Error obtaining realm")?;
        println!("realm: {:?}", realm);
    }

    // Zip file processing
    for (file_name, mut file_contents) in zip_entries {
        info!("Importing file: {:?}", file_name);

        let mut cursor = Cursor::new(&mut file_contents[..]);

        // Process and import tenant configurations
        if file_name.contains(&format!("{}", EDocuments::TENANT_CONFIG.to_file_name()))
            && import_options.include_tenant == Some(true)
        {
            let mut temp_file =
                NamedTempFile::new().context("Failed to create tenant temporary file")?;
            io::copy(&mut cursor, &mut temp_file)
                .context("Failed to copy contents of tenant to temporary file")?;
            temp_file.as_file_mut().rewind()?;

            upsert_tenant(&hasura_transaction, &tenant_id.clone(), temp_file)
                .await
                .with_context(|| "Failed to upsert tenant")?;
        }

        // Process and import roles & permissions configurations
        if file_name.contains(&format!(
            "{}",
            EDocuments::ROLES_PERMISSIONS_CONFIG.to_file_name()
        )) && import_options.include_roles == Some(true)
        {
            // TODO: move the temp file creation to a separate function
            let mut temp_file =
                NamedTempFile::new().context("Failed to create tenant temporary file")?;
            io::copy(&mut cursor, &mut temp_file)
                .context("Failed to copy contents of tenant to temporary file")?;
            temp_file.as_file_mut().rewind()?;
            // TODO: finish the implementation
            read_roles_config_file(&tenant_id.clone(), temp_file).await?;
        }

        // TODO: Process and import keycloak configurations
        if file_name.contains(&format!("{}", EDocuments::KEYCLOAK_CONFIG.to_file_name()))
            && import_options.include_keycloak == Some(true)
        {
            // TODO
        }
    }

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {e}"));

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
