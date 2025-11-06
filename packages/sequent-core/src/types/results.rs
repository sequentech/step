// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]
use chrono::{DateTime, Local};
use ordered_float::NotNan;
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
    pub tar_gz_pdfs: Option<String>,
    pub vote_receipts_pdf: Option<String>,
}

impl ResultDocuments {
    pub fn get_document_by_type(
        &self,
        doc_type: &ResultDocumentType,
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

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsElection {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub results_event_id: String,
    pub name: Option<String>,
    pub elegible_census: Option<i64>,
    pub total_voters: Option<i64>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub total_voters_percent: Option<NotNan<f64>>,
    pub documents: Option<ResultDocuments>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsElectionArea {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub area_id: String,
    pub results_event_id: String,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub documents: Option<ResultDocuments>,
    pub name: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsContest {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub contest_id: String,
    pub results_event_id: String,
    pub elegible_census: Option<i64>,
    pub total_valid_votes: Option<i64>,
    pub explicit_invalid_votes: Option<i64>,
    pub implicit_invalid_votes: Option<i64>,
    pub blank_votes: Option<i64>,
    pub voting_type: Option<String>,
    pub counting_algorithm: Option<String>,
    pub name: Option<String>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub total_invalid_votes: Option<i64>,
    pub total_invalid_votes_percent: Option<NotNan<f64>>,
    pub total_valid_votes_percent: Option<NotNan<f64>>,
    pub explicit_invalid_votes_percent: Option<NotNan<f64>>,
    pub implicit_invalid_votes_percent: Option<NotNan<f64>>,
    pub blank_votes_percent: Option<NotNan<f64>>,
    pub total_votes: Option<i64>,
    pub total_votes_percent: Option<NotNan<f64>>,
    pub documents: Option<ResultDocuments>,
    pub total_auditable_votes: Option<i64>,
    pub total_auditable_votes_percent: Option<NotNan<f64>>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsContestCandidate {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub contest_id: String,
    pub candidate_id: String,
    pub results_event_id: String,
    pub cast_votes: Option<i64>,
    pub winning_position: Option<i64>,
    pub points: Option<i64>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub cast_votes_percent: Option<NotNan<f64>>,
    pub documents: Option<ResultDocuments>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsAreaContest {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub contest_id: String,
    pub area_id: String,
    pub results_event_id: String,
    pub elegible_census: Option<i64>,
    pub total_valid_votes: Option<i64>,
    pub explicit_invalid_votes: Option<i64>,
    pub implicit_invalid_votes: Option<i64>,
    pub blank_votes: Option<i64>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub total_valid_votes_percent: Option<NotNan<f64>>,
    pub total_invalid_votes: Option<i64>,
    pub total_invalid_votes_percent: Option<NotNan<f64>>,
    pub explicit_invalid_votes_percent: Option<NotNan<f64>>,
    pub blank_votes_percent: Option<NotNan<f64>>,
    pub implicit_invalid_votes_percent: Option<NotNan<f64>>,
    pub total_votes: Option<i64>,
    pub total_votes_percent: Option<NotNan<f64>>,
    pub documents: Option<ResultDocuments>,
    pub total_auditable_votes: Option<i64>,
    pub total_auditable_votes_percent: Option<NotNan<f64>>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsAreaContestCandidate {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub contest_id: String,
    pub area_id: String,
    pub candidate_id: String,
    pub results_event_id: String,
    pub cast_votes: Option<i64>,
    pub winning_position: Option<i64>,
    pub points: Option<i64>,
    pub created_at: Option<DateTime<Local>>,
    pub last_updated_at: Option<DateTime<Local>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub cast_votes_percent: Option<NotNan<f64>>,
    pub documents: Option<ResultDocuments>,
}
