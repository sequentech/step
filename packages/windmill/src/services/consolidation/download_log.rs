// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The chain of custody of a package received by hand: every download of
//! its EML, results CSV or archive adds a line to the package's log (the
//! file, its SHA-256, who downloaded it and when) and a
//! `TransmissionFileDownloaded` entry to the election event's log.

use super::create_transmission_package_service::{
    file_sha256, update_transmission_package_annotations,
};
use super::package_manifest::{eml_name, ALL_SERVERS_NAME};
use super::results_csv::csv_name;
use super::signed_transmission_package::{lock_transmission_data, transmission_zone};
use crate::postgres::audit_log_outbox::{
    insert_audit_log_outbox, AuditLogTarget, NewAuditLogEntry,
};
use crate::postgres::document::get_document;
use crate::postgres::election_event::get_election_event_by_election_area;
use crate::services::database::get_hasura_pool;
use crate::services::documents::get_document_as_temp_file;
use crate::types::miru_plugin::{MiruDocument, MiruTransmissionPackageData};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::messages::newtypes::AuditStatementKind;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use sequent_core::types::ceremonies::Log;
use sequent_core::util::temp_path::read_temp_file;
use serde_json::json;
use std::fmt;
use strum_macros::Display;
use tracing::instrument;
use uuid::Uuid;

/// The files of a package that can be downloaded for manual receipt.
#[derive(Display, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransmissionFile {
    #[strum(serialize = "EML")]
    Eml,
    #[strum(serialize = "ResultsCsv")]
    ResultsCsv,
    #[strum(serialize = "Package")]
    Package,
}

/// A file of a package as a download names it.
#[derive(Debug, Clone)]
pub struct PackageFile<'a> {
    pub kind: TransmissionFile,
    pub file_name: String,
    /// The document of the package the file belongs to: the latest one that
    /// holds it.
    pub document: &'a MiruDocument,
    /// Its uppercase hex SHA-256, when the package recorded it.
    pub sha256: Option<String>,
}

/// The file of `package` that `document_id` stores, if any.
pub fn package_file<'a>(
    package: &'a MiruTransmissionPackageData,
    document_id: &str,
) -> Option<PackageFile<'a>> {
    package.documents.iter().rev().find_map(|document| {
        let ids = &document.document_ids;
        let hashes = document.sha256.as_ref();
        let (kind, file_name, sha256) = if ids.eml == document_id {
            (
                TransmissionFile::Eml,
                eml_name(&document.transaction_id),
                hashes.map(|hashes| hashes.eml.clone()),
            )
        } else if ids.csv.as_deref() == Some(document_id) {
            (
                TransmissionFile::ResultsCsv,
                csv_name(&document.transaction_id),
                hashes.and_then(|hashes| hashes.csv.clone()),
            )
        } else if ids.all_servers == document_id {
            (
                TransmissionFile::Package,
                ALL_SERVERS_NAME.to_string(),
                hashes.map(|hashes| hashes.all_servers.clone()),
            )
        } else {
            return None;
        };
        Some(PackageFile {
            kind,
            file_name,
            document,
            sha256,
        })
    })
}

/// Who downloaded a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Downloader {
    pub user_id: String,
    pub username: Option<String>,
    /// As the log line names them.
    pub name: String,
}

/// What a download is of.
#[derive(Debug, Clone)]
pub struct TransmissionDownload {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub area_id: String,
    pub tally_session_id: String,
    pub document_id: String,
}

/// The file a download was recorded for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedDownload {
    pub file_name: String,
    pub sha256: String,
}

/// Why a download can't be recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadRefusal {
    /// The tally session has no package for this Post and country, or the
    /// Post is not in the election event.
    NoPackage,
    /// The document is not one of the package's files.
    NotAPackageFile,
}

impl fmt::Display for DownloadRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadRefusal::NoPackage => {
                write!(
                    f,
                    "There is no transmission package for this Post and country."
                )
            }
            DownloadRefusal::NotAPackageFile => {
                write!(
                    f,
                    "The document is not a file of this transmission package."
                )
            }
        }
    }
}

impl std::error::Error for DownloadRefusal {}

/// The package log's line for a download.
pub fn download_log(
    datetime: &DateTime<Tz>,
    file_name: &str,
    sha256: &str,
    downloader: &Downloader,
) -> Log {
    Log {
        created_date: datetime.to_rfc3339(),
        log_text: format!(
            "Downloaded {file_name} (SHA-256 {sha256}) by {}",
            downloader.name
        ),
    }
}

