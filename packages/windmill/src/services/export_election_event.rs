// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::get_reports_by_election_event_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::export_election_event_logs;
use crate::services::import_election_event::ImportElectionEventSchema;
use crate::services::s3;
use crate::tasks::export_election_event::ExportOptions;
use crate::types::documents::EDocuments;

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

use super::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use super::documents::upload_and_return_document_postgres;
use super::export_schedule_events;
use super::export_users::export_users_file;
use super::export_users::ExportBody;
use super::password;

pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ImportElectionEventSchema> {
    let client = KeycloakAdminClient::new().await?;
    let other_client = KeycloakAdminClient::pub_new().await?;
    let board_name = get_event_realm(tenant_id, election_event_id);
    let realm = client.get_realm(&other_client, &board_name).await?;
    let (
        election_event,
        elections,
        contests,
        candidates,
        areas,
        area_contests,
        scheduled_events,
        reports,
    ) = try_join!(
        get_election_event_by_id(&transaction, tenant_id, election_event_id),
        export_elections(&transaction, tenant_id, election_event_id),
        export_contests(&transaction, tenant_id, election_event_id),
        export_candidates(&transaction, tenant_id, election_event_id),
        get_event_areas(&transaction, tenant_id, election_event_id),
        export_area_contests(&transaction, tenant_id, election_event_id),
        find_scheduled_event_by_election_event_id(&transaction, tenant_id, election_event_id),
        get_reports_by_election_event_id(&transaction, tenant_id, election_event_id)
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
        scheduled_events: scheduled_events,
        reports: reports,
    })
}

async fn generate_encrypted_zip(
    temp_path_string: String,
    encrypted_temp_file_string: String,
    password: String,
) -> Result<()> {
    encrypt_file_aes_256_cbc(&temp_path_string, &encrypted_temp_file_string, &password)
        .map_err(|e| anyhow!("Failed encrypting the ZIP file"))?;

    Ok(())
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
        FileOptions::default().compression_method(zip::CompressionMethod::DEFLATE);

    // Add election event data file to the ZIP archive
    let export_data = read_export_data(&hasura_transaction, tenant_id, election_event_id).await?;
    let temp_election_event_file = write_export_document(export_data).await?;
    let election_event_filename = format!(
        "{}-{}.json",
        EDocuments::ELECTION_EVENT.to_file_name(),
        election_event_id
    );
    zip_writer.start_file(&election_event_filename, options)?;

    let mut election_event_file = File::open(temp_election_event_file.path())?;
    std::io::copy(&mut election_event_file, &mut zip_writer)?;

    // Add voters data file to the ZIP archive if required
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
        .await
        .map_err(|e| anyhow!("Error exporting users file: {e:?}"))?;
        let voters_filename = format!(
            "{}-{}.csv",
            EDocuments::VOTERS.to_file_name(),
            election_event_id
        );
        zip_writer.start_file(&voters_filename, options)?;

        let mut voters_file = File::open(temp_voters_file_path)?;
        std::io::copy(&mut voters_file, &mut zip_writer)?;
    }

    // Add Activity Logs data file to the ZIP archive
    let is_include_activity_logs = export_config.activity_logs;
    if is_include_activity_logs {
        let activity_logs_filename = format!(
            "{}-{}.csv",
            EDocuments::ACTIVITY_LOGS.to_file_name(),
            election_event_id
        );
        let temp_activity_logs_file = export_election_event_logs::read_export_data(
            tenant_id,
            election_event_id,
            &activity_logs_filename,
        )
        .await
        .map_err(|e| anyhow!("Error reading activity logs data: {e:?}"))?;
        zip_writer.start_file(&activity_logs_filename, options)?;

        let mut activity_logs_file = File::open(temp_activity_logs_file.path())?;
        std::io::copy(&mut activity_logs_file, &mut zip_writer)?;
    }

    // Add the S3 files to the ZIP archive
    let is_include_s3_files = export_config.s3_files;
    if is_include_s3_files {
        let s3_folder_name = format!("{}", EDocuments::S3_FILES.to_file_name());
        let documents_prefix = format!("tenant-{}/event-{}/", tenant_id, election_event_id);
        let bucket = s3::get_private_bucket()?;

        let s3_files = s3::get_files_from_s3(bucket, documents_prefix)
            .await
            .map_err(|err| anyhow!("Error retrieving files from S3: {err:?}"))?;
        let mut file_counter = 1;

        for file_path in s3_files {
            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
            let file_name_in_zip = format!("{}/{}-{}", s3_folder_name, file_counter, file_name);
            zip_writer.start_file(&file_name_in_zip, options)?;

            let mut s3_file = File::open(&file_path)?;
            std::io::copy(&mut s3_file, &mut zip_writer)?;

            file_counter += 1;
        }
    }

    // Add Activity Logs data file to the ZIP archive
    let is_include_schedule_events = export_config.scheduled_events;
    if is_include_schedule_events {
        let schedule_events_filename = format!(
            "{}-{}.csv",
            EDocuments::SCHEDULED_EVENTS.to_file_name(),
            election_event_id
        );
        let temp_schedule_events_file = export_schedule_events::read_export_data(
            &hasura_transaction,
            tenant_id,
            election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error reading activity logs data: {e:?}"))?;
        zip_writer.start_file(&schedule_events_filename, options)?;

        let mut schedule_events_file = File::open(temp_schedule_events_file)?;
        std::io::copy(&mut schedule_events_file, &mut zip_writer)?;
    }

    // Finalize the ZIP file
    zip_writer.finish()?;

    // Encrypt ZIP file if required
    let encryption_password = export_config.password.unwrap_or("".to_string());
    let encrypted_zip_path = zip_path.with_extension("ezip");
    if encryption_password.len() > 0 {
        generate_encrypted_zip(
            zip_path.to_string_lossy().to_string(),
            encrypted_zip_path.to_string_lossy().to_string(),
            encryption_password.clone(),
        )
        .await?;
    }

    // Use encrypted_zip_path if encryption is enabled, otherwise use zip_path
    let upload_path = if encryption_password.len() > 0 && encrypted_zip_path.exists() {
        &encrypted_zip_path
    } else {
        &zip_path
    };

    let zip_size = std::fs::metadata(&upload_path)?.len();

    // Upload the ZIP file (encrypted or original) to Hasura
    let document = upload_and_return_document_postgres(
        &hasura_transaction,
        upload_path.to_str().unwrap(),
        zip_size,
        "application/zip",
        &tenant_id.to_string(),
        Some(election_event_id.to_string()),
        &zip_filename,
        Some(document_id.to_string()),
        false,
    )
    .await?;

    // Clean up the ZIP files (optional)
    std::fs::remove_file(&zip_path)?;
    if encrypted_zip_path.exists() {
        std::fs::remove_file(&encrypted_zip_path)?;
    }

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
