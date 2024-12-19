use crate::postgres::application::get_applications_by_election;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::get_reports_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::reports::activity_log;
use crate::services::reports::activity_log::{ActivityLogsTemplate, ReportFormat};
use crate::services::reports::template_renderer::{
    ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use crate::services::reports_vault::get_password;
use crate::services::s3;
use crate::tasks::export_election_event::ExportOptions;
use crate::types::documents::EDocuments;

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::Election;
use sequent_core::types::hasura::core::KeysCeremony;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;
use zip::write::FileOptions;

use super::export_bulletin_boards;
use super::export_schedule_events;
use super::export_users::export_users_file;
use super::export_users::ExportBody;
use crate::services::documents::upload_and_return_document_postgres;
use crate::services::password;

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

    // TODO: Add tenant data file to the ZIP archive
    // if export_config.include_voters {
    //     let temp_voters_file_path = export_users_file(
    //         &hasura_transaction,
    //         ExportBody::Users {
    //             tenant_id: tenant_id.to_string(),
    //             election_event_id: Some(election_event_id.to_string()),
    //             election_id: None,
    //         },
    //     )
    //     .await
    //     .map_err(|e| anyhow!("Error exporting users file: {e:?}"))?;
    //     let voters_filename = format!(
    //         "{}-{}.csv",
    //         EDocuments::VOTERS.to_file_name(),
    //         election_event_id
    //     );
    //     zip_writer
    //         .start_file(&voters_filename, options)
    //         .map_err(|e| anyhow!("Error starting voters file in ZIP: {e:?}"))?;

    //     let mut voters_file = File::open(temp_voters_file_path)
    //         .map_err(|e| anyhow!("Error opening voters file: {e:?}"))?;
    //     std::io::copy(&mut voters_file, &mut zip_writer)
    //         .map_err(|e| anyhow!("Error copying voters file to ZIP: {e:?}"))?;
    // }

    // TODO: Add users & roles data file to the ZIP archive
    // if export_config.reports {
    //     let reports_filename = format!(
    //         "{}-{}.csv",
    //         EDocuments::REPORTS.to_file_name(),
    //         election_event_id
    //     );
    //     let reports_data =
    //         get_reports_by_election_event_id(&hasura_transaction, tenant_id, election_event_id)
    //             .await
    //             .map_err(|e| anyhow!("Error reading reports data: {e:?}"))?;

    //     zip_writer
    //         .start_file(&reports_filename, options)
    //         .map_err(|e| anyhow!("Error starting reports file in ZIP: {e:?}"))?;

    //     let temp_reports_file = NamedTempFile::new()
    //         .map_err(|e| anyhow!("Error creating temporary reports file: {e:?}"))?;
    //     {
    //         let mut wtr = csv::Writer::from_writer(&temp_reports_file);
    //         wtr.write_record(&[
    //             "ID",
    //             "Election ID",
    //             "Report Type",
    //             "Template Alias",
    //             "Cron Config",
    //             "Encryption Policy",
    //             "Password",
    //         ])
    //         .map_err(|e| anyhow!("Error writing CSV header: {e:?}"))?;
    //         for report in reports_data {
    //             let password = get_password(
    //                 report.tenant_id,
    //                 report.election_event_id,
    //                 Some(report.id.clone()),
    //             )
    //             .await?
    //             .unwrap_or("".to_string());

    //             wtr.write_record(&[
    //                 report.id.to_string(),
    //                 report.election_id.unwrap_or_default().to_string(),
    //                 report.report_type.to_string(),
    //                 report.template_alias.unwrap_or_default().to_string(),
    //                 serde_json::to_string(&report.cron_config)
    //                     .map_err(|e| anyhow!("Error serializing cron config: {e:?}"))?,
    //                 report.encryption_policy.to_string(),
    //                 password,
    //             ])
    //             .map_err(|e| anyhow!("Error writing CSV record: {e:?}"))?;
    //         }
    //         wtr.flush()
    //             .map_err(|e| anyhow!("Error flushing CSV writer: {e:?}"))?;
    //     }
    //     let mut reports_file = File::open(temp_reports_file.path())
    //         .map_err(|e| anyhow!("Error opening temporary reports file: {e:?}"))?;
    //     std::io::copy(&mut reports_file, &mut zip_writer)
    //         .map_err(|e| anyhow!("Error copying reports file to ZIP: {e:?}"))?;
    // }

    // TODO: Add keycloak config data file to the ZIP archive
    // if export_config.activity_logs {
    //     let activity_logs_filename = format!(
    //         "{}-{}",
    //         EDocuments::ACTIVITY_LOGS.to_file_name(),
    //         election_event_id
    //     );

    //     // Create an instance of ActivityLogsTemplate
    //     let activity_logs_template = ActivityLogsTemplate::new(
    //         ReportOrigins {
    //             tenant_id: tenant_id.to_string(),
    //             election_event_id: election_event_id.to_string(),
    //             election_id: None,
    //             template_alias: None,
    //             voter_id: None,
    //             report_origin: ReportOriginatedFrom::ExportFunction,
    //         },
    //         ReportFormat::CSV, // Assuming CSV format for this export
    //     );

    //     // Prepare user data
    //     let user_data = activity_logs_template
    //         .prepare_user_data(&hasura_transaction, &hasura_transaction)
    //         .await
    //         .map_err(|e| anyhow!("Error preparing activity logs data: {e:?}"))?;

    //     // Generate the CSV file using generate_export_data
    //     let temp_activity_logs_file =
    //         activity_log::generate_export_data(&user_data.electoral_log, &activity_logs_filename)
    //             .await
    //             .map_err(|e| anyhow!("Error generating export data: {e:?}"))?;

    //     zip_writer
    //         .start_file(&activity_logs_filename, options)
    //         .map_err(|e| anyhow!("Error starting activity logs file in ZIP: {e:?}"))?;

    //     let mut activity_logs_file = File::open(temp_activity_logs_file.path())
    //         .map_err(|e| anyhow!("Error opening temporary activity logs file: {e:?}"))?;
    //     std::io::copy(&mut activity_logs_file, &mut zip_writer)
    //         .map_err(|e| anyhow!("Error copying activity logs file to ZIP: {e:?}"))?;
    // }

    // Finalize the ZIP file
    zip_writer
        .finish()
        .map_err(|e| anyhow!("Error finalizing ZIP file: {e:?}"))?;

    // Use encrypted_zip_path if encryption is enabled, otherwise use zip_path
    let upload_path = &zip_path;

    let zip_size = std::fs::metadata(&upload_path)
        .map_err(|e| anyhow!("Error getting ZIP file metadata: {e:?}"))?
        .len();

    // Upload the ZIP file (encrypted or original) to Hasura
    let document = upload_and_return_document_postgres(
        &hasura_transaction,
        upload_path.to_str().unwrap(),
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
    std::fs::remove_file(&zip_path).map_err(|e| anyhow!("Error removing ZIP file: {e:?}"))?;

    Ok(())
}
