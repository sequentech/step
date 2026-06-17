// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Decoded vote counts stored on published tally sheets.

#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::{Display, EnumString};

/// Delivery channel for paper-based tally sheet results.
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Hash,
)]
pub enum VotingChannel {
    /// Paper ballots counted manually.
    PAPER,
    /// Postal ballots.
    POSTAL,
    /// In-person paper voting at a polling station.
    IN_PERSON,
}

impl Default for VotingChannel {
    fn default() -> Self {
        VotingChannel::PAPER
    }
}

impl From<Option<String>> for VotingChannel {
    fn from(opt: Option<String>) -> Self {
        opt.and_then(|s| VotingChannel::from_str(&s).ok())
            .unwrap_or_else(|| VotingChannel::default())
    }
}

/// Breakdown of invalid ballots on a tally sheet.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Default)]
pub struct InvalidVotes {
    /// Total invalid ballots.
    pub total_invalid: Option<u64>,
    /// Ballots invalidated by counting rules.
    pub implicit_invalid: Option<u64>,
    /// Ballots explicitly marked invalid by the voter.
    pub explicit_invalid: Option<u64>,
}

/// Vote count for one candidate on a tally sheet.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct CandidateResults {
    /// Candidate identifier.
    pub candidate_id: String,
    /// Votes cast for this candidate.
    pub total_votes: Option<u64>,
}

/// Decoded results for one contest in one geographic area.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct AreaContestResults {
    /// Geographic area identifier.
    pub area_id: String,
    /// Contest identifier.
    pub contest_id: String,
    /// Total ballots cast.
    pub total_votes: Option<u64>,
    /// Ballots counted as valid votes.
    pub total_valid_votes: Option<u64>,
    /// Invalid ballot breakdown.
    pub invalid_votes: Option<InvalidVotes>,
    /// Blank ballots (no selection made).
    pub total_blank_votes: Option<u64>,
    /// Eligible voter count for this area and contest.
    pub census: Option<u64>,
    /// Per-candidate vote counts keyed by candidate identifier.
    pub candidate_results: HashMap<String, CandidateResults>,
}
