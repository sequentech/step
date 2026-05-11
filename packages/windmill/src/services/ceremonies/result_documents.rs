// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::encrypter::{
    encrypt_directory_contents, encrypt_directory_contents_sql, get_file_report_type,
    traversal_encrypt_files, traversal_find_secrets_for_files,
};
use super::renamer::{rename_folders, take_first_n_chars, FOLDER_MAX_CHARS};
use crate::postgres::document::get_document;
use crate::postgres::reports::Report;
use crate::postgres::reports::{get_reports_by_election_event_id, ReportType};
use crate::postgres::results_election_area::insert_results_election_area_documents;
use crate::{
    postgres::{
        results_area_contest::update_results_area_contest_documents,
        results_contest::update_results_contest_documents,
        results_election::update_results_election_documents,
        results_event::update_results_event_documents,
    },
    services::{
        compress::create_archive_from_folder, documents::upload_and_return_document,
        folders::copy_to_temp_dir,
    },
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use rusqlite::Transaction as SqliteTransaction;
use sequent_core::services::translations::Name;
use sequent_core::sqlite::results_area_contest::update_results_area_contest_documents_sqlite;
use sequent_core::sqlite::results_contest::update_results_contest_documents_sqlite;
use sequent_core::sqlite::results_election::update_results_election_documents_sqlite;
use sequent_core::sqlite::results_election_area::create_results_election_area_sqlite;
use sequent_core::sqlite::results_event::update_results_event_documents_sqlite;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::Area;
use sequent_core::types::results::ResultDocuments;
use sequent_core::util::temp_path::get_file_size;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use strand::hash::hash_b64;
use tokio::task;
use tracing::instrument;
use velvet::pipes::generate_reports::{
    BasicArea, ElectionReportDataComputed, ReportDataComputed, OUTPUT_ALL_AREAS_HTML,
    OUTPUT_ALL_AREAS_JSON, OUTPUT_HTML, OUTPUT_JSON, OUTPUT_PDF,
};
use velvet::pipes::pipe_inputs::PREFIX_ALL_AREAS;

/// MIME type recorded when persisting tally PDFs.
pub const MIME_PDF: &str = "application/pdf";
/// MIME type recorded when persisting tally JSON exports.
pub const MIME_JSON: &str = "application/json";
/// MIME type recorded when persisting tally HTML exports.
pub const MIME_HTML: &str = "text/html";

/// Re-export of [`ResultDocuments`] describing optional filesystem paths for each rendered artifact.
pub type ResultDocumentPaths = ResultDocuments;

/// Uploads every populated path on `document_paths`, applying encryption when the active tally type
/// maps to a password-protected report definition.
///
/// # Errors
///
/// Report listing failures, encryption errors, file size lookups, or document upload failures from
/// [`process_and_upload_document`].
#[instrument(err, skip_all)]
async fn generic_save_documents(
    document_paths: &ResultDocumentPaths,
    tenant_id: &str,
    election_event_id: &str,
    hasura_transaction: &Transaction<'_>,
    tally_type_enum: TallyType,
) -> Result<ResultDocuments> {
    let mut documents: ResultDocuments = ResultDocuments::default();

    // Retrieve reports
    let all_reports =
        get_reports_by_election_event_id(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|err| anyhow!("Error getting reports: {err:?}"))?;

    let report_type = get_file_report_type(&tally_type_enum.to_string())
        .context("Error getting file report type")?;

    documents.pdf = process_and_upload_document(
        hasura_transaction,
        document_paths.pdf.clone(),
        MIME_PDF,
        OUTPUT_PDF,
        &all_reports,
        report_type.clone(),
        tenant_id,
        election_event_id,
    )
    .await?;

    documents.json = process_and_upload_document(
        hasura_transaction,
        document_paths.json.clone(),
        MIME_JSON,
        OUTPUT_JSON,
        &all_reports,
        report_type.clone(),
        tenant_id,
        election_event_id,
    )
    .await?;

    documents.html = process_and_upload_document(
        hasura_transaction,
        document_paths.html.clone(),
        MIME_HTML,
        OUTPUT_HTML,
        &all_reports,
        report_type.clone(),
        tenant_id,
        election_event_id,
    )
    .await?;

    if (document_paths.all_areas_html.is_some()) {
        documents.all_areas_html = process_and_upload_document(
            hasura_transaction,
            document_paths.all_areas_html.clone(),
            MIME_HTML,
            OUTPUT_ALL_AREAS_HTML,
            &all_reports,
            report_type.clone(),
            tenant_id,
            election_event_id,
        )
        .await?;
    }
    if (document_paths.all_areas_json.is_some()) {
        documents.all_areas_json = process_and_upload_document(
            hasura_transaction,
            document_paths.all_areas_json.clone(),
            MIME_HTML,
            OUTPUT_ALL_AREAS_JSON,
            &all_reports,
            report_type.clone(),
            tenant_id,
            election_event_id,
        )
        .await?;
    }

    Ok(documents)
}

/// Encrypts `path_option` when `report_type` matches a configured-password report, uploads the bytes
/// to object storage, and returns the new document id string.
///
/// # Errors
///
/// Encryption helper failures, missing files, size detection errors, or Hasura document insert errors.
#[instrument(err, skip(hasura_transaction, all_reports))]
async fn process_and_upload_document(
    hasura_transaction: &Transaction<'_>,
    path_option: Option<String>,
    mime_type: &str,
    output_type: &str,
    all_reports: &[Report],
    report_type: Option<ReportType>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<String>> {
    if let Some(mut path) = path_option {
        // Encrypt the file if necessary before uploading
        if let Some(report_type) = report_type {
            path = encrypt_directory_contents_sql(
                hasura_transaction,
                tenant_id,
                election_event_id,
                None,
                report_type,
                &path,
                all_reports,
            )
            .await
            .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;
        }

        let file_size = get_file_size(&path)?;

        let document = upload_and_return_document(
            hasura_transaction,
            &path,
            file_size,
            mime_type,
            tenant_id,
            Some(election_event_id.to_string()),
            output_type,
            None,
            false,
        )
        .await?;

        return Ok(Some(document.id));
    }
    Ok(None)
}

/// Implemented by Velvet result aggregates to expose filesystem layout and persist uploaded document ids.
pub trait GenerateResultDocuments {
    /// Resolves filesystem paths for each artifact type, optionally scoped to `area_id`.
    fn get_document_paths(&self, area_id: Option<String>, base_path: &Path) -> ResultDocumentPaths;
    /// Uploads artifacts described by `document_paths`, updates results tables, and optionally mirrors
    /// ids into `SQLite`.
    ///
    /// # Errors
    ///
    /// Implementation-defined failures from encryption, uploads, or database updates.
    async fn save_documents(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
        rename_map: Option<HashMap<String, String>>,
        tally_type_enum: TallyType,
        sqlite_transaction_opt: Option<&SqliteTransaction<'_>>,
    ) -> Result<ResultDocuments>;
}

/// Event-wide export stores a tarball path covering all contests in one archive.
impl GenerateResultDocuments for Vec<ElectionReportDataComputed> {
    /// Points to the Velvet output directory root (tarball export mode).
    #[instrument(skip_all, name = "Vec<ElectionReportDataComputed>::get_document_paths")]
    fn get_document_paths(&self, area_id: Option<String>, base_path: &Path) -> ResultDocumentPaths {
        ResultDocumentPaths {
            json: None,
            pdf: None,
            html: None,
            tar_gz: Some(base_path.display().to_string()),
            tar_gz_original: None,
            tar_gz_pdfs: None,
            all_areas_html: None,
            all_areas_json: None,
        }
    }

    /// Create event related documents and update the `results_event` table.
    ///
    /// # Errors
    ///
    /// Tar creation, encryption traversal, secret discovery, uploads, folder rename operations, or
    /// Postgres/SQLite updates performed inside this implementation.
    #[allow(clippy::too_many_lines, clippy::future_not_send)]
    // clippy::future_not_send: sqlite transaction is not send when passing as Option.
    #[instrument(
        skip(self, rename_map),
        err,
        name = "Vec<ElectionReportDataComputed>::save_documents"
    )]
    async fn save_documents(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
        rename_map: Option<HashMap<String, String>>,
        tally_type_enum: TallyType,
        sqlite_transaction_opt: Option<&SqliteTransaction<'_>>,
    ) -> Result<ResultDocuments> {
        let tenant_id_clone = tenant_id.to_string();
        let election_event_id_clone = election_event_id.to_string();
        let elections_ids_clone = self
            .iter()
            .map(|el| el.election_id.clone())
            .collect::<Vec<_>>();

        let dir_report_type = get_file_report_type(&tally_type_enum.to_string())?
            .context("Error getting file report type")?;

        if let Some(tar_gz_path) = document_paths.clone().tar_gz {
            // compressed file with the tally
            // PART 1: original zip
            // Spawn the task
            let tar_gz_path_clone = tar_gz_path.clone();
            let original_handle = tokio::task::spawn_blocking(move || {
                let path = Path::new(&tar_gz_path_clone);
                create_archive_from_folder(path, false)
            });

            // Await the result
            let original_result = original_handle.await??;

            let (_original_tarfile_temp_path, original_tarfile_path, original_tarfile_size) =
                original_result;

            let first_election = self.first().context("empty election report batch")?;
            let first_report = first_election
                .reports
                .first()
                .context("missing report row for tarball export")?;
            let report_tenant_id = &first_report.tenant_id;
            let report_election_event_id = &first_report.election_event_id;

            let all_reports =
                get_reports_by_election_event_id(hasura_transaction, tenant_id, election_event_id)
                    .await?;
            let all_reports_clone = all_reports.clone();

            // Encrypt the tar.gz folder if necessary before uploading
            let mut upload_path = original_tarfile_path.clone();
            upload_path = encrypt_directory_contents_sql(
                hasura_transaction,
                tenant_id,
                election_event_id,
                Some(elections_ids_clone.clone()),
                dir_report_type.clone(),
                &original_tarfile_path,
                &all_reports,
            )
            .await
            .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

            // upload binary data into a document (s3 and hasura)
            let original_document = upload_and_return_document(
                hasura_transaction,
                &upload_path,
                original_tarfile_size,
                "application/gzip",
                report_tenant_id,
                Some(report_election_event_id.clone()),
                "tally.tar.gz",
                None,
                false,
            )
            .await?;

            // PART 2: renamed folders zip
            // Spawn the task
            let tgz_path = Path::new(&tar_gz_path);
            let report_secrets_map = traversal_find_secrets_for_files(
                hasura_transaction,
                tgz_path,
                &tenant_id_clone,
                &election_event_id_clone,
                &all_reports_clone,
            )
            .await
            .map_err(|_| anyhow!("Error encrypting file"))?;

            let handle = tokio::task::spawn_blocking(move || {
                let path = Path::new(&tar_gz_path);
                let temp_dir = copy_to_temp_dir(&path.to_path_buf())?;
                let mut temp_dir_path = temp_dir.path().to_path_buf();
                let renames = rename_map.unwrap_or_default();
                let report_secrets_map = report_secrets_map.clone();
                rename_folders(&renames, &temp_dir_path)?;
                // Execute asynchronous encryption
                tokio::runtime::Handle::current().block_on(async {
                    traversal_encrypt_files(report_secrets_map, &temp_dir_path, &all_reports_clone)
                        .await
                        .map_err(|err| anyhow!("Error encrypting file"))?;

                    Ok::<_, anyhow::Error>(())
                })?;

                create_archive_from_folder(&temp_dir_path, false)
            });

            // Await the result
            let result = handle.await??;

            let (_tarfile_temp_path, tarfile_path, tarfile_size) = result;

            let mut renamed_tar_upload_path = tarfile_path.clone();

            // Encrypt the tar.gz folder if necessary before uploading
            renamed_tar_upload_path = encrypt_directory_contents_sql(
                hasura_transaction,
                tenant_id,
                election_event_id,
                Some(elections_ids_clone),
                dir_report_type,
                &tarfile_path,
                &all_reports,
            )
            .await
            .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

            // upload binary data into a document (s3 and hasura)
            let document = upload_and_return_document(
                hasura_transaction,
                &renamed_tar_upload_path,
                tarfile_size,
                "application/gzip",
                report_tenant_id,
                Some(report_election_event_id.clone()),
                "tally.tar.gz",
                None,
                false,
            )
            .await?;

            let documents = ResultDocuments {
                json: None,
                pdf: None,
                html: None,
                tar_gz: Some(document.id),
                tar_gz_original: Some(original_document.id),
                tar_gz_pdfs: None,
                all_areas_html: None,
                all_areas_json: None,
            };

            update_results_event_documents(
                hasura_transaction,
                report_tenant_id,
                results_event_id,
                report_election_event_id,
                &documents,
            )
            .await?;

            if let Some(sqlite_transaction) = sqlite_transaction_opt {
                update_results_event_documents_sqlite(
                    sqlite_transaction,
                    report_tenant_id,
                    results_event_id,
                    report_election_event_id,
                    &documents,
                )?;
            }

            Ok(documents)
        } else {
            Ok(ResultDocuments {
                json: None,
                pdf: None,
                html: None,
                tar_gz: None,
                tar_gz_original: None,
                tar_gz_pdfs: None,
                all_areas_html: None,
                all_areas_json: None,
            })
        }
    }
}

