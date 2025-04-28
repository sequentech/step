use crate::postgres::application::get_applications_by_election;
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::ballot_publication::get_ballot_publication;
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::get_keys_ceremonies;
use crate::postgres::reports::get_reports_by_election_event_id;
use crate::postgres::trustee::get_all_trustees;
use crate::services::database::get_hasura_pool;
use crate::services::export::export_ballot_publication;
use crate::services::import::import_election_event::ImportElectionEventSchema;
use crate::services::reports::activity_log;
use crate::services::reports::activity_log::{ActivityLogsTemplate, ReportFormat};
use crate::services::reports::template_renderer::{
    ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use crate::services::reports_vault::get_password;
use crate::tasks::export_election_event::ExportOptions;
use crate::types::documents::EDocuments;

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::s3;
use sequent_core::types::hasura::core::Election;
use sequent_core::types::hasura::core::KeysCeremony;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use strand::hash::hash_sha256_file;
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;
use zip::write::FileOptions;

use super::export_bulletin_boards;
use super::export_schedule_events;
use super::export_tally;
use super::export_users::export_users_file;
use super::export_users::ExportBody;
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::documents::upload_and_return_document_postgres;
use crate::services::password;

#[instrument(err, skip(transaction))]
pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    export_config: &ExportOptions,
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
        reports,
        keys_ceremonies,
        trustees,
        applications,
    ) = try_join!(
        get_election_event_by_id(&transaction, tenant_id, election_event_id),
        export_elections(&transaction, tenant_id, election_event_id),
        export_contests(&transaction, tenant_id, election_event_id),
        export_candidates(&transaction, tenant_id, election_event_id),
        get_event_areas(&transaction, tenant_id, election_event_id),
        export_area_contests(&transaction, tenant_id, election_event_id),
        get_reports_by_election_event_id(&transaction, tenant_id, election_event_id),
        get_keys_ceremonies(&transaction, tenant_id, election_event_id),
        get_all_trustees(&transaction, tenant_id),
        get_applications_by_election(&transaction, tenant_id, election_event_id, None),
    )?;

    // map keys ceremonies to names
    let trustee_map: HashMap<String, String> = trustees
        .into_iter()
        .map(|trustee| (trustee.id.clone(), trustee.name.clone().unwrap_or_default()))
        .collect();

    let named_keys_ceremonies: Vec<KeysCeremony> = keys_ceremonies
        .into_iter()
        .map(|keys_ceremony| {
            let mut new_ceremony = keys_ceremony.clone();
            new_ceremony.trustee_ids = new_ceremony
                .trustee_ids
                .into_iter()
                .map(|trustee_id| trustee_map.get(&trustee_id).cloned().unwrap_or_default())
                .collect();
            new_ceremony
        })
        .collect();

    let export_elections = if !export_config.bulletin_board {
        elections
            .into_iter()
            .map(|election| Election {
                keys_ceremony_id: None,
                ..election.clone()
            })
            .collect()
    } else {
        elections
    };

    let export_keys_ceremonies = if export_config.bulletin_board {
        named_keys_ceremonies
    } else {
        vec![]
    };

    let export_reports = if export_config.reports {
        reports
    } else {
        vec![]
    };

    let export_applications = if export_config.applications {
        applications
    } else {
        vec![]
    };

    Ok(ImportElectionEventSchema {
        tenant_id: Uuid::parse_str(&tenant_id)?,
        keycloak_event_realm: Some(realm),
        election_event: election_event,
        elections: export_elections,
        contests: contests,
        candidates: candidates,
        areas: areas,
        area_contests: area_contests,
        scheduled_events: None,
        reports: export_reports,
        keys_ceremonies: Some(export_keys_ceremonies),
        applications: Some(export_applications),
    })
}

