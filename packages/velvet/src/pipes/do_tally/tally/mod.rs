mod error;
pub mod plurality_at_large;

use std::collections::HashMap;

use super::{invalid_vote::InvalidVote, voting_system::VotingSystem};
use serde::Serialize;

pub use error::{Error, Result};

pub trait Tally {
    fn please_do(&self) -> Result<ContestResult>;
}

#[derive(Debug, Clone, Serialize)]
pub struct ContestResult {
    pub contest_id: String,
    pub total_valid_votes: u64,
    pub total_invalid_votes: HashMap<InvalidVote, u64>,
    pub choice_result: Vec<ContestChoiceResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContestChoiceResult {
    pub choice_id: String,
    pub total_count: u64,
}