/// Per-election aggregate export (JSON/PDF/HTML under the election’s Velvet folder).
impl GenerateResultDocuments for ElectionReportDataComputed {
    /// Locates standard Velvet report filenames for this election’s output directory.
    fn get_document_paths(
        &self,
        _area_id: Option<String>,
        base_path: &Path,
    ) -> ResultDocumentPaths {
        let folder_path = base_path.join(format!(
            "output/velvet-generate-reports/election__{}",
            self.election_id
        ));
        let json_path = folder_path.join(OUTPUT_JSON);
        let pdf_path = folder_path.join(OUTPUT_PDF);
        let html_path = folder_path.join(OUTPUT_HTML);

        let all_areas_folder_path = folder_path.join(PREFIX_ALL_AREAS);
        let all_areas_html_path = all_areas_folder_path.join(OUTPUT_ALL_AREAS_HTML);
        let all_areas_json_path = all_areas_folder_path.join(OUTPUT_ALL_AREAS_JSON);

        ResultDocumentPaths {
            json: if json_path.is_file() {
                Some(json_path.display().to_string())
            } else {
                None
            },
            pdf: if pdf_path.is_file() {
                Some(pdf_path.display().to_string())
            } else {
                None
            },
            html: if html_path.is_file() {
                Some(html_path.display().to_string())
            } else {
                None
            },
            tar_gz: None,
            tar_gz_original: None,
            tar_gz_pdfs: None,
            all_areas_html: if (all_areas_html_path.is_file()) {
                Some(all_areas_html_path.display().to_string())
            } else {
                None
            },
            all_areas_json: if (all_areas_json_path.is_file()) {
                Some(all_areas_json_path.display().to_string())
            } else {
                None
            },
        }
    }

