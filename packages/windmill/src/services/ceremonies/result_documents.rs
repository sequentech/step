// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::renamer::rename_and_encrypt_folders;
use crate::postgres::reports::{get_report_by_type, ReportType};
use crate::services::ceremonies::renamer::*;
use crate::services::database::get_hasura_pool;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_secret_key;
use crate::services::temp_path::generate_temp_file;
use crate::services::vault;
use crate::{
    postgres::{
        results_area_contest::update_results_area_contest_documents,
        results_contest::update_results_contest_documents,
        results_election::update_results_election_documents,
        results_event::update_results_event_documents,
    },
    services::{
        compress::compress_folder,
        consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc,
        documents::{upload_and_return_document, upload_and_return_document_postgres},
        folders::copy_to_temp_dir,
        temp_path::get_file_size,
    },
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::translations::Name;
use sequent_core::{services::connection::AuthHeaders, types::results::ResultDocuments};
use sequent_core::{services::keycloak, types::hasura::core::Area};
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};
use tempfile::{NamedTempFile, TempPath};
use tokio::task;
use tracing::instrument;
use velvet::pipes::generate_reports::{
    ElectionReportDataComputed, ReportDataComputed, OUTPUT_HTML, OUTPUT_JSON, OUTPUT_PDF,
};
use velvet::pipes::vote_receipts::OUTPUT_FILE_PDF as OUTPUT_RECEIPT_PDF;
use deadpool_postgres::Client as DbClient;

pub const MIME_PDF: &str = "application/pdf";
pub const MIME_JSON: &str = "application/json";
pub const MIME_HTML: &str = "text/html";

pub type ResultDocumentPaths = ResultDocuments;

#[instrument(err, skip_all)]
async fn generic_save_documents(
    auth_headers: &AuthHeaders,
    document_paths: &ResultDocumentPaths,
    tenant_id: &str,
    election_event_id: &str,
    hasura_transaction: &Transaction<'_>,
) -> Result<ResultDocuments> {
    let mut documents: ResultDocuments = Default::default();

    // PDF
    if let Some(pdf_path) = document_paths.pdf.clone() {
        let pdf_size = get_file_size(pdf_path.as_str())?;

        // upload binary data into a document (s3 and hasura)
        let document = upload_and_return_document(
            pdf_path,
            pdf_size,
            MIME_PDF.to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            OUTPUT_PDF.to_string(),
            None,
            false,
        )
        .await?;
        documents.pdf = Some(document.id);
    }

    // vote_receipts_pdf PDF
    if let Some(pdf_path) = document_paths.vote_receipts_pdf.clone() {
        let pdf_size = get_file_size(pdf_path.as_str())?;

        // upload binary data into a document (s3 and hasura)
        let document = upload_and_return_document(
            pdf_path,
            pdf_size,
            MIME_JSON.to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            OUTPUT_JSON.to_string(),
            None,
            false,
        )
        .await?;
        documents.json = Some(document.id);
    }

    // HTML
    if let Some(html_path) = document_paths.html.clone() {
        let html_size = get_file_size(html_path.as_str())?;

        // upload binary data into a document (s3 and hasura)
        let document = upload_and_return_document(
            html_path,
            html_size,
            MIME_HTML.to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            OUTPUT_HTML.to_string(),
            None,
            false,
        )
        .await?;
        documents.html = Some(document.id);
    }
    Ok(documents)
}

pub trait GenerateResultDocuments {
    fn get_document_paths(
        &self,
        area_id: Option<String>,
        base_path: &PathBuf,
    ) -> ResultDocumentPaths;
    async fn save_documents(
        &self,
        auth_headers: &AuthHeaders,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
        rename_map: Option<HashMap<String, String>>,
    ) -> Result<ResultDocuments>;
}

