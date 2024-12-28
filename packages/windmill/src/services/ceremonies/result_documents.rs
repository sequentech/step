// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::encrypter::{encrypt_directory_contents, get_file_report_type, traversal_encrypt_files};
use super::renamer::rename_folders;
use crate::postgres::reports::Report;
use crate::postgres::reports::{get_reports_by_election_event_id, ReportType};
use crate::postgres::results_election_area::insert_results_election_area_documents;
use crate::services::ceremonies::renamer::*;
use crate::{
    postgres::{
        results_area_contest::update_results_area_contest_documents,
        results_contest::update_results_contest_documents,
        results_election::update_results_election_documents,
        results_event::update_results_event_documents,
    },
    services::{
        compress::compress_folder,
        documents::{upload_and_return_document, upload_and_return_document_postgres},
        folders::copy_to_temp_dir,
        temp_path::get_file_size,
    },
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::translations::Name;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::{services::connection::AuthHeaders, types::results::ResultDocuments};
use sequent_core::{services::keycloak, types::hasura::core::Area};
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};
use tokio::task;
use tracing::instrument;
use velvet::pipes::generate_reports::{
    BasicArea, ElectionReportDataComputed, ReportDataComputed, OUTPUT_HTML, OUTPUT_JSON, OUTPUT_PDF,
};
use velvet::pipes::vote_receipts::VOTE_RECEIPT_OUTPUT_FILE_PDF as OUTPUT_RECEIPT_PDF;

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
    tally_type_enum: TallyType,
) -> Result<ResultDocuments> {
    let mut documents: ResultDocuments = Default::default();

    // Retrieve reports
    let all_reports =
        get_reports_by_election_event_id(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|err| anyhow!("Error getting reports: {err:?}"))?;

    let report_type = get_file_report_type(&tally_type_enum.to_string())
        .context("Error getting file report type")?;

    documents.pdf = process_and_upload_document(
        document_paths.pdf.clone(),
        MIME_PDF,
        OUTPUT_PDF,
        &all_reports,
        report_type.clone(),
        auth_headers,
        tenant_id,
        election_event_id,
    )
    .await?;

    documents.json = process_and_upload_document(
        document_paths.vote_receipts_pdf.clone(),
        MIME_JSON,
        OUTPUT_JSON,
        &all_reports,
        report_type.clone(),
        auth_headers,
        tenant_id,
        election_event_id,
    )
    .await?;

    documents.html = process_and_upload_document(
        document_paths.html.clone(),
        MIME_HTML,
        OUTPUT_HTML,
        &all_reports,
        report_type.clone(),
        auth_headers,
        tenant_id,
        election_event_id,
    )
    .await?;

    Ok(documents)
}