    /// Uploads election-level artifacts, records JSON content hash, and updates `results_election`.
    ///
    /// # Errors
    ///
    /// Missing report metadata, filesystem/hash errors, [`generic_save_documents`] failures, or
    /// Postgres/SQLite update errors.
    #[allow(clippy::future_not_send)]
    #[instrument(
        err,
        skip(self, hasura_transaction),
        name = "ElectionReportDataComputed::save_documents"
    )]
    async fn save_documents(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
        rename_map: Option<HashMap<String, String>>,
        tally_type_enum: TallyType,
        sqlite_transaction_opt: Option<&SqliteTransaction<'_>>,
    ) -> Result<ResultDocuments> {
        let doc_tenant_id = self
            .reports
            .first()
            .context("Missing reports")?
            .tenant_id
            .clone();

        let doc_election_event_id = self
            .reports
            .first()
            .context("Missing reports")?
            .election_event_id
            .clone();
        let election_id = self
            .reports
            .first()
            .context("Missing reports")?
            .election_id
            .clone();

        // Read the json file and hash it
        let file_path = document_paths
            .json
            .clone()
            .context("Missing json file path")?;
        let content = fs::read(file_path.clone())
            .with_context(|| format!("Failed to read the file at {file_path}"))?;
        let json_hash = hash_b64(&content).map_err(|err| anyhow!("Error hashing json: {err:?}"))?;

        // Save election results documents to S3 and Hasura
        let documents = generic_save_documents(
            document_paths,
            &doc_tenant_id,
            &doc_election_event_id,
            hasura_transaction,
            tally_type_enum,
        )
        .await?;

        update_results_election_documents(
            hasura_transaction,
            &doc_tenant_id,
            results_event_id,
            &doc_election_event_id,
            &election_id,
            &documents,
            &json_hash,
        )
        .await?;

        if let Some(sqlite_transaction) = sqlite_transaction_opt {
            update_results_election_documents_sqlite(
                sqlite_transaction,
                &doc_tenant_id,
                results_event_id,
                &doc_election_event_id,
                &election_id,
                &documents,
                &json_hash,
            )
            .await?;
        }

        Ok(documents)
    }
}