/// The election event log's entry for a download.
pub fn download_entry(
    download: &TransmissionDownload,
    file: &PackageFile<'_>,
    sha256: &str,
    downloader: &Downloader,
) -> Result<NewAuditLogEntry> {
    let uuid = |value: &str, what: &str| {
        Uuid::parse_str(value).with_context(|| format!("Error parsing the {what}"))
    };
    Ok(NewAuditLogEntry {
        target: AuditLogTarget {
            tenant_id: uuid(&download.tenant_id, "tenant id")?,
            election_event_id: Some(uuid(&download.election_event_id, "election event id")?),
        },
        statement_kind: AuditStatementKind::TransmissionFileDownloaded,
        event_type: StatementEventType::USER,
        log_type: StatementLogType::INFO,
        user_id: Some(downloader.user_id.clone()),
        username: downloader.username.clone(),
        election_id: Some(uuid(&download.election_id, "election id")?),
        area_id: Some(uuid(&download.area_id, "area id")?),
        description: format!(
            "Downloaded {} of the ER transmission package (SHA-256 {sha256})",
            file.file_name
        ),
        details: json!({
            "file": file.kind.to_string(),
            "file_name": file.file_name,
            "sha256": sha256,
            "document_id": download.document_id,
            "transaction_id": file.document.transaction_id,
            "tally_session_id": download.tally_session_id,
            "election_id": download.election_id,
            "area_id": download.area_id,
            "downloaded_by": downloader.name,
        }),
    })
}

