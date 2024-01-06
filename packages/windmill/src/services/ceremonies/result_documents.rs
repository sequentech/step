use std::path::PathBuf;

// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use sequent_core::types::results::ResultDocuments;
use tracing::instrument;
use velvet::pipes::generate_reports::{
    CandidateResultForReport, ElectionReportDataComputed, ReportDataComputed, OUTPUT_HTML,
    OUTPUT_JSON, OUTPUT_PDF,
};

pub type ResultDocumentPaths = ResultDocuments;

pub trait GenerateResultDocuments {
    fn get_document_paths(self, area_id: Option<String>, base_path: &PathBuf) -> ResultDocuments;
    async fn save_documents(self) -> Result<()>;
}

impl GenerateResultDocuments for ElectionReportDataComputed {
    fn get_document_paths(self, area_id: Option<String>, base_path: &PathBuf) -> ResultDocuments {
        let folder_path = base_path.join(format!(
            "output/velvet-generate-reports/election__{}",
            self.election_id
        ));
        ResultDocuments {
            json: Some(folder_path.join(OUTPUT_JSON).display().to_string()),
            pdf: Some(folder_path.join(OUTPUT_PDF).display().to_string()),
            html: Some(folder_path.join(OUTPUT_HTML).display().to_string()),
        }
    }

    async fn save_documents(self) -> Result<()> {
        Ok(())
    }
}

impl GenerateResultDocuments for ReportDataComputed {
    fn get_document_paths(self, area_id: Option<String>, base_path: &PathBuf) -> ResultDocuments {
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
        ResultDocuments {
            json: Some(folder_path.join(OUTPUT_JSON).display().to_string()),
            pdf: Some(folder_path.join(OUTPUT_PDF).display().to_string()),
            html: Some(folder_path.join(OUTPUT_HTML).display().to_string()),
        }
    }

    async fn save_documents(self) -> Result<()> {
        Ok(())
    }
}
