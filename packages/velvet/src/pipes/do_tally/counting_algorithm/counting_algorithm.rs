// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub use super::error::{Error, Result};
use crate::pipes::do_tally::invalid_vote::InvalidVote;
use serde::Serialize;
use std::collections::HashMap;

pub trait CountingAlgorithm {
    fn tally(&self) -> Result<ContestResult>;
}

#[derive(Debug, Clone, Serialize)]
pub struct ContestResult {
    pub contest_id: String,
    pub total_valid_votes: u64,
    pub total_invalid_votes: HashMap<InvalidVote, u64>,
    pub candidate_result: Vec<CandidateResult>,
    // TODO:
    // contest: Contest
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateResult {
    pub choice_id: String,
    pub total_count: u64,
    // TODO:
    // candidate: Candidate
}