/// Per-contest (optionally per-area) export paths under Velvet’s hierarchical output layout.
impl GenerateResultDocuments for ReportDataComputed {
    /// Resolves JSON/PDF/HTML paths under `output/velvet-generate-reports/...`.
    ///
    /// # Panics
    ///
    /// Panics when `self.contest` is missing (`expect("report is missing contest")`).
    fn get_document_paths(&self, area_id: Option<String>, base_path: &Path) -> ResultDocumentPaths {
        let contest = self.contest.as_ref().expect("report is missing contest");

        let folder_path = match area_id.clone() {
            Some(area_id_str) => base_path.join(format!(
                "output/velvet-generate-reports/election__{}/contest__{}/area__{}",
                self.election_id, contest.id, area_id_str
            )),
            None => base_path.join(format!(
                "output/velvet-generate-reports/election__{}/contest__{}",
                self.election_id, contest.id
            )),
        };

        let json_path = folder_path.join(OUTPUT_JSON);
        let pdf_path = folder_path.join(OUTPUT_PDF);
        let html_path = folder_path.join(OUTPUT_HTML);

        ResultDocumentPaths {
            json: if json_path.is_file() {
                Some(json_path.display().to_string())
            } else {
                None
            },
            pdf: if pdf_path.is_file() {
                Some(pdf_path.display().to_string())
            } else {
                None
            },
            html: if html_path.is_file() {
                Some(html_path.display().to_string())
            } else {
                None
            },
            tar_gz: None,
            tar_gz_original: None,
            tar_gz_pdfs: None,
            all_areas_html: None,
            all_areas_json: None,
        }
    }

