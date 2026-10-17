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

pub const MC_BALLOT_IMAGES_FILE_NAME: &str = "mcballots_images";
pub const BALLOT_IMAGES_FILE_NAME: &str = "ballot_images";
pub const INITIALIZATION_REPORT_FILE_NAME: &str = "INITIALIZATION_REPORT";
pub const ELECTORAL_RESULTS_FILE_NAME: &str = "ELECTORAL_RESULTS";

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

// returns a map from the report id to the secret password
#[instrument(err, skip_all)]
pub async fn traversal_find_secrets_for_files(
    hasura_transaction: &Transaction<'_>,
    folder_path: &Path,
    tenant_id: &str,
    election_event_id: &str,
    all_reports: &Vec<Report>,
) -> Result<HashMap<String, String>> {
    let mut report_secrets_map: HashMap<String, String> = HashMap::new();

    if !folder_path.is_dir() {
        return Err(anyhow!("The provided path is not a directory"));
    }

    let entries = WalkDir::new(folder_path).into_iter().filter_map(|e| e.ok());
    let election_id_regex =
        Regex::new(r"election__[a-zA-Z0-9\s\-\_]*__([0-9a-fA-F\-]{36})").unwrap();

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
                                    election_ids.contains(&election_id)
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

/// Encrypts all eligible files in a directory
#[instrument(err, skip_all)]
pub async fn traversal_encrypt_files(
    report_secrets_map: HashMap<String, String>,
    folder_path: &Path,
    all_reports: &Vec<Report>,
) -> Result<()> {
    if !folder_path.is_dir() {
        return Err(anyhow!("The provided path is not a directory"));
    }

    let entries = WalkDir::new(folder_path).into_iter().filter_map(|e| e.ok());
    let election_id_regex =
        Regex::new(r"election__[a-zA-Z0-9\s\-\_]*__([0-9a-fA-F\-]{36})").unwrap();

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

/// The report of `report_type` that sets the encryption policy for the
/// documents of `election_id`: the election's own report or, when it has
/// none, a report that is not tied to any election.
pub fn find_report_for_election<'a>(
    all_reports: &'a [Report],
    report_type: &ReportType,
    election_id: &str,
) -> Option<&'a Report> {
    let report_type = report_type.to_string();
    let find = |election: Option<&str>| {
        all_reports.iter().find(|report| {
            report.report_type == report_type && report.election_id.as_deref() == election
        })
    };
    find(Some(election_id)).or_else(|| find(None))
}

/// The distinct reports of `report_type` that require a password for any of
/// `election_ids`.
pub fn find_password_reports_for_elections<'a>(
    all_reports: &'a [Report],
    report_type: &ReportType,
    election_ids: &[String],
) -> Vec<&'a Report> {
    let mut reports: Vec<&Report> = Vec::new();
    for election_id in election_ids {
        let Some(report) = find_report_for_election(all_reports, report_type, election_id) else {
            continue;
        };
        if report.encryption_policy == EReportEncryption::ConfiguredPassword
            && !reports.iter().any(|known| known.id == report.id)
        {
            reports.push(report);
        }
    }
    reports
}

/// The password for a file that several reports require to be encrypted, or
/// `None` when none does. No single password serves reports whose passwords
/// differ, so that case is an error.
pub fn shared_encryption_password(passwords: Vec<String>) -> Result<Option<String>> {
    let mut passwords = passwords.into_iter();
    let Some(first) = passwords.next() else {
        return Ok(None);
    };
    if passwords.any(|password| password != first) {
        return Err(anyhow!(
            "The reports that cover this file have different passwords"
        ));
    }
    Ok(Some(first))
}

#[instrument(err, skip(hasura_transaction, election_ids, all_reports, old_path))]
pub async fn encrypt_directory_contents_sql(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &[String],
    report_type: ReportType,
    old_path: &str,
    all_reports: &Vec<Report>,
) -> Result<String> {
    let reports = find_password_reports_for_elections(all_reports, &report_type, election_ids);
    info!("Reports: {:?}", reports);

    let mut passwords = Vec::with_capacity(reports.len());
    for report in reports {
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

        passwords.push(encryption_password);
    }

    let encryption_password = shared_encryption_password(passwords).with_context(|| {
        format!("Set the same password on the {report_type} reports of these elections")
    })?;

    match encryption_password {
        Some(encryption_password) => {
            info!("Encrypting file: {:?}", old_path);
            encrypt_file_inner(old_path, &encryption_password)
        }
        None => Ok(old_path.to_string()),
    }
}

