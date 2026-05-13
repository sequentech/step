// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::reports::{Report, ReportType};
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_secret_key;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::{info, instrument};
use walkdir::WalkDir;

/// File name for multi-contest ballot image exports.
pub const MC_BALLOT_IMAGES_FILE_NAME: &str = "mcballots_images";
/// File name for single-contest ballot image exports.
pub const BALLOT_IMAGES_FILE_NAME: &str = "ballot_images";
/// File name for trustee initialization PDFs.
pub const INITIALIZATION_REPORT_FILE_NAME: &str = "INITIALIZATION_REPORT";
/// File name for aggregated electoral results reports.
pub const ELECTORAL_RESULTS_FILE_NAME: &str = "ELECTORAL_RESULTS";

/// Maps a file name to a [`ReportType`].
///
/// # Errors
///
/// This helper only returns `Ok` variants today; errors are reserved for future filename parsing.
#[instrument(err, skip_all)]
pub fn get_file_report_type(file_name: &str) -> Result<Option<ReportType>> {
    if file_name.contains(MC_BALLOT_IMAGES_FILE_NAME) || file_name.contains(BALLOT_IMAGES_FILE_NAME)
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

/// Walks `folder_path`, finds report-like files, extracts embedded election UUIDs from each path
/// string, and loads configured-password secrets from the vault keyed by matching [`Report`] rows.
///
/// # Panics
///
/// Panics if the static election-id regular expression fails to compile (a programmer error).
///
/// # Errors
///
/// - `Err` when `folder_path` is not a directory.
/// - Vault read failures, missing passwords, or I/O errors while traversing files.
#[instrument(err, skip_all)]
pub async fn traversal_find_secrets_for_files(
    hasura_transaction: &Transaction<'_>,
    folder_path: &Path,
    tenant_id: &str,
    election_event_id: &str,
    all_reports: &[Report],
) -> Result<HashMap<String, String>> {
    let mut report_secrets_map: HashMap<String, String> = HashMap::new();

    if !folder_path.is_dir() {
        return Err(anyhow!("The provided path is not a directory"));
    }

    let entries = WalkDir::new(folder_path)
        .into_iter()
        .filter_map(std::result::Result::ok);
    let election_id_regex = Regex::new(r"election__[a-zA-Z0-9\s\-\_]*__([0-9a-fA-F\-]{36})")
        .expect("static election id regex");

    for entry in entries {
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                let report_type =
                    get_file_report_type(file_name).context("Error getting file report type")?;

                // Use the regex to extract the election_id
                let election_ids = path
                    .to_string_lossy()
                    .lines()
                    .filter_map(|line| {
                        election_id_regex
                            .captures(line)
                            .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                    })
                    .collect::<Vec<String>>();

                if let Some(report_type) = report_type {
                    let report = all_reports
                        .iter()
                        .find(|report| {
                            report.report_type == report_type.to_string() && {
                                if let Some(election_id) = &report.election_id {
                                    election_ids.contains(election_id)
                                } else {
                                    false
                                }
                            }
                        })
                        .cloned();

                    if let Some(report) = report {
                        if report.encryption_policy == EReportEncryption::ConfiguredPassword {
                            let secret_key = get_report_secret_key(
                                tenant_id,
                                election_event_id,
                                Some(report.id.clone()),
                            );

                            let encryption_password = vault::read_secret(
                                hasura_transaction,
                                tenant_id,
                                Some(election_event_id),
                                &secret_key,
                            )
                            .await?
                            .ok_or_else(|| anyhow!("Encryption password not found"))?;

                            report_secrets_map.insert(report.id.clone(), encryption_password);
                        }
                    }
                }
            }
        }
    }

    Ok(report_secrets_map)
}

