// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::reports::{get_report_by_type, Report, ReportType};
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::database::get_hasura_pool;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_secret_key;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::types::ceremonies::TallyType;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::{info, instrument};
use walkdir::WalkDir;

pub const MC_VOTE_RECEIOT_FILE_NAME: &str = "mcballots_receipts";
pub const MC_BALLOT_IMAGES_FILE_NAME: &str = "mcballots_images";
pub const VOTE_RECEIOT_FILE_NAME: &str = "vote_receipts";
pub const BALLOT_IMAGES_FILE_NAME: &str = "ballot_images";
pub const INITIALIZATION_REPORT_FILE_NAME: &str = "INITIALIZATION_REPORT";
pub const ELECTORAL_RESULTS_FILE_NAME: &str = "ELECTORAL_RESULTS";

#[instrument(err, skip_all)]
pub fn get_file_report_type(file_name: &str) -> Result<Option<ReportType>> {
    if file_name.contains(MC_VOTE_RECEIOT_FILE_NAME) || file_name.contains(VOTE_RECEIOT_FILE_NAME) {
        Ok(Some(ReportType::VOTE_RECEIPT))
    } else if file_name.contains(MC_BALLOT_IMAGES_FILE_NAME)
        || file_name.contains(BALLOT_IMAGES_FILE_NAME)
    {
        Ok(Some(ReportType::BALLOT_IMAGES))
    } else if file_name.contains(INITIALIZATION_REPORT_FILE_NAME) {
        Ok(Some(ReportType::INITIALIZATION_REPORT))
    } else if file_name.contains(ELECTORAL_RESULTS_FILE_NAME) {
        Ok(Some(ReportType::ELECTORAL_RESULTS))
    } else {
        Ok(None)
    }
}

/// Encrypts all eligible files in a directory
#[instrument(err, skip_all)]
pub async fn traversal_encrypt_files(
    folder_path: &Path,
    tenant_id: &str,
    election_event_id: &str,
    all_reports: &Vec<Report>,
) -> Result<()> {
    if !folder_path.is_dir() {
        return Err(anyhow!("The provided path is not a directory"));
    }

    let entries = WalkDir::new(folder_path).into_iter().filter_map(|e| e.ok());

    for entry in entries {
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                let report_type =
                    get_file_report_type(file_name).context("Error getting file report type")?;

                if report_type.is_some() {
                    encrypt_directory_contents(
                        tenant_id,
                        election_event_id,
                        report_type.unwrap(),
                        &path.to_string_lossy().to_string(),
                        all_reports,
                    )
                    .await
                    .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;
                }
            }
        }
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn encrypt_directory_contents(
    tenant_id: &str,
    election_event_id: &str,
    report_type: ReportType,
    old_path: &str,
    all_reports: &Vec<Report>,
) -> Result<String> {
    let report = all_reports
        .iter()
        .find(|report| report.report_type == report_type.to_string())
        .map(|el| el.clone());

    println!("*** report: {:?}", report);
    let mut upload_path = old_path.to_string();
    if let Some(report) = report {
        if report.encryption_policy == EReportEncryption::ConfiguredPassword {
            info!("Encrypting file: {:?}", old_path);

            let secret_key =
                get_report_secret_key(tenant_id, election_event_id, Some(report.id.clone()));

            let encryption_password = vault::read_secret(secret_key.clone())
                .await?
                .ok_or_else(|| anyhow!("Encryption password not found"))?;

            let new_path = format!("{}.enc", old_path);

            encrypt_file_aes_256_cbc(old_path, &new_path, &encryption_password)
                .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

            upload_path = new_path;
            std::fs::remove_file(old_path)
                .map_err(|err| anyhow!("Error removing original file: {err:?}"))?;
        }
    }

    Ok(upload_path)
}