    /// Uploads contest (or area-contest) documents and updates the corresponding results tables.
    ///
    /// # Errors
    ///
    /// Hashing or IO errors when reading JSON proofs, [`generic_save_documents`] failures, or database
    /// updates for contest/area-contest rows (Hasura and optional `SQLite`).
    #[allow(clippy::future_not_send)]
    #[instrument(err, skip(self), name = "ReportDataComputed::save_documents")]
    async fn save_documents(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
        rename_map: Option<HashMap<String, String>>,
        tally_type_enum: TallyType,
        sqlite_transaction_opt: Option<&SqliteTransaction<'_>>,
    ) -> Result<ResultDocuments> {
        let documents = generic_save_documents(
            document_paths,
            &self.tenant_id.clone(),
            &self.election_event_id.clone(),
            hasura_transaction,
            tally_type_enum,
        )
        .await?;

        if let Some(contest) = self.contest.clone() {
            if let Some(area) = self.area.clone() {
                update_results_area_contest_documents(
                    hasura_transaction,
                    &self.tenant_id,
                    results_event_id,
                    &self.election_event_id,
                    &self.election_id,
                    &contest.id,
                    &area.id,
                    &documents,
                )
                .await?;

                if let Some(sqlite_transaction) = sqlite_transaction_opt {
                    update_results_area_contest_documents_sqlite(
                        sqlite_transaction,
                        &self.tenant_id,
                        results_event_id,
                        &self.election_event_id,
                        &self.election_id,
                        &contest.id,
                        &area.id,
                        &documents,
                    )
                    .await?;
                }
            } else {
                update_results_contest_documents(
                    hasura_transaction,
                    &self.tenant_id,
                    results_event_id,
                    &self.election_event_id,
                    &self.election_id,
                    &contest.id,
                    &documents,
                )
                .await?;

                if let Some(sqlite_transaction) = sqlite_transaction_opt {
                    update_results_contest_documents_sqlite(
                        sqlite_transaction,
                        &self.tenant_id,
                        results_event_id,
                        &self.election_event_id,
                        &self.election_id,
                        &contest.id,
                        &documents,
                    )
                    .await?;
                }
            }
        }

        Ok(documents)
    }
}

