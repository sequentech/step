// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]
use chrono::{DateTime, Local};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::default::Default;

/// Key for extended metrics in `ResultAreaContest` annotations
pub const EXTENDED_METRICS: &str = "extended_metrics";
/// Key for process results in `ResultAreaContest` annotations
pub const PROCESS_RESULTS: &str = "process_results";

/// Represents the type of result document generated in the Tally ceremony.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ResultDocumentType {
    /// JSON format result document.
    Json,
    /// PDF format result document.
    Pdf,
    /// HTML format result document.
    Html,
    /// TAR.GZ archive containing result documents.
    TarGz,
    /// TAR.GZ archive containing original result documents.
    TarGzOriginal,
}

/// Collection of result documents in various formats generated from the Tally ceremony.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultDocuments {
    /// JSON format result document.
    pub json: Option<String>,
    /// PDF format result document.
    pub pdf: Option<String>,
    /// HTML format result document.
    pub html: Option<String>,
    /// TAR.GZ archive containing result documents.
    pub tar_gz: Option<String>,
    /// TAR.GZ archive containing original result documents.
    pub tar_gz_original: Option<String>,
    /// TAR.GZ archive containing PDFs of result documents.
    pub tar_gz_pdfs: Option<String>,
    /// HTML result document for all areas.
    pub all_areas_html: Option<String>,
    /// JSON result document for all areas.
    pub all_areas_json: Option<String>,
}

impl ResultDocuments {
    /// Returns the document corresponding to the given type, if available.
    ///
    /// # Arguments
    /// * `doc_type` - The type of document to retrieve.
    ///
    /// # Returns
    /// An `Option<String>` containing the document path.
    #[must_use]
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
        }
    }
}

/// Represents a results for election event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsEvent {
    /// Unique identifier for the results event.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Optional name of the event.
    pub name: Option<String>,
    /// Timestamp when the results event was created.
    pub created_at: Option<DateTime<Local>>,
    /// Timestamp when the results event was last updated.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Optional labels for the results event.
    pub labels: Option<Value>,
    /// Optional annotations for the results event.
    pub annotations: Option<Value>,
    /// Associated result documents.
    pub documents: Option<ResultDocuments>,
}

/// Represents the results for a specific election.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsElection {
    /// Unique identifier for the election results.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Election identifier.
    pub election_id: String,
    /// Results event identifier.
    pub results_event_id: String,
    /// Optional name of the election.
    pub name: Option<String>,
    /// Optional eligible census count.
    pub elegible_census: Option<i64>,
    /// Optional total number of voters.
    pub total_voters: Option<i64>,
    /// Timestamp when the election results were created.
    pub created_at: Option<DateTime<Local>>,
    /// Timestamp when the election results were last updated.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Optional labels for the election results.
    pub labels: Option<Value>,
    /// Optional annotations for the election results.
    pub annotations: Option<Value>,
    /// Optional percentage of total voters.
    pub total_voters_percent: Option<NotNan<f64>>,
    /// Associated result documents.
    pub documents: Option<ResultDocuments>,
}

/// Represents the results for a specific area within an election.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsElectionArea {
    /// Unique identifier for the area results.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Election identifier.
    pub election_id: String,
    /// Area identifier.
    pub area_id: String,
    /// Results event identifier.
    pub results_event_id: String,
    /// Timestamp when the area results were created.
    pub created_at: Option<DateTime<Local>>,
    /// Timestamp when the area results were last updated.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Associated result documents.
    pub documents: Option<ResultDocuments>,
    /// Optional name of the area.
    pub name: Option<String>,
}

/// Represents the results for a specific contest within an election.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsContest {
    /// Unique identifier for the contest results.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Election identifier.
    pub election_id: String,
    /// Contest identifier.
    pub contest_id: String,
    /// Results event identifier.
    pub results_event_id: String,
    /// Optional eligible census count.
    pub elegible_census: Option<i64>,
    /// Optional total number of valid votes.
    pub total_valid_votes: Option<i64>,
    /// Optional explicit invalid votes count.
    pub explicit_invalid_votes: Option<i64>,
    /// Optional implicit invalid votes count.
    pub implicit_invalid_votes: Option<i64>,
    /// Optional blank votes count.
    pub blank_votes: Option<i64>,
    /// Optional voting type.
    pub voting_type: Option<String>,
    /// Optional counting algorithm used.
    pub counting_algorithm: Option<String>,
    /// Optional name of the contest.
    pub name: Option<String>,
    /// Timestamp when the contest results were created.
    pub created_at: Option<DateTime<Local>>,
    /// Timestamp when the contest results were last updated.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Optional labels for the contest results.
    pub labels: Option<Value>,
    /// Optional annotations for the contest results.
    pub annotations: Option<Value>,
    /// Optional total invalid votes count.
    pub total_invalid_votes: Option<i64>,
    /// Optional percentage of total invalid votes.
    pub total_invalid_votes_percent: Option<NotNan<f64>>,
    /// Optional percentage of total valid votes.
    pub total_valid_votes_percent: Option<NotNan<f64>>,
    /// Optional percentage of explicit invalid votes.
    pub explicit_invalid_votes_percent: Option<NotNan<f64>>,
    /// Optional percentage of implicit invalid votes.
    pub implicit_invalid_votes_percent: Option<NotNan<f64>>,
    /// Optional percentage of blank votes.
    pub blank_votes_percent: Option<NotNan<f64>>,
    /// Optional total votes count.
    pub total_votes: Option<i64>,
    /// Optional percentage of total votes.
    pub total_votes_percent: Option<NotNan<f64>>,
    /// Associated result documents.
    pub documents: Option<ResultDocuments>,
    /// Optional total auditable votes count.
    pub total_auditable_votes: Option<i64>,
    /// Optional percentage of total auditable votes.
    pub total_auditable_votes_percent: Option<NotNan<f64>>,
}

