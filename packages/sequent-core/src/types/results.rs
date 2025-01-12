// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::default::Default;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ResultDocumentType {
    Json,
    Pdf,
    Html,
    TarGz,
    TarGzOriginal,
    VoteReceiptsPdf,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultDocuments {
    pub json: Option<String>,
    pub pdf: Option<String>,
    pub html: Option<String>,
    pub tar_gz: Option<String>,
    pub tar_gz_original: Option<String>,
    pub vote_receipts_pdf: Option<String>,
}

impl ResultDocuments {
    pub fn get_document_by_type(
        &self,
        doc_type: ResultDocumentType,
    ) -> Option<String> {
        match doc_type {
            ResultDocumentType::Json => self.json.clone(),
            ResultDocumentType::Pdf => self.pdf.clone(),
            ResultDocumentType::Html => self.html.clone(),
            ResultDocumentType::TarGz => self.tar_gz.clone(),
            ResultDocumentType::TarGzOriginal => self.tar_gz_original.clone(),
            ResultDocumentType::VoteReceiptsPdf => {
                self.vote_receipts_pdf.clone()
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsEvent {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub name: Option<String>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub documents: Option<ResultDocuments>,
}