/// Builds a map from raw UUID ids to shortened `"{name}__{uuid}"` folder tokens used when renaming
/// Velvet export directories.
///
/// # Errors
///
/// Reserved for future validation failures; currently always returns `Ok`.
#[instrument(skip(results, areas), err)]
pub fn generate_ids_map(
    results: &[ElectionReportDataComputed],
    areas: &Vec<Area>,
    default_language: &str,
) -> Result<HashMap<String, String>> {
    let mut rename_map: HashMap<String, String> = HashMap::new();
    let election_reports = results
        .iter()
        .flat_map(|result| result.reports.clone())
        .collect::<Vec<ReportDataComputed>>();

    const UUID_LEN: usize = 36;
    const MAX_LEN: usize = FOLDER_MAX_CHARS - UUID_LEN - 2 /* 2: (include the __ characters) */;

    for election_report in election_reports {
        let election_name = election_report.election_name;
        rename_map.insert(
            election_report.election_id.clone(),
            format!(
                "{}__{}",
                take_first_n_chars(&election_name, MAX_LEN),
                election_report.election_id
            ),
        );

        if let Some(contest) = election_report.contest.clone() {
            let contest_name = contest.get_name(default_language);
            rename_map.insert(
                contest.id.clone(),
                format!(
                    "{}__{}",
                    take_first_n_chars(&contest_name, MAX_LEN),
                    contest.id
                ),
            );
        }
    }

    for area in areas {
        let Some(name) = area.name.clone() else {
            continue;
        };
        rename_map.insert(area.id.clone(), format!("{:.30}__{}", name, area.id));
    }

    Ok(rename_map)
}