#[instrument(err)]
pub async fn generate_encrypted_zip(
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

fn get_export_election_event_filename(
    election_event_id: &str,
    file_path: &PathBuf,
    is_encrypted: bool,
) -> Result<String> {
    let election_event_hash: String = hash_sha256_file(file_path)
        .with_context(|| "Error hashing the exported election_event")?
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect();
    let extension = if is_encrypted { "ezip" } else { "zip" };

    Ok(format!(
        "election-event-{election_event_id}-export-sha256-{election_event_hash}.{extension}"
    ))
}

#[instrument(err)]
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
    info!("export_config: {:?}", export_config);
    // Temporary file path for the ZIP archive
    let zip_filename = format!("export-election-event-{election_event_id}.zip");
    let zip_path = env::temp_dir().join(&zip_filename);

    // Create a new ZIP file
    let zip_file =
        File::create(&zip_path).map_err(|e| anyhow!("Error creating ZIP file: {e:?}"))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::DEFLATE);

    // Add election event data file to the ZIP archive
    let export_data = read_export_data(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &export_config,
    )
    .await?;
    let temp_election_event_file = write_export_document(export_data).await?;
    let election_event_filename = format!(
        "{}-{}.json",
        EDocuments::ELECTION_EVENT.to_file_name(),
        election_event_id
    );
    zip_writer
        .start_file(&election_event_filename, options)
        .map_err(|e| anyhow!("Error starting file in ZIP: {e:?}"))?;

    let mut election_event_file = File::open(temp_election_event_file.path())
        .map_err(|e| anyhow!("Error opening election event file: {e:?}"))?;
    std::io::copy(&mut election_event_file, &mut zip_writer)
        .map_err(|e| anyhow!("Error copying election event file to ZIP: {e:?}"))?;

    // Add voters data file to the ZIP archive if required
    if export_config.include_voters {
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
        zip_writer
            .start_file(&voters_filename, options)
            .map_err(|e| anyhow!("Error starting voters file in ZIP: {e:?}"))?;

        let mut voters_file = File::open(temp_voters_file_path)
            .map_err(|e| anyhow!("Error opening voters file: {e:?}"))?;
        std::io::copy(&mut voters_file, &mut zip_writer)
            .map_err(|e| anyhow!("Error copying voters file to ZIP: {e:?}"))?;
    }

    // Add reports data file to the ZIP archive if required
    if export_config.reports {
        let reports_filename = format!(
            "{}-{}.csv",
            EDocuments::REPORTS.to_file_name(),
            election_event_id
        );
        let reports_data =
            get_reports_by_election_event_id(&hasura_transaction, tenant_id, election_event_id)
                .await
                .map_err(|e| anyhow!("Error reading reports data: {e:?}"))?;

        zip_writer
            .start_file(&reports_filename, options)
            .map_err(|e| anyhow!("Error starting reports file in ZIP: {e:?}"))?;

        let temp_reports_file = NamedTempFile::new()
            .map_err(|e| anyhow!("Error creating temporary reports file: {e:?}"))?;
        {
            let mut wtr = csv::Writer::from_writer(&temp_reports_file);
            wtr.write_record(&[
                "ID",
                "Election ID",
                "Report Type",
                "Template Alias",
                "Cron Config",
                "Encryption Policy",
                "Password",
                "Permission Labels",
            ])
            .map_err(|e| anyhow!("Error writing CSV header: {e:?}"))?;
            for report in reports_data {
                let password = get_password(
                    &hasura_transaction,
                    report.tenant_id,
                    report.election_event_id,
                    Some(report.id.clone()),
                )
                .await?
                .unwrap_or("".to_string());

                wtr.write_record(&[
                    report.id.to_string(),
                    report.election_id.unwrap_or_default().to_string(),
                    report.report_type.to_string(),
                    report.template_alias.unwrap_or_default().to_string(),
                    serde_json::to_string(&report.cron_config)
                        .map_err(|e| anyhow!("Error serializing cron config: {e:?}"))?,
                    report.encryption_policy.to_string(),
                    password,
                    report.permission_label.unwrap_or_default().join("|"),
                ])
                .map_err(|e| anyhow!("Error writing CSV record: {e:?}"))?;
            }
            wtr.flush()
                .map_err(|e| anyhow!("Error flushing CSV writer: {e:?}"))?;
        }
        let mut reports_file = File::open(temp_reports_file.path())
            .map_err(|e| anyhow!("Error opening temporary reports file: {e:?}"))?;
        std::io::copy(&mut reports_file, &mut zip_writer)
            .map_err(|e| anyhow!("Error copying reports file to ZIP: {e:?}"))?;
    }

    // Add Activity Logs data file to the ZIP archive
    if export_config.activity_logs {
        let activity_logs_filename = format!(
            "{}-{}",
            EDocuments::ACTIVITY_LOGS.to_file_name(),
            election_event_id
        );

        // Create an instance of ActivityLogsTemplate
        let activity_logs_template = ActivityLogsTemplate::new(
            ReportOrigins {
                tenant_id: tenant_id.to_string(),
                election_event_id: election_event_id.to_string(),
                election_id: None,
                template_alias: None,
                voter_id: None,
                report_origin: ReportOriginatedFrom::ExportFunction,
                executer_username: None,
                tally_session_id: None,
            },
            ReportFormat::CSV, // Assuming CSV format for this export
        );

        // Prepare user data
        let user_data = activity_logs_template
            .prepare_user_data(&hasura_transaction, &hasura_transaction)
            .await
            .map_err(|e| anyhow!("Error preparing activity logs data: {e:?}"))?;

        // Generate the CSV file using generate_export_data
        let temp_activity_logs_file =
            activity_log::generate_export_data(&user_data.electoral_log, &activity_logs_filename)
                .await
                .map_err(|e| anyhow!("Error generating export data: {e:?}"))?;

        zip_writer
            .start_file(&activity_logs_filename, options)
            .map_err(|e| anyhow!("Error starting activity logs file in ZIP: {e:?}"))?;

        let mut activity_logs_file = File::open(temp_activity_logs_file.path())
            .map_err(|e| anyhow!("Error opening temporary activity logs file: {e:?}"))?;
        std::io::copy(&mut activity_logs_file, &mut zip_writer)
            .map_err(|e| anyhow!("Error copying activity logs file to ZIP: {e:?}"))?;
    }

    // Add the S3 files to the ZIP archive
    if export_config.s3_files {
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
            zip_writer
                .start_file(&file_name_in_zip, options)
                .map_err(|e| anyhow!("Error starting S3 file in ZIP: {e:?}"))?;

            let mut s3_file =
                File::open(&file_path).map_err(|e| anyhow!("Error opening S3 file: {e:?}"))?;
            std::io::copy(&mut s3_file, &mut zip_writer)
                .map_err(|e| anyhow!("Error copying S3 file to ZIP: {e:?}"))?;

            file_counter += 1;
        }
    }

    // Add Scheduled Events data file to the ZIP archive
    if export_config.scheduled_events {
        let schedule_events_filename = format!(
            "{}-{}.csv",
            EDocuments::SCHEDULED_EVENTS.to_file_name(),
            election_event_id
        );
        let temp_schedule_events_data = export_schedule_events::read_export_data(
            &hasura_transaction,
            tenant_id,
            election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error reading scheduled events data: {e:?}"))?;

        zip_writer
            .start_file(&schedule_events_filename, options)
            .map_err(|e| anyhow!("Error starting scheduled events file in ZIP: {e:?}"))?;

        let temp_path = export_schedule_events::write_export_document(
            temp_schedule_events_data,
            &hasura_transaction,
            document_id,
            tenant_id,
            election_event_id,
            false,
        )
        .await
        .map_err(|err| anyhow!("Error exporting scheduled events: {err}"))?;

        let mut schedule_events_file = File::open(temp_path)
            .map_err(|e| anyhow!("Error opening temporary scheduled events file: {e:?}"))?;
        std::io::copy(&mut schedule_events_file, &mut zip_writer)
            .map_err(|e| anyhow!("Error copying scheduled events file to ZIP: {e:?}"))?;
    }

    // Add Publications data file to the ZIP archive
    if export_config.publications {
        let publications_filename = format!(
            "{}-{}.json",
            EDocuments::PUBLICATIONS.to_file_name(),
            election_event_id
        );

        zip_writer
            .start_file(&publications_filename, options)
            .map_err(|e| anyhow!("Error starting ballot publications file in ZIP: {e:?}"))?;

        let temp_path = export_ballot_publication::export_ballot_publications(
            &hasura_transaction,
            document_id,
            tenant_id,
            election_event_id,
        )
        .await
        .map_err(|err| anyhow!("Error exporting ballot publications: {err}"))?;

        let mut ballot_publication_file = File::open(temp_path)
            .map_err(|e| anyhow!("Error opening temporary ballot publications file: {e:?}"))?;
        std::io::copy(&mut ballot_publication_file, &mut zip_writer)
            .map_err(|e| anyhow!("Error copying ballot publications file to ZIP: {e:?}"))?;
    }

    // add protocol manager secrets
    if export_config.bulletin_board || export_config.activity_logs || export_config.publications {
        // read protocol manager keys (one per board)
        let protocol_manager_keys_filename = format!(
            "{}-{}.csv",
            EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name(),
            election_event_id
        );

        let temp_protocol_manager_keys_file = export_bulletin_boards::read_protocol_manager_keys(
            &hasura_transaction,
            tenant_id,
            election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error reading protocol manager keys data: {e:?}"))?;
        zip_writer
            .start_file(&protocol_manager_keys_filename, options)
            .map_err(|e| anyhow!("Error starting protocol manager keys file in ZIP: {e:?}"))?;

        let mut protocol_manager_keys_file = File::open(temp_protocol_manager_keys_file)
            .map_err(|e| anyhow!("Error opening temporary protocol manager keys file: {e:?}"))?;
        std::io::copy(&mut protocol_manager_keys_file, &mut zip_writer)
            .map_err(|e| anyhow!("Error copying protocol manager keys file to ZIP: {e:?}"))?;
    }

    // Add boards info
    let keys_ceremonies =
        get_keys_ceremonies(&hasura_transaction, tenant_id, election_event_id).await?;
    if export_config.bulletin_board && keys_ceremonies.len() > 0 {
        // read boards
        let bulletin_boards_filename = format!(
            "{}-{}.csv",
            EDocuments::BULLETIN_BOARDS.to_file_name(),
            election_event_id
        );

        let temp_bulletin_boards_file = export_bulletin_boards::read_election_event_boards(
            &hasura_transaction,
            tenant_id,
            election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error reading bulletin boards data: {e:?}"))?;
        zip_writer
            .start_file(&bulletin_boards_filename, options)
            .map_err(|e| anyhow!("Error starting bulletin boards file in ZIP: {e:?}"))?;

        let mut bulletin_boards_file = File::open(temp_bulletin_boards_file)
            .map_err(|e| anyhow!("Error opening temporary bulletin boards file: {e:?}"))?;
        std::io::copy(&mut bulletin_boards_file, &mut zip_writer)
            .map_err(|e| anyhow!("Error copying bulletin boards file to ZIP: {e:?}"))?;
    }

    if export_config.tally {
        let tally_folder_name = format!("{}", EDocuments::TALLY.to_file_name());

        let tally_data =
            export_tally::read_tally_data(&hasura_transaction, tenant_id, election_event_id)
                .await
                .map_err(|e| anyhow!("Error reading tally data: {e:?}"))?;

        for (file_name, file_path) in tally_data {
            let file_name_in_zip = format!("{}/{}.csv", tally_folder_name, file_name);

            zip_writer
                .start_file(&file_name_in_zip, options)
                .map_err(|e| anyhow!("Error starting tally file in ZIP: {e:?}"))?;

            let mut tally_file = File::open(&file_path)
                .map_err(|e| anyhow!("Error opening {file_name} file: {e:?}"))?;
            std::io::copy(&mut tally_file, &mut zip_writer)
                .map_err(|e| anyhow!("Error copying tally file to ZIP: {e:?}"))?;
        }
    }

    // Finalize the ZIP file
    zip_writer
        .finish()
        .map_err(|e| anyhow!("Error finalizing ZIP file: {e:?}"))?;

    // Encrypt ZIP file if required
    let encryption_password = export_config.password.unwrap_or("".to_string());
    if 0 == encryption_password.len() && (export_config.bulletin_board || export_config.reports) {
        return Err(anyhow!("Bulletin Board requires password"));
    }
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

    let zip_size = std::fs::metadata(&upload_path)
        .map_err(|e| anyhow!("Error getting ZIP file metadata: {e:?}"))?
        .len();

    let export_event_filename = get_export_election_event_filename(
        election_event_id,
        upload_path,
        encryption_password.len() > 0,
    )
    .map_err(|e| anyhow!("Error generating the exported election event filename: {e:?}"))?;

    // Upload the ZIP file (encrypted or original) to Hasura
    let _document = upload_and_return_document_postgres(
        &hasura_transaction,
        upload_path.to_str().unwrap(),
        zip_size,
        "application/zip",
        &tenant_id.to_string(),
        Some(election_event_id.to_string()),
        &export_event_filename,
        Some(document_id.to_string()),
        false,
    )
    .await?;

    // Clean up the ZIP files (optional)
    std::fs::remove_file(&zip_path).map_err(|e| anyhow!("Error removing ZIP file: {e:?}"))?;
    if encrypted_zip_path.exists() {
        std::fs::remove_file(&encrypted_zip_path)
            .map_err(|e| anyhow!("Error removing encrypted ZIP file: {e:?}"))?;
    }

    hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {e:?}"))?;

    Ok(())
}
