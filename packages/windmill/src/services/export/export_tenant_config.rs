// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! ZIP bundle export combining tenant CSV, full Keycloak realm JSON, and a roles-to-permissions CSV.

use crate::services::documents::upload_and_return_document;
use crate::services::export::export_tenant;
use crate::types::documents::EDocuments;
use anyhow::{anyhow, Context, Result};
use csv::Writer;
use deadpool_postgres::{Client as DbClient, Transaction};
use keycloak::types::RealmRepresentation;
use sequent_core::services::keycloak::{get_tenant_realm, KeycloakAdminClient};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use zip::write::FileOptions;

/// Serializes a Keycloak [`RealmRepresentation`] to a temporary JSON file.
///
/// # Errors
///
/// Returns an error when serialization or temp file IO fails.
pub fn write_export_keycloak_config(data: RealmRepresentation) -> Result<NamedTempFile> {
    // Serialize the data into JSON string
    let data_str = serde_json::to_string(&data)?;
    let data_bytes = data_str.into_bytes();

    // Create and write the data into a temporary file
    let mut tmp_file = NamedTempFile::new()?;
    tmp_file.write_all(&data_bytes)?;

    Ok(tmp_file)
}

/// Builds a CSV mapping each realm group name to a joined list of realm roles.
///
/// # Errors
///
/// Returns an error when CSV buffering or writing the temp file fails.
pub fn write_export_roles_permissions_config(
    data: RealmRepresentation,
    tenant_id: &str,
) -> Result<NamedTempFile> {
    let headers = vec!["role", "permissions"];

    let mut writer = Writer::from_writer(vec![]);
    writer.write_record(&headers)?;

    // Parse groups and construct the roles and permissions mapping
    let mut roles_and_permissions: HashMap<String, String> = HashMap::new();

    if let Some(groups) = data.groups {
        for group in groups {
            if let (Some(name), Some(realm_roles)) = (&group.name, &group.realm_roles) {
                let permissions = realm_roles.join("|"); // Combine roles into a single string
                roles_and_permissions.insert(name.clone(), permissions);
            }
        }
    }

    for (role, permissions) in roles_and_permissions {
        writer.write_record(&[role, permissions])?;
    }

    let data_bytes = writer
        .into_inner()
        .map_err(|e| anyhow!("Error converting writer into inner: {:?}", e))?;

    let temp_file = NamedTempFile::new()?;
    std::fs::write(temp_file.path(), &data_bytes)
        .with_context(|| "Failed to write roles & permissions into temp file")?;

    Ok(temp_file)
}

/// Assembles tenant CSV, Keycloak realm JSON, and roles/permissions CSV
/// into a deflate ZIP, uploads to the database and s3, then deletes the archive.
///
/// # Errors
///
/// Propagates tenant export failures, Keycloak admin errors,
/// ZIP IO errors, or document upload failures.
#[instrument(err)]
pub async fn process_export_zip(
    tenant_id: &str,
    document_id: &str,
    hasura_transaction: &Transaction<'_>,
) -> Result<()> {
    // Temporary file path for the ZIP archive
    let zip_filename = format!("export-tenant-config-{}.zip", tenant_id);
    let zip_path = env::temp_dir().join(&zip_filename);

    // Create a new ZIP file
    let zip_file =
        File::create(&zip_path).map_err(|e| anyhow!("Error creating ZIP file: {e:?}"))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::DEFLATE);

    // Add tenant data file to the ZIP archive
    let tenant_filename = format!(
        "{}-{}.csv",
        EDocuments::TENANT_CONFIG.to_file_name(),
        tenant_id
    );

    let tenant_data = export_tenant::read_tenant_export_data(hasura_transaction, tenant_id)
        .await
        .map_err(|e| anyhow!("Error reading tenant data: {e:?}"))?;

    zip_writer
        .start_file(&tenant_filename, options)
        .map_err(|e| anyhow!("Error starting tenant file in ZIP: {e:?}"))?;

    let temp_path = export_tenant::write_export_document(
        tenant_data,
        hasura_transaction,
        document_id,
        tenant_id,
    )
    .await
    .map_err(|err| anyhow!("Error exporting tenant: {err}"))?;

    let mut tenant_confug_file = File::open(temp_path)
        .map_err(|e| anyhow!("Error opening temporary tenant config file: {e:?}"))?;
    std::io::copy(&mut tenant_confug_file, &mut zip_writer)
        .map_err(|e| anyhow!("Error copying tenant config file to ZIP: {e:?}"))?;

    // Add keycloak config data file to the ZIP archive
    let keycloak_filename = format!(
        "{}-{}.json",
        EDocuments::KEYCLOAK_CONFIG.to_file_name(),
        tenant_id
    );

    let client = KeycloakAdminClient::new().await?;
    let other_client = KeycloakAdminClient::pub_new().await?;
    let board_name = get_tenant_realm(tenant_id);
    let realm = client.get_realm(&other_client, &board_name).await?;

    zip_writer
        .start_file(&keycloak_filename, options)
        .map_err(|e| anyhow!("Error starting keycloak file in ZIP: {e:?}"))?;

    let temp_path = write_export_keycloak_config(realm.clone())
        .map_err(|e| anyhow!("Error copying keycloak config data to temp file: {e:?}"))?;

    let mut keycloak_config_file = File::open(temp_path)
        .map_err(|e| anyhow!("Error opening temporary keycloak config file: {e:?}"))?;
    std::io::copy(&mut keycloak_config_file, &mut zip_writer)
        .map_err(|e| anyhow!("Error copying keycloak config file to ZIP: {e:?}"))?;

    // Add roles & permissions data file to the ZIP archive
    let roles_permissions_filename = format!(
        "{}-{}.csv",
        EDocuments::ROLES_PERMISSIONS_CONFIG.to_file_name(),
        tenant_id
    );

    zip_writer
        .start_file(&roles_permissions_filename, options)
        .map_err(|e| anyhow!("Error starting roles_permissions file in ZIP: {e:?}"))?;

    let temp_path = write_export_roles_permissions_config(realm, tenant_id).map_err(|e| {
        anyhow!("Error copying roles & permissions config data to temp file: {e:?}")
    })?;

    let mut roles_permissions_file = File::open(temp_path)
        .map_err(|e| anyhow!("Error opening temporary roles & permissions config file: {e:?}"))?;
    std::io::copy(&mut roles_permissions_file, &mut zip_writer)
        .map_err(|e| anyhow!("Error copying roles_permissions config file to ZIP: {e:?}"))?;

    // Finalize the ZIP file
    zip_writer
        .finish()
        .map_err(|e| anyhow!("Error finalizing ZIP file: {e:?}"))?;

    let upload_path = &zip_path;

    let zip_size = std::fs::metadata(upload_path)
        .map_err(|e| anyhow!("Error getting ZIP file metadata: {e:?}"))?
        .len();

    // Upload the ZIP file to Hasura
    let document = upload_and_return_document(
        hasura_transaction,
        upload_path.to_str().ok_or(anyhow!("Empty upload path"))?,
        zip_size,
        "application/zip",
        tenant_id,
        None,
        &zip_filename,
        Some(document_id.to_string()),
        false,
    )
    .await?;

    // Clean up the ZIP file
    std::fs::remove_file(&zip_path).map_err(|e| anyhow!("Error removing ZIP file: {e:?}"))?;

    Ok(())
}
