// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::{services::keycloak::get_event_realm, types::hasura::core::Document};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fs::File;
use std::io::{Write};
use uuid::Uuid;
use zip::write::FileOptions;
use tempfile::NamedTempFile;

use super::{
    documents::upload_and_return_document_postgres, temp_path::write_into_named_temp_file,
};

pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ImportElectionEventSchema> {
    let client = KeycloakAdminClient::new().await?;
    let other_client = KeycloakAdminClient::pub_new().await?;
    let board_name = get_event_realm(tenant_id, election_event_id);
    let realm = client.get_realm(&other_client, &board_name).await?;
    let (election_event, elections, contests, candidates, areas, area_contests) = try_join!(
        get_election_event_by_id(&transaction, tenant_id, election_event_id),
        export_elections(&transaction, tenant_id, election_event_id),
        export_contests(&transaction, tenant_id, election_event_id),
        export_candidates(&transaction, tenant_id, election_event_id),
        get_event_areas(&transaction, tenant_id, election_event_id),
        export_area_contests(&transaction, tenant_id, election_event_id),
    )?;

    Ok(ImportElectionEventSchema {
        tenant_id: Uuid::parse_str(&tenant_id)?,
        keycloak_event_realm: Some(realm),
        election_event: election_event,
        elections: elections,
        contests: contests,
        candidates: candidates,
        areas: areas,
        area_contests: area_contests,
    })
}

pub async fn write_export_document(
    data: ImportElectionEventSchema,
) -> Result<NamedTempFile> {
    // Serialize the data into JSON string
    let data_str = serde_json::to_string(&data)?;
    let data_bytes = data_str.into_bytes();

    // Create a temporary file
    let mut tmp_file = NamedTempFile::new()?;

    // Write the JSON data into the temporary file
    tmp_file.write_all(&data_bytes)?;

    // Return the temporary file (it's automatically cleaned up when dropped)
    Ok(tmp_file)
}


pub async fn process_export_zip(
    tenant_id: &str,
    election_event_id: &str,
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

    // Temporary file path for the ZIP archive
    let zip_filename = format!("export-election-event-{}.zip", election_event_id);
    let zip_path = std::env::temp_dir().join(&zip_filename);

    // Create a new ZIP file
    let zip_file = File::create(&zip_path)?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);

    // Election event data file
    let export_data = read_export_data(&hasura_transaction, tenant_id, election_event_id).await?;
    let temp_file = write_export_document(export_data).await?;

    // Add the temp file to the ZIP
    let options: FileOptions<()> = FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let json_filename = format!("export-election-event-{}.json", election_event_id);
    zip_writer.start_file(&json_filename, options)?;

    let mut file = File::open(temp_file.path())?;
    std::io::copy(&mut file, &mut zip_writer)?;

    // TODO: Users file

    // Finalize the ZIP file
    zip_writer.finish()?;

    // Get the size of the ZIP file
    let zip_size = std::fs::metadata(&zip_path)?.len();

    // Upload the ZIP file to Hasura
    let document = upload_and_return_document_postgres(
        &hasura_transaction,
        zip_path.to_str().unwrap(),
        zip_size,
        "application/zip",
        &tenant_id.to_string(),
        &election_event_id,
        &zip_filename,
        Some(document_id.to_string()),
        false, // is_public: bool,
    )
    .await?;

    // Clean up the temporary ZIP file (optional)
    std::fs::remove_file(&zip_path)?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