/// Encrypts every password-protected file under `folder_path` using secrets from
/// `report_secrets_map`.
///
/// # Panics
///
/// Panics if the static election-id regular expression fails to compile (a programmer error).
///
/// # Errors
///
/// - `Err` when `folder_path` is not a directory.
/// - Encryption failures bubbled up from [`encrypt_directory_contents`].
#[allow(clippy::implicit_hasher)]
#[instrument(err, skip_all)]
pub async fn traversal_encrypt_files(
    report_secrets_map: HashMap<String, String>,
    folder_path: &Path,
    all_reports: &[Report],
) -> Result<()> {
    if !folder_path.is_dir() {
        return Err(anyhow!("The provided path is not a directory"));
    }

    let entries = WalkDir::new(folder_path)
        .into_iter()
        .filter_map(std::result::Result::ok);
    let election_id_regex = Regex::new(r"election__[a-zA-Z0-9\s\-\_]*__([0-9a-fA-F\-]{36})")
        .expect("static election id regex");

    for entry in entries {
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                let report_type =
                    get_file_report_type(file_name).context("Error getting file report type")?;

                // Use the regex to extract the election_id
                let election_ids = path
                    .to_string_lossy()
                    .lines()
                    .filter_map(|line| {
                        election_id_regex
                            .captures(line)
                            .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                    })
                    .collect::<Vec<String>>();

                if let Some(report_type) = report_type {
                    encrypt_directory_contents(
                        &report_secrets_map,
                        Some(election_ids),
                        report_type,
                        path.to_string_lossy().as_ref(),
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

/// Encrypts `old_path` when `report_type` matches a report configured with a configured password,
/// returning either the encrypted path or the original path when encryption does not apply.
///
/// # Errors
///
/// - Missing report match, missing vault secret, AES encryption errors, or filesystem errors while
///   replacing the plaintext file.
#[instrument(err, skip(hasura_transaction, election_ids, all_reports, old_path))]
pub async fn encrypt_directory_contents_sql(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: Option<Vec<String>>,
    report_type: ReportType,
    old_path: &str,
    all_reports: &[Report],
) -> Result<String> {
    let report = all_reports
        .iter()
        .find(|report| {
            report.report_type == report_type.to_string() && {
                if let Some(election_ids) = &election_ids {
                    if let Some(election_id) = &report.election_id {
                        election_ids.contains(election_id)
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
        })
        .cloned();
    info!("Report: {:?}", report);

    let upload_path = if let Some(report) = report {
        if report.encryption_policy == EReportEncryption::ConfiguredPassword {
            info!("Encrypting file: {:?}", old_path);

            let secret_key =
                get_report_secret_key(tenant_id, election_event_id, Some(report.id.clone()));

            let encryption_password = vault::read_secret(
                hasura_transaction,
                tenant_id,
                Some(election_event_id),
                &secret_key,
            )
            .await?
            .ok_or_else(|| anyhow!("Encryption password not found"))?;

            encrypt_file_inner(old_path, &encryption_password)?
        } else {
            old_path.to_string()
        }
    } else {
        old_path.to_string()
    };

    Ok(upload_path)
}

/// Same selection rules as [`encrypt_directory_contents_sql`] but reads the password from an
/// in-memory map produced by [`traversal_find_secrets_for_files`].
///
/// # Errors
///
/// - Missing password entry for a matched report, encryption failures, or filesystem errors when
///   removing the plaintext source file.
#[allow(clippy::implicit_hasher)]
#[instrument(err, skip(report_secrets_map, election_ids, all_reports, old_path))]
pub async fn encrypt_directory_contents(
    report_secrets_map: &HashMap<String, String>,
    election_ids: Option<Vec<String>>,
    report_type: ReportType,
    old_path: &str,
    all_reports: &[Report],
) -> Result<String> {
    let report = all_reports
        .iter()
        .find(|report| {
            report.report_type == report_type.to_string() && {
                if let Some(election_ids) = &election_ids {
                    if let Some(election_id) = &report.election_id {
                        election_ids.contains(election_id)
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
        })
        .cloned();

    info!("Report: {:?}", report);

    let upload_path = if let Some(report) = report {
        if report.encryption_policy == EReportEncryption::ConfiguredPassword {
            let encryption_password = report_secrets_map
                .get(&report.id)
                .cloned()
                .ok_or_else(|| anyhow!("Encryption password not found"))?;

            encrypt_file_inner(old_path, &encryption_password)?
        } else {
            old_path.to_string()
        }
    } else {
        old_path.to_string()
    };

    Ok(upload_path)
}

/// Writes `old_path` to `{old_path}.enc` with AES-256-CBC and deletes the plaintext copy.
///
/// # Errors
///
/// Propagates encryption or deletion failures from the consolidation helper and filesystem APIs.
#[instrument(err, skip_all)]
pub fn encrypt_file_inner(old_path: &str, encryption_password: &str) -> Result<String> {
    let new_path = format!("{old_path}.enc");

    encrypt_file_aes_256_cbc(old_path, &new_path, encryption_password)
        .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

    std::fs::remove_file(old_path)
        .map_err(|err| anyhow!("Error removing original file: {err:?}"))?;

    return Ok(new_path);
}

/// Encrypts a single on-disk report when `report` requests a configured password, otherwise returns
/// `old_path` unchanged.
///
/// # Errors
///
/// Vault read failures, missing secrets, or errors from [`encrypt_file_inner`].
#[instrument(err, skip_all)]
pub async fn encrypt_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    old_path: &str,
    report: Option<&Report>,
) -> Result<String> {
    let mut upload_path = old_path.to_string();
    if let Some(report) = report {
        if report.encryption_policy == EReportEncryption::ConfiguredPassword {
            info!("Encrypting file: {:?}", old_path);

            let secret_key =
                get_report_secret_key(tenant_id, election_event_id, Some(report.id.clone()));

            let encryption_password = vault::read_secret(
                hasura_transaction,
                tenant_id,
                Some(election_event_id),
                &secret_key,
            )
            .await?
            .ok_or_else(|| anyhow!("Encryption password not found"))?;

            upload_path = encrypt_file_inner(old_path, &encryption_password)?;
        }
    }

    Ok(upload_path)
}