impl GenerateResultDocuments for Vec<ElectionReportDataComputed> {
    fn get_document_paths(
        &self,
        area_id: Option<String>,
        base_path: &PathBuf,
    ) -> ResultDocumentPaths {
        ResultDocumentPaths {
            json: None,
            pdf: None,
            html: None,
            tar_gz: Some(base_path.display().to_string()),
            tar_gz_original: None,
            vote_receipts_pdf: None,
        }
    }

    #[instrument(skip_all, err)]
    async fn save_documents(
        &self,
        auth_headers: &AuthHeaders,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
        rename_map: Option<HashMap<String, String>>,
    ) -> Result<ResultDocuments> {
        let tenant_id_clone = tenant_id.to_string();
        let election_event_id_clone = election_event_id.to_string();
        
        if let Some(tar_gz_path) = document_paths.clone().tar_gz {
            // compressed file with the tally
            // PART 1: original zip
            // Spawn the task
            let tar_gz_path_clone = tar_gz_path.clone();
            let original_handle = tokio::task::spawn_blocking(move || {
                let path = Path::new(&tar_gz_path_clone);
                compress_folder(&path)
            });

            // Await the result
            let original_result = original_handle.await??;

            let (_original_tarfile_temp_path, original_tarfile_path, original_tarfile_size) =
                original_result;

            let contest = &self[0].reports[0].contest;

            // upload binary data into a document (s3 and hasura)
            let original_document = upload_and_return_document_postgres(
                hasura_transaction,
                &original_tarfile_path,
                original_tarfile_size,
                "application/gzip",
                &contest.tenant_id,
                Some(contest.election_event_id.to_string()),
                "tally.tar.gz",
                None,
                false,
            )
            .await?;

            // PART 2: renamed folders zip
            // Spawn the task
            let handle = tokio::task::spawn_blocking(move || {
                let path = Path::new(&tar_gz_path);
                let temp_dir = copy_to_temp_dir(&path.to_path_buf())?;
                let temp_dir_path = temp_dir.path().to_path_buf();
                let renames = rename_map.unwrap_or(HashMap::new());
            
                // Perform renaming and collect paths for encryption
                let to_encrypt_paths = rename_and_encrypt_folders(
                    &renames,
                    &temp_dir_path,
                )?;
            
                // Execute asynchronous encryption
                tokio::runtime::Handle::current().block_on(async {
                    for path in to_encrypt_paths {
                        encrypt_directory_contents(
                            &tenant_id_clone,
                            &election_event_id_clone,
                            ReportType::VOTE_RECEIPT,
                            &path,
                        )
                        .await
                        .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;
                    }
                    Ok::<_, anyhow::Error>(())
                })?;
            
                // Compress the folder
                compress_folder(&temp_dir_path)
            });            

            // Await the result
            let result = handle.await??;

            let (_tarfile_temp_path, tarfile_path, tarfile_size) = result;

            // upload binary data into a document (s3 and hasura)
            let document = upload_and_return_document_postgres(
                hasura_transaction,
                &tarfile_path,
                tarfile_size,
                "application/gzip",
                &contest.tenant_id,
                Some(contest.election_event_id.to_string()),
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
                vote_receipts_pdf: None,
            };

            update_results_event_documents(
                hasura_transaction,
                &contest.tenant_id,
                results_event_id,
                &contest.election_event_id,
                &documents,
            )
            .await?;

            Ok(documents)
        } else {
            Ok(ResultDocuments {
                json: None,
                pdf: None,
                html: None,
                tar_gz: None,
                tar_gz_original: None,
                vote_receipts_pdf: None,
            })
        }
    }
}

