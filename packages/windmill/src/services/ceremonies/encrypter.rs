use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::reports::{get_report_by_type, ReportType};
use crate::services::database::get_hasura_pool;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_secret_key;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::instrument;
use walkdir::{DirEntry, WalkDir};

#[instrument(err, skip_all)]
pub async fn traversal_encrypt_files(
    folder_path: &Path,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    if !folder_path.is_dir() {
        return Err(anyhow::anyhow!("Provided path is not a directory"));
    }

    let entries = WalkDir::new(folder_path)
        .into_iter()
        .filter_map(|e| e.ok()); // Collect entries lazily

    for entry in entries {
        let path = entry.path();

        // If it's a file, process it
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                println!("Processing file: {:?}", file_name);
                if file_name.contains("vote_receipts") {
                    encrypt_directory_contents(
                        tenant_id,
                        election_event_id,
                        ReportType::VOTE_RECEIPT,
                        &path.to_string_lossy().to_string(),
                    )
                    .await
                    .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;
                }
            }
        }
    }

    Ok(())
}

/// Encrypt all files in a directory
#[instrument(err, skip_all)]
pub async fn encrypt_directory_contents(
    tenant_id: &str,
    election_event_id: &str,
    report_type: ReportType,
    old_path: &str,
) -> Result<String> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    let report = get_report_by_type(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &report_type.to_string(),
    )
    .await
    .map_err(|err| anyhow!("Error getting report: {err:?}"))?;

    let mut upload_path = old_path.to_string();
    if let Some(report) = &report {
        if report.encryption_policy == EReportEncryption::ConfiguredPassword {
            let secret_key =
                get_report_secret_key(tenant_id, election_event_id, Some(report.id.clone()));

            let encryption_password = vault::read_secret(secret_key.clone())
                .await?
                .ok_or_else(|| anyhow!("Encryption password not found"))?;

            // Modify the path to add the `.enc` suffix
            let new_path = format!("{}.enc", old_path);

            encrypt_file_aes_256_cbc(old_path, &new_path, &encryption_password)
                .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

            upload_path = new_path;
        }
    }

    Ok(upload_path)
}
