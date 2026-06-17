// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Published tally results stored in Hasura and exported as reports.

#![allow(non_camel_case_types)]
use chrono::{DateTime, Local};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::default::Default;

/// Annotation key for extended turnout metrics on area-contest results.
pub const EXTENDED_METRICS: &str = "extended_metrics";
/// Annotation key for post-processing metadata on area-contest results.
pub const PROCESS_RESULTS: &str = "process_results";

/// Format of a generated results document.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ResultDocumentType {
    /// JSON results export.
    Json,
    /// PDF results report.
    Pdf,
    /// HTML results report.
    Html,
    /// Compressed archive of all result files.
    TarGz,
    /// Unmodified original tarball before post-processing.
    TarGzOriginal,
}

/// Storage paths or URLs for generated results documents at one hierarchy level.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultDocuments {
    /// JSON export location.
    pub json: Option<String>,
    /// PDF report location.
    pub pdf: Option<String>,
    /// HTML report location.
    pub html: Option<String>,
    /// Combined tarball location.
    pub tar_gz: Option<String>,
    /// Original tarball location before transformations.
    pub tar_gz_original: Option<String>,
    /// Tarball containing PDF reports only.
    pub tar_gz_pdfs: Option<String>,
    /// HTML report aggregating all areas.
    pub all_areas_html: Option<String>,
    /// JSON export aggregating all areas.
    pub all_areas_json: Option<String>,
}

impl ResultDocuments {
    /// Returns the document location for the given format.
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

/// A published results snapshot for an entire election event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsEvent {
    /// Unique results-event identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Display name.
    pub name: Option<String>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Generated report document locations.
    pub documents: Option<ResultDocuments>,
}

/// Election-level turnout and results summary within a results event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsElection {
    /// Unique results-election identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Parent results event identifier.
    pub results_event_id: String,
    /// Display name.
    pub name: Option<String>,
    /// Number of eligible voters in the census.
    pub elegible_census: Option<i64>,
    /// Total voters who cast a ballot.
    pub total_voters: Option<i64>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Turnout as a fraction of eligible voters.
    pub total_voters_percent: Option<NotNan<f64>>,
    /// Generated report document locations.
    pub documents: Option<ResultDocuments>,
}

/// Results summary for one geographic area within an election.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsElectionArea {
    /// Unique results-election-area identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Geographic area identifier.
    pub area_id: String,
    /// Parent results event identifier.
    pub results_event_id: String,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Generated report document locations.
    pub documents: Option<ResultDocuments>,
    /// Display name.
    pub name: Option<String>,
}

/// Contest-level vote counts and percentages within a results event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsContest {
    /// Unique results-contest identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Parent contest identifier.
    pub contest_id: String,
    /// Parent results event identifier.
    pub results_event_id: String,
    /// Number of eligible voters for this contest.
    pub elegible_census: Option<i64>,
    /// Ballots counted as valid votes.
    pub total_valid_votes: Option<i64>,
    /// Ballots explicitly marked invalid by the voter.
    pub explicit_invalid_votes: Option<i64>,
    /// Ballots invalidated by the counting rules.
    pub implicit_invalid_votes: Option<i64>,
    /// Blank ballots (no selection made).
    pub blank_votes: Option<i64>,
    /// Voting mechanism identifier (e.g. plurality).
    pub voting_type: Option<String>,
    /// Tally algorithm identifier.
    pub counting_algorithm: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Total invalid votes (explicit + implicit).
    pub total_invalid_votes: Option<i64>,
    /// Invalid votes as a percentage of all ballots.
    pub total_invalid_votes_percent: Option<NotNan<f64>>,
    /// Valid votes as a percentage of all ballots.
    pub total_valid_votes_percent: Option<NotNan<f64>>,
    /// Explicit invalid votes as a percentage.
    pub explicit_invalid_votes_percent: Option<NotNan<f64>>,
    /// Implicit invalid votes as a percentage.
    pub implicit_invalid_votes_percent: Option<NotNan<f64>>,
    /// Blank votes as a percentage.
    pub blank_votes_percent: Option<NotNan<f64>>,
    /// Total ballots cast (valid + invalid + blank).
    pub total_votes: Option<i64>,
    /// Total ballots as a percentage of eligible voters.
    pub total_votes_percent: Option<NotNan<f64>>,
    /// Generated report document locations.
    pub documents: Option<ResultDocuments>,
    /// Ballots included in the verifiable audit trail.
    pub total_auditable_votes: Option<i64>,
    /// Auditable ballots as a percentage.
    pub total_auditable_votes_percent: Option<NotNan<f64>>,
}

