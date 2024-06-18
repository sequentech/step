// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    postgres::{
        results_area_contest::update_results_area_contest_documents,
        results_contest::update_results_contest_documents,
        results_election::update_results_election_documents,
        results_event::update_results_event_documents,
    },
    services::{
        compress::compress_folder, documents::upload_and_return_document, temp_path::get_file_size,
    },
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak;
use sequent_core::{services::connection::AuthHeaders, types::results::ResultDocuments};
use std::path::{Path, PathBuf};
use tokio::task;
use tracing::instrument;
use velvet::pipes::generate_reports::{
    ElectionReportDataComputed, ReportDataComputed, OUTPUT_HTML, OUTPUT_JSON, OUTPUT_PDF,
};
use velvet::pipes::vote_receipts::OUTPUT_FILE_PDF as OUTPUT_RECEIPT_PDF;

pub const MIME_PDF: &str = "application/pdf";
pub const MIME_JSON: &str = "application/json";
pub const MIME_HTML: &str = "text/html";

pub type ResultDocumentPaths = ResultDocuments;

#[instrument(err, skip(auth_headers))]
async fn generic_save_documents(
    auth_headers: &AuthHeaders,
    document_paths: &ResultDocumentPaths,
    tenant_id: &str,
    election_event_id: &str,
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
            MIME_PDF.to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            OUTPUT_PDF.to_string(),
            None,
            false,
        )
        .await?;
        documents.vote_receipts_pdf = Some(document.id);
    }

    // json
    if let Some(json_path) = document_paths.json.clone() {
        let json_size = get_file_size(json_path.as_str())?;

        // upload binary data into a document (s3 and hasura)
        let document = upload_and_return_document(
            json_path,
            json_size,
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
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
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
            vote_receipts_pdf: None,
        }
    }

    #[instrument(skip_all, err)]
    async fn save_documents(
        &self,
        auth_headers: &AuthHeaders,
        hasura_transaction: &Transaction<'_>,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
    ) -> Result<ResultDocuments> {
        if let Some(tar_gz_path) = document_paths.clone().tar_gz {
            // compressed file with the tally

            // Spawn the task
            let handle = tokio::task::spawn_blocking(move || {
                let path = Path::new(&tar_gz_path);
                compress_folder(path)
            });

            // Await the result
            let result = handle.await??;

            let (_tarfile_temp_path, tarfile_path, tarfile_size) = result;

            let contest = &self[0].reports[0].contest;

            // upload binary data into a document (s3 and hasura)
            let document = upload_and_return_document(
                tarfile_path.clone(),
                tarfile_size,
                "application/gzip".to_string(),
                auth_headers.clone(),
                contest.tenant_id.clone(),
                contest.election_event_id.clone(),
                "tally.tar.gz".into(),
                None,
                false,
            )
            .await?;

            let documents = ResultDocuments {
                json: None,
                pdf: None,
                html: None,
                tar_gz: Some(document.id),
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
            vote_receipts_pdf: None,
        }
    }

    #[instrument(err, skip(self, auth_headers, hasura_transaction))]
    async fn save_documents(
        &self,
        auth_headers: &AuthHeaders,
        hasura_transaction: &Transaction<'_>,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
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
            vote_receipts_pdf: vote_receipts_pdf,
        }
    }

    #[instrument(err, skip(self, auth_headers))]
    async fn save_documents(
        &self,
        auth_headers: &AuthHeaders,
        hasura_transaction: &Transaction<'_>,
        document_paths: &ResultDocumentPaths,
        results_event_id: &str,
    ) -> Result<ResultDocuments> {
        let documents = generic_save_documents(
            auth_headers,
            document_paths,
            &self.contest.tenant_id.to_string(),
            &self.contest.election_event_id.to_string(),
        )
        .await?;

        if let Some(area_id) = self.area_id.clone() {
            update_results_area_contest_documents(
                hasura_transaction,
                &self.contest.tenant_id,
                results_event_id,
                &self.contest.election_event_id,
                &self.contest.election_id,
                &self.contest.id,
                &area_id,
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

#[instrument(skip(hasura_transaction, results), err)]
pub async fn save_result_documents(
    hasura_transaction: &Transaction<'_>,
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
    base_tally_path: &PathBuf,
) -> Result<()> {
    let mut auth_headers = keycloak::get_client_credentials().await?;
    let mut idx: usize = 0;
    let event_document_paths = results.get_document_paths(None, base_tally_path);
    results
        .save_documents(
            &auth_headers,
            hasura_transaction,
            &event_document_paths,
            results_event_id,
        )
        .await?;

    for election_report in results {
        let document_paths =
            election_report.get_document_paths(election_report.area_id.clone(), base_tally_path);
        idx += 1;
        if idx % 200 == 0 {
            auth_headers = keycloak::get_client_credentials().await?;
        }
        election_report
            .save_documents(
                &auth_headers,
                hasura_transaction,
                &document_paths,
                results_event_id,
            )
            .await?;
        for contest_report in election_report.reports {
            let contest_document_paths =
                contest_report.get_document_paths(contest_report.area_id.clone(), base_tally_path);
            idx += 1;
            if idx % 200 == 0 {
                auth_headers = keycloak::get_client_credentials().await?;
            }
            contest_report
                .save_documents(
                    &auth_headers,
                    hasura_transaction,
                    &contest_document_paths,
                    results_event_id,
                )
                .await?;
        }
    }
    Ok(())
}
