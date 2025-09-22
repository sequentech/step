// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres;
use crate::services::database::get_hasura_pool;
use crate::services::documents;
use crate::services::import::import_tenant::upsert_tenant;
use crate::services::keycloak::read_roles_config_file;
use crate::tasks::import_tenant_config::ImportOptions;
use crate::types::documents::EDocuments;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use keycloak::types::RealmRepresentation;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};
use std::fs::File;
use std::io;
use std::io::{Cursor, Read, Seek};
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use zip::read::ZipArchive;

pub async fn import_tenant_config_zip(
    import_options: ImportOptions,
    tenant_id: &str,
    document_id: &str,
    sha256: Option<String>,
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

    let document =
        postgres::document::get_document(&hasura_transaction, tenant_id, None, document_id)
            .await?
            .ok_or(anyhow!(
                "Error trying to get document id {}: not found",
                &document_id
            ))?;

    let temp_zip_file = documents::get_document_as_temp_file(tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))?;

    match sha256 {
        Some(hash) if !hash.is_empty() => match integrity_check(&temp_zip_file, hash) {
            Ok(_) => {
                info!("Hash verified !");
            }
            Err(HashFileVerifyError::HashMismatch(input_hash, gen_hash)) => {
                let err_str = format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}");
                return Err(anyhow!(err_str));
            }
            Err(err) => {
                let err_str = format!("Failed to verify the integrity: {err:?}");
                return Err(anyhow!(err_str));
            }
        },
        _ => {
            info!("No hash provided, skipping integrity check");
        }
    }

    // Iterate over zip files
    let zip_entries = get_zip_entries(temp_zip_file)
        .await
        .context("Failed to get zip entries")?;

    // Fetching realm
    let realm_name = get_tenant_realm(tenant_id);
    let keycloak_client = KeycloakAdminClient::new().await?;
    let other_client = KeycloakAdminClient::pub_new().await?;
    let mut realm = keycloak_client
        .get_realm(&other_client, &realm_name)
        .await
        .with_context(|| "Error obtaining realm")?;

    // Zip file processing
    for (file_name, mut file_contents) in zip_entries {
        info!("Importing file: {:?}", file_name);

        let mut cursor = Cursor::new(&mut file_contents[..]);

        // Process and import tenant configurations
        if file_name.contains(&EDocuments::TENANT_CONFIG.to_file_name())
            && import_options.include_tenant == Some(true)
        {
            let temp_file = read_into_tmp_file(&mut cursor)
                .await
                .map_err(|e| anyhow!("Failed create tenant temp file: {e}"))?;

            upsert_tenant(&hasura_transaction, tenant_id, temp_file)
                .await
                .with_context(|| "Failed to upsert tenant")?;
        }

        // Process and import roles & permissions configurations
        if file_name.contains(&EDocuments::ROLES_PERMISSIONS_CONFIG.to_file_name())
            && import_options.include_roles == Some(true)
        {
            let temp_file = read_into_tmp_file(&mut cursor)
                .await
                .map_err(|e| anyhow!("Failed create roles & permissions temp file: {e}"))?;

            read_roles_config_file(temp_file, &realm, tenant_id).await?;
        }
        if file_name.contains(&EDocuments::KEYCLOAK_CONFIG.to_file_name())
            && import_options.include_keycloak.unwrap_or(false)
        {
            info!("Starting Keycloak config import from file: {}", file_name);

            // Convert file contents to a string
            let data_str = String::from_utf8_lossy(cursor.get_ref());
            info!("Keycloak config file contents: {:?}", data_str);
            // Deserialize the JSON into a RealmRepresentation
            let imported_realm: RealmRepresentation =
                deserialize_str(&data_str).with_context(|| {
                    format!("Failed to deserialize Keycloak realm configuration: {file_name}")
                })?;

            // Update only the fields that are present
            let localization = imported_realm.localization_texts.clone();
            if let Some(localization) = imported_realm.localization_texts {
                realm.localization_texts = Some(localization);
            }
            if let Some(display_name) = imported_realm.display_name {
                realm.display_name = Some(display_name);
            }

            info!("Updated realm display_name: {:?}", realm.display_name);
            info!(
                "Updated realm localization_texts: {:?}",
                realm.localization_texts
            );

            // Serialize and upsert the updated realm in Keycloak
            let realm_string = serde_json::to_string(&realm)
                .with_context(|| "Failed to serialize updated realm configuration")?;
            let keycloack_pub_client = KeycloakAdminClient::pub_new().await?;
            let keycloak_client = KeycloakAdminClient::new().await?;
            keycloak_client
                .update_localization_texts_from_import(
                    localization,
                    &keycloack_pub_client,
                    tenant_id,
                )
                .await?;
            keycloak_client
                .upsert_realm(&realm_name, &realm_string, tenant_id, false, None, None)
                .await
                .with_context(|| "Failed to upsert realm configuration in Keycloak")?;
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

        // Skip operating system files or hidden files
        if file_name.starts_with("__MACOSX/")
            || file_name.starts_with(".")
            || file_name.ends_with("/")
        {
            info!("Skipping OS or hidden file: {:?}", file_name);
            continue;
        }

        let mut file_contents = Vec::new();
        file.read_to_end(&mut file_contents)
            .map_err(|e| anyhow!("File read error: {}", e))?;

        entries.push((file_name, file_contents));
    }

    Ok(entries)
}

pub async fn read_into_tmp_file(cursor: &mut Cursor<&mut [u8]>) -> Result<NamedTempFile> {
    let mut temp_file = NamedTempFile::new().context("Failed to create temporary file")?;
    io::copy(cursor, &mut temp_file).context("Failed to copy contents to temporary file")?;
    temp_file.as_file_mut().rewind()?;
    Ok(temp_file)
}