#[instrument(err, skip(report_secrets_map, election_ids, all_reports, old_path))]
pub async fn encrypt_directory_contents(
    report_secrets_map: &HashMap<String, String>,
    election_ids: Option<Vec<String>>,
    report_type: ReportType,
    old_path: &str,
    all_reports: &Vec<Report>,
) -> Result<String> {
    let report = all_reports
        .iter()
        .find(|report| {
            report.report_type == report_type.to_string() && {
                if let Some(election_ids) = &election_ids {
                    if let Some(election_id) = &report.election_id {
                        election_ids.contains(&election_id)
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

            let new_path = encrypt_file_inner(old_path, &encryption_password)?;

            new_path
        } else {
            old_path.to_string()
        }
    } else {
        old_path.to_string()
    };

    Ok(upload_path)
}

#[instrument(err, skip_all)]
pub fn encrypt_file_inner(old_path: &str, encryption_password: &str) -> Result<String> {
    let new_path = format!("{}.enc", old_path);

    encrypt_file_aes_256_cbc(old_path, &new_path, encryption_password)
        .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

    std::fs::remove_file(old_path)
        .map_err(|err| anyhow!("Error removing original file: {err:?}"))?;

    return Ok(new_path);
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    const ELECTION_A: &str = "election-a";
    const ELECTION_B: &str = "election-b";
    const ELECTION_C: &str = "election-c";

    fn report(
        id: &str,
        election_id: Option<&str>,
        report_type: ReportType,
        encryption_policy: EReportEncryption,
    ) -> Report {
        Report {
            id: id.to_string(),
            election_event_id: "election-event".to_string(),
            tenant_id: "tenant".to_string(),
            election_id: election_id.map(str::to_string),
            report_type: report_type.to_string(),
            template_alias: None,
            encryption_policy,
            cron_config: None,
            created_at: Utc::now(),
            permission_label: None,
        }
    }

    fn report_ids(reports: Vec<&Report>) -> Vec<String> {
        reports
            .into_iter()
            .map(|report| report.id.clone())
            .collect()
    }

    #[test]
    fn find_report_for_election_uses_the_election_report() {
        let reports = vec![
            report(
                "report-a",
                Some(ELECTION_A),
                ReportType::ELECTORAL_RESULTS,
                EReportEncryption::Unencrypted,
            ),
            report(
                "report-b",
                Some(ELECTION_B),
                ReportType::ELECTORAL_RESULTS,
                EReportEncryption::ConfiguredPassword,
            ),
        ];

        let found_b =
            find_report_for_election(&reports, &ReportType::ELECTORAL_RESULTS, ELECTION_B)
                .map(|report| report.id.as_str());
        let found_a =
            find_report_for_election(&reports, &ReportType::ELECTORAL_RESULTS, ELECTION_A)
                .map(|report| report.id.as_str());

        assert_eq!(found_b, Some("report-b"));
        assert_eq!(found_a, Some("report-a"));
    }

    #[test]
    fn find_report_for_election_falls_back_to_report_without_election() {
        let reports = vec![
            report(
                "report-event",
                None,
                ReportType::ELECTORAL_RESULTS,
                EReportEncryption::ConfiguredPassword,
            ),
            report(
                "report-a",
                Some(ELECTION_A),
                ReportType::ELECTORAL_RESULTS,
                EReportEncryption::Unencrypted,
            ),
        ];

        let found_a =
            find_report_for_election(&reports, &ReportType::ELECTORAL_RESULTS, ELECTION_A)
                .map(|report| report.id.as_str());
        let found_c =
            find_report_for_election(&reports, &ReportType::ELECTORAL_RESULTS, ELECTION_C)
                .map(|report| report.id.as_str());

        assert_eq!(found_a, Some("report-a"));
        assert_eq!(found_c, Some("report-event"));
    }

    #[test]
    fn find_report_for_election_returns_none_without_a_matching_report() {
        let reports = vec![
            report(
                "report-a",
                Some(ELECTION_A),
                ReportType::ELECTORAL_RESULTS,
                EReportEncryption::ConfiguredPassword,
            ),
            report(
                "ballot-images-b",
                Some(ELECTION_B),
                ReportType::BALLOT_IMAGES,
                EReportEncryption::ConfiguredPassword,
            ),
        ];

        let found_b =
            find_report_for_election(&reports, &ReportType::ELECTORAL_RESULTS, ELECTION_B);

        assert!(found_b.is_none());
    }

    #[test]
    fn password_reports_cover_every_election_in_any_order() {
        let unencrypted_a = report(
            "report-a",
            Some(ELECTION_A),
            ReportType::ELECTORAL_RESULTS,
            EReportEncryption::Unencrypted,
        );
        let password_b = report(
            "report-b",
            Some(ELECTION_B),
            ReportType::ELECTORAL_RESULTS,
            EReportEncryption::ConfiguredPassword,
        );
        let election_ids = vec![ELECTION_A.to_string(), ELECTION_B.to_string()];

        for reports in [
            vec![unencrypted_a.clone(), password_b.clone()],
            vec![password_b.clone(), unencrypted_a.clone()],
        ] {
            let found = find_password_reports_for_elections(
                &reports,
                &ReportType::ELECTORAL_RESULTS,
                &election_ids,
            );
            assert_eq!(report_ids(found), vec!["report-b".to_string()]);
        }
    }

    #[test]
    fn password_reports_list_each_report_once() {
        let reports = vec![
            report(
                "report-event",
                None,
                ReportType::INITIALIZATION_REPORT,
                EReportEncryption::ConfiguredPassword,
            ),
            report(
                "report-b",
                Some(ELECTION_B),
                ReportType::INITIALIZATION_REPORT,
                EReportEncryption::ConfiguredPassword,
            ),
        ];
        let election_ids = vec![
            ELECTION_A.to_string(),
            ELECTION_B.to_string(),
            ELECTION_C.to_string(),
        ];

        let found = find_password_reports_for_elections(
            &reports,
            &ReportType::INITIALIZATION_REPORT,
            &election_ids,
        );

        assert_eq!(
            report_ids(found),
            vec!["report-event".to_string(), "report-b".to_string()]
        );
    }

    #[test]
    fn shared_encryption_password_requires_one_password() {
        assert_eq!(shared_encryption_password(vec![]).ok(), Some(None));
        assert_eq!(
            shared_encryption_password(vec!["one".to_string(), "one".to_string()]).ok(),
            Some(Some("one".to_string()))
        );
        assert!(shared_encryption_password(vec!["one".to_string(), "two".to_string()]).is_err());
    }
}