/// The SHA-256 of a stored document, for files of packages made before
/// their hashes were recorded.
async fn stored_sha256(
    hasura_transaction: &Transaction<'_>,
    download: &TransmissionDownload,
) -> Result<String> {
    let document = get_document(
        hasura_transaction,
        &download.tenant_id,
        Some(download.election_event_id.clone()),
        &download.document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Can't find document {}", download.document_id))?;
    let mut file = get_document_as_temp_file(&download.tenant_id, &document).await?;
    Ok(file_sha256(&read_temp_file(&mut file)?))
}

/// Records a download of a package's file: its log line and its electoral
/// log entry, committed together.
#[instrument(err)]
pub async fn record_transmission_download(
    download: &TransmissionDownload,
    downloader: &Downloader,
) -> Result<std::result::Result<RecordedDownload, DownloadRefusal>> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .context("Error acquiring hasura transaction")?;
    let election_event = get_election_event_by_election_area(
        &hasura_transaction,
        &download.tenant_id,
        &download.election_id,
        &download.area_id,
    )
    .await
    .context("Error fetching election event")?;
    if election_event.id != download.election_event_id {
        return Ok(Err(DownloadRefusal::NoPackage));
    }
    let locked = lock_transmission_data(
        &hasura_transaction,
        &download.tenant_id,
        &download.election_event_id,
        &download.tally_session_id,
    )
    .await?;
    let Some(mut package) = locked
        .packages
        .iter()
        .find(|package| {
            package.area_id == download.area_id && package.election_id == download.election_id
        })
        .cloned()
    else {
        return Ok(Err(DownloadRefusal::NoPackage));
    };
    let Some(file) = package_file(&package, &download.document_id) else {
        return Ok(Err(DownloadRefusal::NotAPackageFile));
    };
    let sha256 = match file.sha256.clone() {
        Some(sha256) => sha256,
        None => stored_sha256(&hasura_transaction, download).await?,
    };
    let entry = download_entry(download, &file, &sha256, downloader)?;
    let recorded = RecordedDownload {
        file_name: file.file_name.clone(),
        sha256: sha256.clone(),
    };

    let now = Utc::now();
    let zone = transmission_zone(
        &hasura_transaction,
        &download.tenant_id,
        &download.election_event_id,
        now,
    )
    .await?;
    package.logs.push(download_log(
        &now.with_timezone(&zone.zone),
        &recorded.file_name,
        &sha256,
        downloader,
    ));
    update_transmission_package_annotations(
        &hasura_transaction,
        &download.tenant_id,
        &download.election_event_id,
        &download.tally_session_id,
        &download.area_id,
        &download.election_id,
        locked.packages.clone(),
        package,
        locked.annotations.clone(),
    )
    .await?;
    insert_audit_log_outbox(&hasura_transaction, &entry).await?;
    hasura_transaction
        .commit()
        .await
        .context("Error committing the download's record")?;
    Ok(Ok(recorded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::miru_plugin::{MiruDocumentHashes, MiruDocumentIds};
    use chrono::TimeZone as _;

    const TENANT: &str = "11111111-1111-4111-8111-111111111111";
    const EVENT: &str = "22222222-2222-4222-8222-222222222222";
    const ELECTION: &str = "33333333-3333-4333-8333-333333333333";
    const AREA: &str = "44444444-4444-4444-8444-444444444444";

    fn document(all_servers: &str, hashes: Option<MiruDocumentHashes>) -> MiruDocument {
        MiruDocument {
            document_ids: MiruDocumentIds {
                eml: "eml".to_string(),
                xz: "xz".to_string(),
                all_servers: all_servers.to_string(),
                csv: hashes.as_ref().map(|_| "csv".to_string()),
            },
            transaction_id: "0000001234".to_string(),
            servers_sent_to: vec![],
            created_at: "2028-05-08T19:00:00+02:00".to_string(),
            signatures: vec![],
            sha256: hashes,
        }
    }

    fn hashes(all_servers: &str) -> MiruDocumentHashes {
        MiruDocumentHashes {
            eml: "E8D67AC4".to_string(),
            csv: Some("C5V".to_string()),
            all_servers: all_servers.to_string(),
        }
    }

    fn package(documents: Vec<MiruDocument>) -> MiruTransmissionPackageData {
        MiruTransmissionPackageData {
            election_id: ELECTION.to_string(),
            area_id: AREA.to_string(),
            servers: vec![],
            documents,
            logs: vec![],
            threshold: 2,
            signing_request: None,
            ovcs_public_key: None,
        }
    }

    fn downloader() -> Downloader {
        Downloader {
            user_id: "user-1".to_string(),
            username: Some("sbei-madrid-1".to_string()),
            name: "Maria L. Santos".to_string(),
        }
    }

    fn download(document_id: &str) -> TransmissionDownload {
        TransmissionDownload {
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            election_id: ELECTION.to_string(),
            area_id: AREA.to_string(),
            tally_session_id: "session".to_string(),
            document_id: document_id.to_string(),
        }
    }

    #[test]
    fn each_file_of_the_package_is_named_with_its_hash() {
        let package = package(vec![
            document("first", Some(hashes("AAA"))),
            document("signed", Some(hashes("BBB"))),
        ]);
        let eml = package_file(&package, "eml").unwrap();
        assert_eq!(
            (eml.kind, eml.file_name.as_str(), eml.sha256.as_deref()),
            (TransmissionFile::Eml, "er_0000001234.xml", Some("E8D67AC4"))
        );
        let csv = package_file(&package, "csv").unwrap();
        assert_eq!(
            (csv.kind, csv.file_name.as_str(), csv.sha256.as_deref()),
            (
                TransmissionFile::ResultsCsv,
                "er_0000001234.csv",
                Some("C5V")
            )
        );
        // Each archive keeps the hash it was sealed with.
        let first = package_file(&package, "first").unwrap();
        assert_eq!(
            (
                first.kind,
                first.file_name.as_str(),
                first.sha256.as_deref()
            ),
            (TransmissionFile::Package, "all_servers.zip", Some("AAA"))
        );
        assert_eq!(
            package_file(&package, "signed").unwrap().sha256.as_deref(),
            Some("BBB")
        );
        // The EML is the latest document's.
        assert_eq!(
            package_file(&package, "eml")
                .unwrap()
                .document
                .document_ids
                .all_servers,
            "signed"
        );
    }

    #[test]
    fn a_package_made_before_hashes_has_none_and_no_csv() {
        let package = package(vec![document("old", None)]);
        assert_eq!(package_file(&package, "eml").unwrap().sha256, None);
        assert_eq!(package_file(&package, "old").unwrap().sha256, None);
        assert!(package_file(&package, "csv").is_none());
        assert!(package_file(&package, "xz").is_none());
        assert!(package_file(&package, "other").is_none());
    }

    #[test]
    fn the_log_line_names_the_file_its_hash_and_who() {
        let zone: Tz = "Europe/Madrid".parse().unwrap();
        let at = zone.with_ymd_and_hms(2028, 5, 8, 21, 4, 5).unwrap();
        let line = download_log(&at, "all_servers.zip", "E8D67AC4", &downloader());
        assert_eq!(line.created_date, "2028-05-08T21:04:05+02:00");
        assert_eq!(
            line.log_text,
            "Downloaded all_servers.zip (SHA-256 E8D67AC4) by Maria L. Santos"
        );
    }

    #[test]
    fn the_event_log_entry_carries_the_file_hash_and_downloader() {
        let package = package(vec![document("archive", Some(hashes("AAA")))]);
        let file = package_file(&package, "csv").unwrap();
        let entry = download_entry(&download("csv"), &file, "C5V", &downloader()).unwrap();
        assert_eq!(
            entry.statement_kind,
            AuditStatementKind::TransmissionFileDownloaded
        );
        assert_eq!(entry.event_type, StatementEventType::USER);
        assert_eq!(entry.target.election_event_id.unwrap().to_string(), EVENT);
        assert_eq!(entry.area_id.unwrap().to_string(), AREA);
        assert_eq!(entry.username.as_deref(), Some("sbei-madrid-1"));
        assert_eq!(
            entry.description,
            "Downloaded er_0000001234.csv of the ER transmission package (SHA-256 C5V)"
        );
        assert_eq!(entry.details["file"], "ResultsCsv");
        assert_eq!(entry.details["sha256"], "C5V");
        assert_eq!(entry.details["transaction_id"], "0000001234");
        assert_eq!(entry.details["downloaded_by"], "Maria L. Santos");

        let mut bad = download("csv");
        bad.area_id = "not-a-uuid".to_string();
        assert!(download_entry(&bad, &file, "C5V", &downloader()).is_err());
    }

    #[test]
    fn refusals_read_as_sentences() {
        assert_eq!(
            DownloadRefusal::NoPackage.to_string(),
            "There is no transmission package for this Post and country."
        );
        assert_eq!(
            DownloadRefusal::NotAPackageFile.to_string(),
            "The document is not a file of this transmission package."
        );
    }
}