/// Per-candidate results for a contest within a results event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsContestCandidate {
    /// Unique results-contest-candidate identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Parent contest identifier.
    pub contest_id: String,
    /// Parent candidate identifier.
    pub candidate_id: String,
    /// Parent results event identifier.
    pub results_event_id: String,
    /// Votes cast for this candidate.
    pub cast_votes: Option<i64>,
    /// Finishing rank when the candidate won or placed.
    pub winning_position: Option<i64>,
    /// Points awarded (e.g. for Borda count).
    pub points: Option<i64>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Votes as a percentage of valid votes in the contest.
    pub cast_votes_percent: Option<NotNan<f64>>,
    /// Generated report document locations.
    pub documents: Option<ResultDocuments>,
}

/// Contest vote counts scoped to one geographic area.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsAreaContest {
    /// Unique results-area-contest identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Parent contest identifier.
    pub contest_id: String,
    /// Geographic area identifier.
    pub area_id: String,
    /// Parent results event identifier.
    pub results_event_id: String,
    /// Number of eligible voters for this contest in this area.
    pub elegible_census: Option<i64>,
    /// Ballots counted as valid votes.
    pub total_valid_votes: Option<i64>,
    /// Ballots explicitly marked invalid by the voter.
    pub explicit_invalid_votes: Option<i64>,
    /// Ballots invalidated by the counting rules.
    pub implicit_invalid_votes: Option<i64>,
    /// Blank ballots (no selection made).
    pub blank_votes: Option<i64>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata (may include [`EXTENDED_METRICS`] or [`PROCESS_RESULTS`]).
    pub annotations: Option<Value>,
    /// Valid votes as a percentage of all ballots.
    pub total_valid_votes_percent: Option<NotNan<f64>>,
    /// Total invalid votes (explicit + implicit).
    pub total_invalid_votes: Option<i64>,
    /// Invalid votes as a percentage of all ballots.
    pub total_invalid_votes_percent: Option<NotNan<f64>>,
    /// Explicit invalid votes as a percentage.
    pub explicit_invalid_votes_percent: Option<NotNan<f64>>,
    /// Blank votes as a percentage.
    pub blank_votes_percent: Option<NotNan<f64>>,
    /// Implicit invalid votes as a percentage.
    pub implicit_invalid_votes_percent: Option<NotNan<f64>>,
    /// Total ballots cast (valid + invalid + blank).
    pub total_votes: Option<i64>,
    /// Total ballots as a percentage of eligible voters.
    pub total_votes_percent: Option<NotNan<f64>>,
    /// Generated report document locations.
    pub documents: Option<ResultDocuments>,
    /// Ballots included in the verifiable audit trail.
    pub total_auditable_votes: Option<i64>,
    /// Auditable ballots as a percentage.
    pub total_auditable_votes_percent: Option<NotNan<f64>>,
}

/// Per-candidate results for a contest in one geographic area.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ResultsAreaContestCandidate {
    /// Unique results-area-contest-candidate identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Parent contest identifier.
    pub contest_id: String,
    /// Geographic area identifier.
    pub area_id: String,
    /// Parent candidate identifier.
    pub candidate_id: String,
    /// Parent results event identifier.
    pub results_event_id: String,
    /// Votes cast for this candidate in this area.
    pub cast_votes: Option<i64>,
    /// Finishing rank when the candidate won or placed.
    pub winning_position: Option<i64>,
    /// Points awarded (e.g. for Borda count).
    pub points: Option<i64>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Local>>,
    /// Last modification timestamp.
    pub last_updated_at: Option<DateTime<Local>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Votes as a percentage of valid votes in the contest and area.
    pub cast_votes_percent: Option<NotNan<f64>>,
    /// Generated report document locations.
    pub documents: Option<ResultDocuments>,
}