/// Uploads top-level tally artifacts plus per-election and per-contest exports, including optional
/// per-area bundles derived from `results`.
///
/// # Errors
///
/// Failures from [`generate_ids_map`], missing report data during saves, encryption/upload errors, or
/// any database update returned by [`GenerateResultDocuments::save_documents`].
#[allow(clippy::future_not_send)]
#[instrument(skip(hasura_transaction, results, areas), err)]
pub async fn save_result_documents(
    hasura_transaction: &Transaction<'_>,
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
    base_tally_path: &PathBuf,
    areas: &Vec<Area>,
    default_language: &str,
    tally_type_enum: TallyType,
    sqlite_transaction_opt: Option<&SqliteTransaction<'_>>,
) -> Result<()> {
    let rename_map = generate_ids_map(&results, areas, default_language)?;
    let event_document_paths = results.get_document_paths(None, base_tally_path);
    results
        .save_documents(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &event_document_paths,
            results_event_id,
            Some(rename_map),
            tally_type_enum.clone(),
            sqlite_transaction_opt,
        )
        .await?;

    for election_report in results {
        let document_paths = election_report.get_document_paths(
            election_report.area.clone().map(|value| value.id),
            base_tally_path,
        );
        election_report
            .save_documents(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &document_paths,
                results_event_id,
                None,
                tally_type_enum.clone(),
                sqlite_transaction_opt,
            )
            .await?;
        let mut election_areas: HashMap<String, BasicArea> = HashMap::new();

        for contest_report in election_report.reports.clone() {
            let area = contest_report.area.clone();
            if let Some(area) = area {
                election_areas.insert(area.id.clone(), area);
            }
            let contest_document_paths = contest_report.get_document_paths(
                contest_report.area.clone().map(|value| value.id),
                base_tally_path,
            );
            contest_report
                .save_documents(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    &contest_document_paths,
                    results_event_id,
                    None,
                    tally_type_enum.clone(),
                    sqlite_transaction_opt,
                )
                .await?;
        }
        let report_areas: Vec<BasicArea> = election_areas.values().cloned().collect();

        let first_report = election_report
            .reports
            .first()
            .context("missing report in election_report")?;
        let report_election_event_id = first_report.election_event_id.clone();
        let report_tenant_id = first_report.tenant_id.clone();
        let report_election_id = first_report.election_id.as_ref();

        for area in report_areas {
            let documents = get_area_document_paths(&area.id, report_election_id, base_tally_path);

            save_area_documents(
                hasura_transaction,
                &report_tenant_id,
                &report_election_event_id,
                report_election_id,
                &documents,
                results_event_id,
                None,
                area,
                tally_type_enum.clone(),
                sqlite_transaction_opt,
            )
            .await?;
        }
    }
    Ok(())
}

/// Builds [`ResultDocumentPaths`] for a single area’s Velvet `generate-reports` subdirectory.
fn get_area_document_paths(
    area_id: &str,
    election_id: &str,
    base_path: &Path,
) -> ResultDocumentPaths {
    let folder_path = base_path.join(format!(
        "output/velvet-generate-reports/election__{election_id}/area__{area_id}",
    ));

    let json_path = folder_path.join(OUTPUT_JSON);
    let pdf_path = folder_path.join(OUTPUT_PDF);
    let html_path = folder_path.join(OUTPUT_HTML);

    ResultDocumentPaths {
        json: if json_path.is_file() {
            Some(json_path.display().to_string())
        } else {
            None
        },
        pdf: if pdf_path.is_file() {
            Some(pdf_path.display().to_string())
        } else {
            None
        },
        html: if html_path.is_file() {
            Some(html_path.display().to_string())
        } else {
            None
        },
        tar_gz: None,
        tar_gz_original: None,
        tar_gz_pdfs: None,
        all_areas_html: None,
        all_areas_json: None,
    }
}

/// Persists per-area report uploads and inserts `results_election_area` document references.
///
/// # Errors
///
/// Propagates failures from [`generic_save_documents`], Hasura inserts, or `SQLite` mirror updates.
#[allow(clippy::future_not_send)]
#[instrument(err, skip(hasura_transaction))]
async fn save_area_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    document_paths: &ResultDocumentPaths,
    results_event_id: &str,
    rename_map: Option<HashMap<String, String>>,
    area: BasicArea,
    tally_type_enum: TallyType,
    sqlite_transaction_opt: Option<&SqliteTransaction<'_>>,
) -> Result<ResultDocuments> {
    let documents = generic_save_documents(
        document_paths,
        tenant_id,
        election_event_id,
        hasura_transaction,
        tally_type_enum.clone(),
    )
    .await?;

    insert_results_election_area_documents(
        hasura_transaction,
        tenant_id,
        results_event_id,
        election_event_id,
        election_id,
        &area.id,
        &area.name,
        &documents,
    )
    .await?;

    if let Some(sqlite_transaction) = sqlite_transaction_opt {
        create_results_election_area_sqlite(
            sqlite_transaction,
            tenant_id,
            results_event_id,
            election_event_id,
            election_id,
            &area.id,
            &area.name,
            &documents,
        )
        .await?;
    }

    Ok(documents)
}
