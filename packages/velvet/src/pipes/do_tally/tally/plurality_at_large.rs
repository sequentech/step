use std::{collections::HashMap, fs, path::Path};

use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};

use super::{ContestChoiceResult, ContestResult, Tally};
use crate::pipes::do_tally::voting_system::VotingSystem;

use super::{Error, Result};

pub struct PluralityAtLargeTally {
    pub vs: VotingSystem,
}

impl Tally for PluralityAtLargeTally {
    fn please_do(&self) -> Result<ContestResult> {
        let res = self.count_choices()?;

        Ok(res)
    }
}

impl PluralityAtLargeTally {
    pub fn new(vs: VotingSystem) -> Self {
        Self { vs }
    }

    fn count_choices(&self) -> Result<ContestResult> {
        let contest = &self.vs.contest;
        let votes = &self.vs.ballots;

        let mut vote_count: HashMap<String, u64> = HashMap::new();
        let mut count_valid: u64 = 0;
        let mut count_invalid: u64 = 0;

        for vote in votes {
            // TODO: how to handle invalid ballots?
            if vote.invalid_errors.len() > 0 || vote.is_explicit_invalid {
                count_invalid += 1;
            } else {
                for choice in &vote.choices {
                    if choice.selected >= 0 {
                        *vote_count.entry(choice.id.clone()).or_insert(0) += 1;
                        count_valid += 1;
                    }
                }
            }
        }

        let result: Vec<ContestChoiceResult> = vote_count
            .into_iter()
            .map(|(choice_id, total_count)| ContestChoiceResult {
                choice_id,
                total_count,
            })
            .collect();

        let result = contest
            .candidates
            .iter()
            .map(|c| {
                result
                    .iter()
                    .find(|r| r.choice_id == c.id)
                    .cloned()
                    .unwrap_or(ContestChoiceResult {
                        choice_id: c.id.clone(),
                        total_count: 0,
                    })
            })
            .collect::<Vec<ContestChoiceResult>>();

        let contest_result = ContestResult {
            contest_id: self.vs.contest.id.to_string(),
            total_valid_votes: count_valid,
            total_invalid_votes: 1,
            choice_result: result,
        };

        Ok(contest_result)
    }
}