impl GenerateResultDocuments for ElectionReportDataComputed {
    fn get_document_paths(
        &self,
        _area_id: Option<String>,
        base_path: &PathBuf,
    ) -> ResultDocumentPaths {
        let folder_path = base_path.join(format!(
            "output/velvet-generate-reports/election__{}",
            self.election_id
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
            vote_receipts_pdf: None,
        }
    }

    #[instrument(err, skip(self, auth_headers, hasura_transaction))]
    async fn save_documents(
        &self,
        auth_headers: &AuthHeaders,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
        rename_map: Option<HashMap<String, String>>,
    ) -> Result<ResultDocuments> {
        let contest = self
            .reports
            .first()
            .context("Missing reports")?
            .contest
            .clone();

        let documents = generic_save_documents(
            auth_headers,
            document_paths,
            &contest.tenant_id.to_string(),
            &contest.election_event_id.to_string(),
            &hasura_transaction,
        )
        .await?;

        update_results_election_documents(
            hasura_transaction,
            &contest.tenant_id,
            results_event_id,
            &contest.election_event_id,
            &contest.election_id,
            &documents,
        )
        .await?;

        Ok(documents)
    }
}

impl GenerateResultDocuments for ReportDataComputed {
    fn get_document_paths(
        &self,
        area_id: Option<String>,
        base_path: &PathBuf,
    ) -> ResultDocumentPaths {
        let folder_path = match area_id.clone() {
            Some(area_id_str) => base_path.join(format!(
                "output/velvet-generate-reports/election__{}/contest__{}/area__{}",
                self.contest.election_id, self.contest.id, area_id_str
            )),
            None => base_path.join(format!(
                "output/velvet-generate-reports/election__{}/contest__{}",
                self.contest.election_id, self.contest.id
            )),
        };
        let vote_receipts_pdf = match area_id {
            Some(area_id_str) => {
                let path = base_path.join(format!(
                    "output/velvet-vote-receipts/election__{}/contest__{}/area__{}",
                    self.contest.election_id, self.contest.id, area_id_str
                ));

                if path.is_file() {
                    Some(path.join(OUTPUT_RECEIPT_PDF).display().to_string())
                } else {
                    None
                }
            }
            None => None,
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
            vote_receipts_pdf: vote_receipts_pdf,
        }
    }

