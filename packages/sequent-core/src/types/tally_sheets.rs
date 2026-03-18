// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::{Display, EnumString};

/// Represents the channel through which voting occurs.
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
    Default,
)]
pub enum VotingChannel {
    /// Paper ballot voting channel.
    #[default]
    PAPER,
    /// Postal voting channel.
    POSTAL,
    /// In-person voting channel.
    IN_PERSON,
}

impl From<Option<String>> for VotingChannel {
    fn from(opt: Option<String>) -> Self {
        opt.and_then(|s| VotingChannel::from_str(&s).ok())
            .unwrap_or_default()
    }
}

/// Represents invalid votes in a contest.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Default)]
pub struct InvalidVotes {
    /// Total number of invalid votes.
    pub total_invalid: Option<u64>,
    /// Number of implicit invalid votes.
    pub implicit_invalid: Option<u64>,
    /// Number of explicit invalid votes.
    pub explicit_invalid: Option<u64>,
}

/// Results for a candidate in a contest.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct CandidateResults {
    /// Unique identifier for the candidate.
    pub candidate_id: String,
    /// Total number of votes received by the candidate.
    pub total_votes: Option<u64>,
}

/// Results for a contest within a specific area.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct AreaContestResults {
    /// Unique identifier for the area.
    pub area_id: String,
    /// Unique identifier for the contest.
    pub contest_id: String,
    /// Total number of votes cast in the contest.
    pub total_votes: Option<u64>,
    /// Total number of valid votes in the contest.
    pub total_valid_votes: Option<u64>,
    /// Invalid votes breakdown.
    pub invalid_votes: Option<InvalidVotes>,
    /// Total number of blank votes.
    pub total_blank_votes: Option<u64>,
    /// Census count for the area.
    pub census: Option<u64>,
    /// Results for each candidate in the contest.
    pub candidate_results: HashMap<String, CandidateResults>,
}
