// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::{Candidate, Contest, StringifiedPeriodDates};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicArea {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ElectionReportDataComputed {
    pub election_id: String,
    pub area: Option<BasicArea>,
    pub census: u64,
    pub total_votes: u64,
    pub reports: Vec<ReportDataComputed>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportDataComputed {
    pub election_name: String,
    pub election_id: String,
    pub election_description: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_annotations: HashMap<String, String>,
    pub election_event_annotations: HashMap<String, String>,
    pub contest: Contest,
    pub area: Option<BasicArea>,
    pub area_annotations: HashMap<String, String>,
    pub is_aggregate: bool,
    pub tally_sheet_id: Option<String>,
    pub contest_result: ContestResult,
    pub candidate_result: Vec<CandidateResultForReport>,
    pub channel_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestResult {
    pub contest: Contest,
    pub census: u64,
    pub percentage_census: f64,
    pub auditable_votes: u64,
    pub percentage_auditable_votes: f64,
    pub total_votes: u64,
    pub percentage_total_votes: f64,
    pub total_valid_votes: u64,
    pub percentage_total_valid_votes: f64,
    pub total_invalid_votes: u64,
    pub percentage_total_invalid_votes: f64,
    pub total_blank_votes: u64,
    pub percentage_total_blank_votes: f64,
    pub invalid_votes: InvalidVotes,
    pub percentage_invalid_votes_explicit: f64,
    pub percentage_invalid_votes_implicit: f64,
    pub candidate_result: Vec<CandidateResult>,
    pub extended_metrics: Option<ExtendedMetricsContest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidVotes {
    pub explicit: u64,
    pub implicit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateResult {
    pub candidate: Candidate,
    pub percentage_votes: f64,
    pub total_count: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CandidateResultForReport {
    pub candidate: Candidate,
    pub total_count: u64,
    pub percentage_votes: f64,
    pub winning_position: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedMetricsContest {
    // Voted more candidates than the allowed amount per contest
    pub over_votes: u64,
    // Voted less than the number of votes allowed for each contest.
    pub under_votes: u64,
    // Total actual marks count of candidates in the contest. Only counted UV
    // and fully votes.
    pub votes_actually: u64,
    // Total expected marks for candidates if all votes were normal
    // (no under-votes, no over-votes) (valid-ballots X number of
    // votes possible in the contest)
    pub expected_votes: u64,
    //Total counted ballots
    pub total_ballots: u64,
}