    #[instrument(err, skip(self, auth_headers))]
    async fn save_documents(
        &self,
        auth_headers: &AuthHeaders,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
        rename_map: Option<HashMap<String, String>>,
    ) -> Result<ResultDocuments> {
        let documents = generic_save_documents(
            auth_headers,
            document_paths,
            &self.contest.tenant_id.to_string(),
            &self.contest.election_event_id.to_string(),
            &hasura_transaction,
        )
        .await?;

        if let Some(area) = self.area.clone() {
            update_results_area_contest_documents(
                hasura_transaction,
                &self.contest.tenant_id,
                results_event_id,
                &self.contest.election_event_id,
                &self.contest.election_id,
                &self.contest.id,
                &area.id,
                &documents,
            )
            .await?;
        } else {
            update_results_contest_documents(
                hasura_transaction,
                &self.contest.tenant_id,
                results_event_id,
                &self.contest.election_event_id,
                &self.contest.election_id,
                &self.contest.id,
                &documents,
            )
            .await?;
        }

        Ok(documents)
    }
}

#[instrument(skip(results), err)]
pub fn generate_ids_map(
    results: &Vec<ElectionReportDataComputed>,
    areas: &Vec<Area>,
    default_language: &str,
) -> Result<HashMap<String, String>> {
    let mut rename_map: HashMap<String, String> = HashMap::new();
    let election_reports = results
        .into_iter()
        .map(|result| result.reports.clone())
        .flat_map(|inner_vec| inner_vec)
        .collect::<Vec<ReportDataComputed>>();

    const UUID_LEN: usize = 36;
    const MAX_LEN: usize = FOLDER_MAX_CHARS - UUID_LEN - 2 /* 2: (include the __ characters) */;

    for election_report in election_reports {
        let election_name = election_report.election_name;
        rename_map.insert(
            election_report.contest.election_id.clone(),
            format!(
                "{}__{}",
                take_first_n_chars(&election_name, MAX_LEN),
                election_report.contest.election_id
            ),
        );

        let contest_name = election_report.contest.get_name(default_language);
        rename_map.insert(
            election_report.contest.id.clone(),
            format!(
                "{}__{}",
                take_first_n_chars(&contest_name, MAX_LEN),
                election_report.contest.id
            ),
        );
    }

    for area in areas {
        let Some(name) = area.name.clone() else {
            continue;
        };
        rename_map.insert(area.id.clone(), format!("{:.30}__{}", name, area.id));
    }

    Ok(rename_map)
}

#[instrument(skip(hasura_transaction, results), err)]
pub async fn save_result_documents(
    hasura_transaction: &Transaction<'_>,
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
    base_tally_path: &PathBuf,
    areas: &Vec<Area>,
    default_language: &str,
) -> Result<()> {
    let mut auth_headers = keycloak::get_client_credentials().await?;
    let mut idx: usize = 0;
    let rename_map = generate_ids_map(&results, areas, default_language)?;
    let event_document_paths = results.get_document_paths(None, base_tally_path);
    results
        .save_documents(
            &auth_headers,
            hasura_transaction,
            tenant_id,
            election_event_id,
            &event_document_paths,
            results_event_id,
            Some(rename_map),
        )
        .await?;

    for election_report in results {
        let document_paths = election_report.get_document_paths(
            election_report.area.clone().map(|value| value.id),
            base_tally_path,
        );
        idx += 1;
        if idx % 200 == 0 {
            auth_headers = keycloak::get_client_credentials().await?;
        }
        election_report
            .save_documents(
                &auth_headers,
                hasura_transaction,
                tenant_id,
                election_event_id,
                &document_paths,
                results_event_id,
                None,
            )
            .await?;
        for contest_report in election_report.reports {
            let contest_document_paths = contest_report.get_document_paths(
                contest_report.area.clone().map(|value| value.id),
                base_tally_path,
            );
            idx += 1;
            if idx % 200 == 0 {
                auth_headers = keycloak::get_client_credentials().await?;
            }
            contest_report
                .save_documents(
                    &auth_headers,
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    &contest_document_paths,
                    results_event_id,
                    None,
                )
                .await?;
        }
    }
    Ok(())
}

/// Encrypt all files in a directory
pub async fn encrypt_directory_contents(
    tenant_id: &str,
    election_event_id: &str,
    report_type: ReportType,
    pdf_path: &String,
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

    let encrypted_temp_data: Option<TempPath> = if let Some(report) = &report {
        if report.encryption_policy == EReportEncryption::ConfiguredPassword {
            let secret_key =
                get_report_secret_key(&tenant_id, &election_event_id, Some(report.id.clone()));

            let encryption_password = vault::read_secret(secret_key.clone())
                .await?
                .ok_or_else(|| anyhow!("Encryption password not found"))?;

            // Encrypt the file
            let enc_file: NamedTempFile = generate_temp_file("vote_receipts_pdf-", ".epdf")
                .with_context(|| "Error creating named temp file")?;

            let enc_temp_path = enc_file.into_temp_path();
            let encrypted_temp_path = enc_temp_path.to_string_lossy().to_string();

            encrypt_file_aes_256_cbc(&pdf_path.as_str(), &encrypted_temp_path, &encryption_password)
                .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

            Some(enc_temp_path)
        } else {
            None // No encryption needed
        }
    } else {
        None // No report, no encryption
    };

    // Use encrypted_temp_data if encryption is enabled and the file exists, otherwise use pdf_path
    let upload_path = if let Some(ref enc_temp_path) = encrypted_temp_data {
        if enc_temp_path.exists() {
            enc_temp_path.to_string_lossy().to_string()
        } else {
            pdf_path.as_str().to_string()
        }
    } else {
        pdf_path.as_str().to_string()
    };    

    Ok(upload_path)
}