/// Represents the results for a specific candidate within a contest.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsContestCandidate {
    /// Unique identifier for the candidate results.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Election identifier.
    pub election_id: String,
    /// Contest identifier.
    pub contest_id: String,
    /// Candidate identifier.
    pub candidate_id: String,
    /// Results event identifier.
    pub results_event_id: String,
    /// Optional number of votes cast for the candidate.
    pub cast_votes: Option<i64>,
    /// Optional winning position of the candidate.
    pub winning_position: Option<i64>,
    /// Optional points awarded to the candidate.
    pub points: Option<i64>,
    /// Timestamp when the candidate results were created.
    pub created_at: Option<DateTime<Local>>,
    /// Timestamp when the candidate results were last updated.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Optional labels for the candidate results.
    pub labels: Option<Value>,
    /// Optional annotations for the candidate results.
    pub annotations: Option<Value>,
    /// Optional percentage of votes cast for the candidate.
    pub cast_votes_percent: Option<NotNan<f64>>,
    /// Associated result documents.
    pub documents: Option<ResultDocuments>,
}

/// Represents the results for a specific contest within an area.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsAreaContest {
    /// Unique identifier for the area contest results.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Election identifier.
    pub election_id: String,
    /// Contest identifier.
    pub contest_id: String,
    /// Area identifier.
    pub area_id: String,
    /// Results event identifier.
    pub results_event_id: String,
    /// Optional eligible census count.
    pub elegible_census: Option<i64>,
    /// Optional total number of valid votes.
    pub total_valid_votes: Option<i64>,
    /// Optional explicit invalid votes count.
    pub explicit_invalid_votes: Option<i64>,
    /// Optional implicit invalid votes count.
    pub implicit_invalid_votes: Option<i64>,
    /// Optional blank votes count.
    pub blank_votes: Option<i64>,
    /// Timestamp when the area contest results were created.
    pub created_at: Option<DateTime<Local>>,
    /// Timestamp when the area contest results were last updated.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Optional labels for the area contest results.
    pub labels: Option<Value>,
    /// Optional annotations for the area contest results.
    pub annotations: Option<Value>,
    /// Optional percentage of valid votes.
    pub total_valid_votes_percent: Option<NotNan<f64>>,
    /// Optional total invalid votes count.
    pub total_invalid_votes: Option<i64>,
    /// Optional percentage of total invalid votes.
    pub total_invalid_votes_percent: Option<NotNan<f64>>,
    /// Optional percentage of explicit invalid votes.
    pub explicit_invalid_votes_percent: Option<NotNan<f64>>,
    /// Optional percentage of blank votes.
    pub blank_votes_percent: Option<NotNan<f64>>,
    /// Optional percentage of implicit invalid votes.
    pub implicit_invalid_votes_percent: Option<NotNan<f64>>,
    /// Optional total votes count.
    pub total_votes: Option<i64>,
    /// Optional percentage of total votes.
    pub total_votes_percent: Option<NotNan<f64>>,
    /// Associated result documents.
    pub documents: Option<ResultDocuments>,
    /// Optional total auditable votes count.
    pub total_auditable_votes: Option<i64>,
    /// Optional percentage of total auditable votes.
    pub total_auditable_votes_percent: Option<NotNan<f64>>,
}

/// Represents the results for a specific candidate within a contest area.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsAreaContestCandidate {
    /// Unique identifier for the area contest candidate results.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Election identifier.
    pub election_id: String,
    /// Contest identifier.
    pub contest_id: String,
    /// Area identifier.
    pub area_id: String,
    /// Candidate identifier.
    pub candidate_id: String,
    /// Results event identifier.
    pub results_event_id: String,
    /// Optional number of votes cast for the candidate.
    pub cast_votes: Option<i64>,
    /// Optional winning position of the candidate.
    pub winning_position: Option<i64>,
    /// Optional points awarded to the candidate.
    pub points: Option<i64>,
    /// Timestamp when the area contest candidate results were created.
    pub created_at: Option<DateTime<Local>>,
    /// Timestamp when the area contest candidate results were last updated.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Optional labels for the area contest candidate results.
    pub labels: Option<Value>,
    /// Optional annotations for the area contest candidate results.
    pub annotations: Option<Value>,
    /// Optional percentage of votes cast for the candidate.
    pub cast_votes_percent: Option<NotNan<f64>>,
    /// Associated result documents.
    pub documents: Option<ResultDocuments>,
}
