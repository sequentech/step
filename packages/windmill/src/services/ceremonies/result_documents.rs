use std::path::PathBuf;

// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::{compress::read_file_to_bytes, documents::upload_and_return_document};
use anyhow::{Context, Result};
use sequent_core::{services::connection::AuthHeaders, types::results::ResultDocuments};
use tracing::instrument;
use velvet::pipes::generate_reports::{
    CandidateResultForReport, ElectionReportDataComputed, ReportDataComputed, OUTPUT_HTML,
    OUTPUT_JSON, OUTPUT_PDF,
};

pub const MIME_PDF: &str = "application/pdf";
pub const MIME_JSON: &str = "application/json";
pub const MIME_HTML: &str = "text/html";

pub type ResultDocumentPaths = ResultDocuments;

async fn generic_save_documents(
    auth_headers: &AuthHeaders,
    document_paths: &ResultDocumentPaths,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ResultDocuments> {
    let mut documents: ResultDocuments = Default::default();

    // PDF
    if let Some(pdf_path) = document_paths.pdf.clone() {
        let bytes = read_file_to_bytes(&PathBuf::from(pdf_path))?;

        // upload binary data into a document (s3 and hasura)
        let document = upload_and_return_document(
            bytes,
            MIME_PDF.to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            OUTPUT_PDF.to_string(),
        )
        .await?;
        documents.pdf = Some(document.id);
    }

    // json
    if let Some(json_path) = document_paths.json.clone() {
        let bytes = read_file_to_bytes(&PathBuf::from(json_path))?;

        // upload binary data into a document (s3 and hasura)
        let document = upload_and_return_document(
            bytes,
            MIME_JSON.to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            OUTPUT_JSON.to_string(),
        )
        .await?;
        documents.json = Some(document.id);
    }

    // HTML
    if let Some(html_path) = document_paths.html.clone() {
        let bytes = read_file_to_bytes(&PathBuf::from(html_path))?;

        // upload binary data into a document (s3 and hasura)
        let document = upload_and_return_document(
            bytes,
            MIME_HTML.to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            OUTPUT_HTML.to_string(),
        )
        .await?;
        documents.html = Some(document.id);
    }
    Ok(documents)
}

pub trait GenerateResultDocuments {
    fn get_document_paths(
        self,
        area_id: Option<String>,
        base_path: &PathBuf,
    ) -> ResultDocumentPaths;
    async fn save_documents(
        self,
        auth_headers: &AuthHeaders,
        document_paths: &ResultDocumentPaths,
    ) -> Result<ResultDocuments>;
}

impl GenerateResultDocuments for ElectionReportDataComputed {
    fn get_document_paths(
        self,
        _area_id: Option<String>,
        base_path: &PathBuf,
    ) -> ResultDocumentPaths {
        let folder_path = base_path.join(format!(
            "output/velvet-generate-reports/election__{}",
            self.election_id
        ));
        ResultDocumentPaths {
            json: Some(folder_path.join(OUTPUT_JSON).display().to_string()),
            pdf: Some(folder_path.join(OUTPUT_PDF).display().to_string()),
            html: Some(folder_path.join(OUTPUT_HTML).display().to_string()),
        }
    }

    async fn save_documents(
        self,
        auth_headers: &AuthHeaders,
        document_paths: &ResultDocumentPaths,
    ) -> Result<ResultDocuments> {
        let contest = self
            .reports
            .first()
            .context("Missing reports")?
            .contest
            .clone();

        generic_save_documents(
            auth_headers,
            document_paths,
            &contest.tenant_id.to_string(),
            &contest.election_event_id.to_string(),
        )
        .await
    }
}

impl GenerateResultDocuments for ReportDataComputed {
    fn get_document_paths(
        self,
        area_id: Option<String>,
        base_path: &PathBuf,
    ) -> ResultDocumentPaths {
        let folder_path = match area_id {
            Some(area_id_str) => base_path.join(format!(
                "output/velvet-generate-reports/election__{}/contest__{}/area__{}",
                self.contest.election_id, self.contest.id, area_id_str
            )),
            None => base_path.join(format!(
                "output/velvet-generate-reports/election__{}/contest__{}",
                self.contest.election_id, self.contest.id
            )),
        };
        ResultDocumentPaths {
            json: Some(folder_path.join(OUTPUT_JSON).display().to_string()),
            pdf: Some(folder_path.join(OUTPUT_PDF).display().to_string()),
            html: Some(folder_path.join(OUTPUT_HTML).display().to_string()),
        }
    }

    async fn save_documents(
        self,
        auth_headers: &AuthHeaders,
        document_paths: &ResultDocumentPaths,
    ) -> Result<ResultDocuments> {
        generic_save_documents(
            auth_headers,
            document_paths,
            &self.contest.tenant_id.to_string(),
            &self.contest.election_event_id.to_string(),
        )
        .await
    }
}
