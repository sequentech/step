// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use sequent_core::types::results::ResultDocuments;
use velvet::pipes::generate_reports::{
    CandidateResultForReport, ElectionReportDataComputed, ReportDataComputed,
};

pub type ResultDocumentPaths = ResultDocuments;

pub trait GenerateResultDocuments {
    fn get_document_paths(area_id: Option<String>) -> ResultDocuments;
    async fn save_documents() -> Result<()>;
}

impl GenerateResultDocuments for ElectionReportDataComputed {
    fn get_document_paths(area_id: Option<String>) -> ResultDocuments {
        ResultDocuments {
            json: None,
            pdf: None,
            html: None,
        }
    }

    async fn save_documents() -> Result<()> {
        Ok(())
    }
}

impl GenerateResultDocuments for ReportDataComputed {
    fn get_document_paths(area_id: Option<String>) -> ResultDocuments {
        ResultDocuments {
            json: None,
            pdf: None,
            html: None,
        }
    }

    async fn save_documents() -> Result<()> {
        Ok(())
    }
}

impl GenerateResultDocuments for CandidateResultForReport {
    fn get_document_paths(area_id: Option<String>) -> ResultDocuments {
        ResultDocuments {
            json: None,
            pdf: None,
            html: None,
        }
    }

    async fn save_documents() -> Result<()> {
        Ok(())
    }
}
