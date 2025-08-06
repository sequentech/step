// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::{Display, EnumString};

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

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct AreaContestResults {
    pub area_id: String,
    pub contest_id: String,
    pub total_votes: Option<u64>,
    pub total_valid_votes: Option<u64>,
    pub invalid_votes: Option<InvalidVotes>,
    pub total_blank_votes: Option<u64>,
    pub census: Option<u64>,
    pub candidate_results: HashMap<String, CandidateResults>,
}
