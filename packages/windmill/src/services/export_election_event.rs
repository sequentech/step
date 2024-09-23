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
use crate::services::export_election_event_logs;
use crate::services::import_election_event::ImportElectionEventSchema;
use crate::tasks::export_election_event::ExportOptions;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use std::env;
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;
use uuid::Uuid;
use zip::write::FileOptions;

use super::documents::upload_and_return_document_postgres;
use super::export_users::export_users_file;
use super::export_users::ExportBody;

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

pub async fn write_export_document(data: ImportElectionEventSchema) -> Result<NamedTempFile> {
    // Serialize the data into JSON string
    let data_str = serde_json::to_string(&data)?;
    let data_bytes = data_str.into_bytes();

    // Create and write the data into a temporary file
    let mut tmp_file = NamedTempFile::new()?;
    tmp_file.write_all(&data_bytes)?;

    Ok(tmp_file)
}

pub async fn process_export_zip(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    export_config: ExportOptions,
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
    let zip_path = env::temp_dir().join(&zip_filename);

    // Create a new ZIP file
    let zip_file = File::create(&zip_path)?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add election event data file to the ZIP archive
    let export_data = read_export_data(&hasura_transaction, tenant_id, election_event_id).await?;
    let temp_election_event_file = write_export_document(export_data).await?;
    let election_event_filename = format!("export-election-event-{}.json", election_event_id);
    zip_writer.start_file(&election_event_filename, options)?;

    let mut election_event_file = File::open(temp_election_event_file.path())?;
    std::io::copy(&mut election_event_file, &mut zip_writer)?;

    // Add voters data file to the ZIP archive
    let is_include_voters = export_config.include_voters;
    if is_include_voters {
        let temp_voters_file_path = export_users_file(
            &hasura_transaction,
            ExportBody::Users {
                tenant_id: tenant_id.to_string(),
                election_event_id: Some(election_event_id.to_string()),
                election_id: None,
            },
        )
        .await?;
        let voters_filename = format!("export-voters-{}.csv", election_event_id);
        zip_writer.start_file(&voters_filename, options)?;

        let mut voters_file = File::open(temp_voters_file_path)?;
        std::io::copy(&mut voters_file, &mut zip_writer)?;
    }

    // Add Activity Logs data file to the ZIP archive
    let is_include_activity_logs = export_config.activity_logs;
    if is_include_activity_logs {
        let activity_logs_filename = format!("export-activity_logs-{}.csv", election_event_id);
        let temp_activity_logs_file = export_election_event_logs::read_export_data(
            tenant_id,
            election_event_id,
            &activity_logs_filename,
        )
        .await?;
        zip_writer.start_file(&activity_logs_filename, options)?;

        let mut activity_logs_file = File::open(temp_activity_logs_file.path())?;
        std::io::copy(&mut activity_logs_file, &mut zip_writer)?;
    }

    // Finalize the ZIP file
    zip_writer.finish()?;
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