// Helper function for processing and uploading a document
async fn process_and_upload_document(
    path_option: Option<String>,
    mime_type: &str,
    output_type: &str,
    all_reports: &Vec<Report>,
    report_type: Option<ReportType>,
    auth_headers: &AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<String>> {
    if let Some(mut path) = path_option {
        // Encrypt the file if necessary before uploading
        if let Some(report_type) = report_type {
            path = encrypt_directory_contents(
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
            path,
            file_size,
            mime_type.to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            output_type.to_string(),
            None,
            false,
        )
        .await?;

        return Ok(Some(document.id));
    }
    Ok(None)
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
        tally_type_enum: TallyType,
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
        tally_type_enum: TallyType,
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
                compress_folder(&path)
            });

            // Await the result
            let original_result = original_handle.await??;

            let (_original_tarfile_temp_path, original_tarfile_path, original_tarfile_size) =
                original_result;

            let contest = &self[0].reports[0].contest;

            let all_reports =
                get_reports_by_election_event_id(hasura_transaction, tenant_id, election_event_id)
                    .await?;
            let all_reports_clone = all_reports.clone();

            // Encrypt the tar.gz folder if necessary before uploading
            let mut upload_path = original_tarfile_path.clone();
            upload_path = encrypt_directory_contents(
                &tenant_id.clone(),
                &election_event_id.clone(),
                Some(elections_ids_clone.clone()),
                dir_report_type.clone(),
                &original_tarfile_path,
                &all_reports.clone(),
            )
            .await
            .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

            // upload binary data into a document (s3 and hasura)
            let original_document = upload_and_return_document_postgres(
                hasura_transaction,
                &upload_path,
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
                let mut temp_dir_path = temp_dir.path().to_path_buf();
                let renames = rename_map.unwrap_or(HashMap::new());
                rename_folders(&renames, &temp_dir_path)?;
                // Execute asynchronous encryption
                tokio::runtime::Handle::current().block_on(async {
                    traversal_encrypt_files(
                        &temp_dir_path,
                        &tenant_id_clone,
                        &election_event_id_clone,
                        &all_reports_clone,
                    )
                    .await
                    .map_err(|err| anyhow!("Error encrypting file"))?;

                    Ok::<_, anyhow::Error>(())
                })?;

                compress_folder(&temp_dir_path)
            });

            // Await the result
            let result = handle.await??;

            let (_tarfile_temp_path, tarfile_path, tarfile_size) = result;

            let mut upload_path = tarfile_path.clone();

            // Encrypt the tar.gz folder if necessary before uploading
            upload_path = encrypt_directory_contents(
                &tenant_id.clone(),
                &election_event_id.clone(),
                Some(elections_ids_clone),
                dir_report_type,
                &tarfile_path,
                &all_reports.clone(),
            )
            .await
            .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

            // upload binary data into a document (s3 and hasura)
            let document = upload_and_return_document_postgres(
                hasura_transaction,
                &upload_path,
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
        tally_type_enum: TallyType,
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
            tally_type_enum,
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
        tally_type_enum: TallyType,
    ) -> Result<ResultDocuments> {
        let documents = generic_save_documents(
            auth_headers,
            document_paths,
            &self.contest.tenant_id.to_string(),
            &self.contest.election_event_id.to_string(),
            &hasura_transaction,
            tally_type_enum,
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
    tally_type_enum: TallyType,
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
            tally_type_enum.clone(),
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
                tally_type_enum.clone(),
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
                    tally_type_enum.clone(),
                )
                .await?;
        }
        let areas: Vec<BasicArea> = election_areas.values().cloned().collect();

        let report_election_event_id = election_report.reports[0].contest.election_event_id.clone();
        let report_tenant_id = election_report.reports[0].contest.tenant_id.clone();
        let report_election_id: String = election_report.reports[0].contest.election_id.clone();

        for area in areas {
            let documents = get_area_document_paths(
                area.id.clone(),
                report_election_id.to_string(),
                base_tally_path,
            );

            save_area_documents(
                &auth_headers,
                hasura_transaction,
                &report_tenant_id,
                &report_election_event_id,
                &report_election_id,
                &documents,
                results_event_id,
                None,
                area,
                tally_type_enum.clone(),
            )
            .await?;
        }
    }
    Ok(())
}

fn get_area_document_paths(
    area_id: String,
    election_id: String,
    base_path: &PathBuf,
) -> ResultDocumentPaths {
    let folder_path = base_path.join(format!(
        "output/velvet-generate-reports/election__{}/area__{}",
        election_id, area_id
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

#[instrument(err, skip(auth_headers))]
async fn save_area_documents(
    auth_headers: &AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    document_paths: &ResultDocumentPaths,
    results_event_id: &str,
    rename_map: Option<HashMap<String, String>>,
    area: BasicArea,
    tally_type_enum: TallyType,
) -> Result<ResultDocuments> {
    let documents = generic_save_documents(
        auth_headers,
        document_paths,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        &hasura_transaction,
        tally_type_enum.clone(),
    )
    .await?;

    insert_results_election_area_documents(
        &hasura_transaction,
        &tenant_id,
        &results_event_id,
        &election_event_id,
        &election_id,
        &area.id,
        &area.name,
        &documents,
    )
    .await?;

    Ok(documents)
}
