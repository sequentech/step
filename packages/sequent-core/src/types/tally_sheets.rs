// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::{AsRefStr, Display, EnumString};

#[derive(
    AsRefStr,
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
    PAPER,
    POSTAL,
    IN_PERSON,
}

#[derive(
    Debug,
    Default,
    Serialize,
    Deserialize,
    Clone,
    Eq,
    PartialEq,
    Display,
    EnumString,
)]
pub enum TallySheetStatus {
    #[default]
    PENDING,
    APPROVED,
    DISAPPROVED,
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

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Default)]
pub struct InvalidVotes {
    pub total_invalid: Option<u64>,
    pub implicit_invalid: Option<u64>,
    pub explicit_invalid: Option<u64>,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct CandidateResults {
    pub candidate_id: String,
    pub total_votes: Option<u64>,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Default)]
pub struct AreaContestResults {
    pub area_id: String,
    pub contest_id: String,
    pub total_votes: Option<u64>,
    pub total_valid_votes: Option<u64>,
    pub invalid_votes: Option<InvalidVotes>,
    pub total_blank_votes: Option<u64>,
    /// Ballots cast blank in every contest, in this area. A ballot-box
    /// property, not a contest property: the same value is replicated
    /// across every contest sheet of one (channel, area). `None` where a
    /// tally-sheet ballot box did not supply it.
    pub blank_ballots: Option<u64>,
    pub census: Option<u64>,
    pub candidate_results: HashMap<String, CandidateResults>,
    /// Free-form extra data from the source tally system that doesn't have
    /// a dedicated field
    #[serde(default)]
    pub annotations: Option<Value>,
}
